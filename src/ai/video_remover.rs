use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;

use crate::ai::remover::remove_bg_image;

pub fn process_video<F>(input: &str, output: &str, mut progress: F) -> Result<()>
where
    F: FnMut(usize, usize),
{
    gst::init()?;

    let in_path = Path::new(input).canonicalize()?;
    let in_uri = gst::glib::filename_to_uri(&in_path, None)
        .map_err(|e| anyhow!("filename_to_uri: {}", e))?;
    let out_path: PathBuf = PathBuf::from(output);

    // ---------- Decode pipeline ----------
    let decode_desc = format!(
        "uridecodebin uri={} ! videoconvert ! video/x-raw,format=RGBA ! \
         appsink name=sink max-buffers=2 drop=false sync=false",
        in_uri.as_str()
    );

    let decode_pipeline = gst::parse::launch(&decode_desc)?
        .downcast::<gst::Pipeline>()
        .map_err(|_| anyhow!("decode pipeline cast failed"))?;

    let appsink = decode_pipeline
        .by_name("sink")
        .ok_or_else(|| anyhow!("appsink missing"))?
        .downcast::<gst_app::AppSink>()
        .map_err(|_| anyhow!("appsink cast failed"))?;

    decode_pipeline.set_state(gst::State::Playing)?;

    // Pull first frame to learn caps and build encoder pipeline.
    let first = appsink
        .try_pull_sample(gst::ClockTime::NONE)
        .ok_or_else(|| anyhow!("No frames decoded"))?;

    let caps = first.caps().ok_or_else(|| anyhow!("no caps"))?;
    let info = gst_video::VideoInfo::from_caps(caps)?;
    let width = info.width();
    let height = info.height();
    let fps = info.fps();
    let fps_n = fps.numer().max(1);
    let fps_d = fps.denom().max(1);
    let frame_duration_ns: u64 = (1_000_000_000u64 * fps_d as u64) / fps_n as u64;
    let frame_duration = gst::ClockTime::from_nseconds(frame_duration_ns);

    // ---------- Encode pipeline ----------
    let encode_desc = format!(
        "appsrc name=src is-live=false format=time block=true ! \
         video/x-raw,format=RGBA,width={},height={},framerate={}/{} ! \
         videoconvert ! vp8enc deadline=1 ! webmmux ! filesink location={}",
        width,
        height,
        fps_n,
        fps_d,
        out_path.display()
    );

    let encode_pipeline = gst::parse::launch(&encode_desc)?
        .downcast::<gst::Pipeline>()
        .map_err(|_| anyhow!("encode pipeline cast failed"))?;

    let appsrc = encode_pipeline
        .by_name("src")
        .ok_or_else(|| anyhow!("appsrc missing"))?
        .downcast::<gst_app::AppSrc>()
        .map_err(|_| anyhow!("appsrc cast failed"))?;

    appsrc.set_caps(Some(
        &gst::Caps::builder("video/x-raw")
            .field("format", "RGBA")
            .field("width", width as i32)
            .field("height", height as i32)
            .field("framerate", gst::Fraction::new(fps_n, fps_d))
            .build(),
    ));

    encode_pipeline.set_state(gst::State::Playing)?;

    // ---------- Main loop: decode -> ONNX -> encode ----------
    let mut frame_idx: u64 = 0;
    let mut pending: Option<gst::Sample> = Some(first);

    loop {
        let sample = match pending.take() {
            Some(s) => s,
            None => match appsink.try_pull_sample(gst::ClockTime::NONE) {
                Some(s) => s,
                None => break, // EOS
            },
        };

        let buffer = sample.buffer().ok_or_else(|| anyhow!("no buffer"))?;
        let pts = buffer.pts().unwrap_or_else(|| {
            gst::ClockTime::from_nseconds(frame_idx * frame_duration_ns)
        });

        let data = {
            let map = buffer.map_readable()?;
            map.as_slice().to_vec()
        };

        let rgba = image::RgbaImage::from_raw(width, height, data)
            .ok_or_else(|| anyhow!("Bad RGBA buffer"))?;
        let out = remove_bg_image(&rgba)?;
        let out_raw = out.into_raw();

        let mut out_buf = gst::Buffer::with_size(out_raw.len())?;
        {
            let b = out_buf
                .get_mut()
                .ok_or_else(|| anyhow!("buffer get_mut failed"))?;
            b.copy_from_slice(0, &out_raw)
                .map_err(|c| anyhow!("copy_from_slice failed: {}", c))?;
            b.set_pts(pts);
            b.set_duration(frame_duration);
        }
        appsrc.push_buffer(out_buf)?;

        frame_idx += 1;
        progress(frame_idx as usize, 0);
    }

    appsrc.end_of_stream()?;

    // ---------- Wait for EOS on encoder ----------
    let bus = encode_pipeline.bus().ok_or_else(|| anyhow!("no bus"))?;
    for msg in bus.iter_timed(gst::ClockTime::from_seconds(600)) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                let _ = encode_pipeline.set_state(gst::State::Null);
                let _ = decode_pipeline.set_state(gst::State::Null);
                return Err(anyhow!("Encoding error: {}", err.error()));
            }
            _ => {}
        }
    }

    let _ = encode_pipeline.set_state(gst::State::Null);
    let _ = decode_pipeline.set_state(gst::State::Null);
    Ok(())
}