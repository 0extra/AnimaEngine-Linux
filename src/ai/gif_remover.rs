use std::fs::File;
use std::io::BufWriter;

use anyhow::{anyhow, Result};
use image::RgbaImage;

use crate::ai::remover::remove_bg_image;

pub fn process_gif<F>(input: &str, output: &str, mut progress: F) -> Result<()>
where
    F: FnMut(usize, usize),
{
    let file = File::open(input)?;
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(file)?;

    let canvas_w = decoder.width() as u32;
    let canvas_h = decoder.height() as u32;

    // Composited canvas — GIF frames can be partial and offset.
    let mut canvas = RgbaImage::new(canvas_w, canvas_h);
    let mut prev_canvas = canvas.clone();

    // (rgba bytes, delay in centiseconds)
    let mut processed: Vec<(Vec<u8>, u16)> = Vec::new();

    while let Some(frame) = decoder.read_next_frame()? {
        let dispose = frame.dispose;

        if dispose == gif::DisposalMethod::Previous {
            prev_canvas = canvas.clone();
        }

        let fw = frame.width as u32;
        let fh = frame.height as u32;
        let expected = (fw as usize) * (fh as usize) * 4;
        if frame.buffer.len() < expected {
            continue;
        }

        let frame_img = RgbaImage::from_raw(fw, fh, frame.buffer.to_vec())
            .ok_or_else(|| anyhow!("Bad frame buffer"))?;

        let ox = frame.left as u32;
        let oy = frame.top as u32;

        // Alpha-composite frame onto canvas
        for (x, y, px) in frame_img.enumerate_pixels() {
            let cx = ox + x;
            let cy = oy + y;
            if cx >= canvas_w || cy >= canvas_h {
                continue;
            }
            if px.0[3] == 0 {
                continue;
            }
            canvas.put_pixel(cx, cy, *px);
        }

        // AI on full composited canvas
        let processed_img = remove_bg_image(&canvas)?;
        processed.push((processed_img.into_raw(), frame.delay));

        // Disposal AFTER current frame's display
        match dispose {
            gif::DisposalMethod::Background => {
                let y_end = (oy + fh).min(canvas_h);
                let x_end = (ox + fw).min(canvas_w);
                for yy in oy..y_end {
                    for xx in ox..x_end {
                        canvas.put_pixel(xx, yy, image::Rgba([0, 0, 0, 0]));
                    }
                }
            }
            gif::DisposalMethod::Previous => {
                canvas = prev_canvas.clone();
            }
            _ => {} // Any / Keep
        }

        progress(processed.len(), 0);
    }

    let total = processed.len();
    if total == 0 {
        return Err(anyhow!("Empty GIF or no frames decoded"));
    }

    let out_file = File::create(output)?;
    let mut encoder = gif::Encoder::new(
        BufWriter::new(out_file),
        canvas_w as u16,
        canvas_h as u16,
        &[],
    )?;

    for (i, (raw, delay)) in processed.into_iter().enumerate() {
        let mut buffer = raw;
        let mut frame =
            gif::Frame::from_rgba_speed(canvas_w as u16, canvas_h as u16, &mut buffer, 10);
        // 0 delay is browser-interpreted as ~100ms; keep that semantic.
        frame.delay = if delay == 0 { 10 } else { delay };
        encoder.write_frame(&frame)?;
        progress(i + 1, total);
    }

    Ok(())
}