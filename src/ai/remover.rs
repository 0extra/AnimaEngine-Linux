use std::path::Path;
use std::sync::Mutex;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use image::{imageops::FilterType, DynamicImage, RgbaImage};
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;

static SESSION: OnceLock<Mutex<Session>> = OnceLock::new();

fn get_session() -> Result<&'static Mutex<Session>> {
    if let Some(s) = SESSION.get() {
        return Ok(s);
    }

    let model_path = Path::new("models/u2net.onnx");
    if !model_path.exists() {
        return Err(anyhow!(
            "Model not found at models/u2net.onnx. Download it with:\n\
             mkdir -p models && \\\n\
             wget https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx -P models/"
        ));
    }

    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(model_path)?;

    // Атомарно: если другой поток успел создать — используем его, нашу сессию дропаем.
    let _ = SESSION.set(Mutex::new(session));

    SESSION
        .get()
        .ok_or_else(|| anyhow!("SESSION init race, retry"))
}

pub fn remove_bg_image(rgba: &RgbaImage) -> Result<RgbaImage> {
    let (orig_w, orig_h) = rgba.dimensions();

    let dynamic = DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
    let resized = image::imageops::resize(&dynamic, 320, 320, FilterType::Triangle);

    let mut input = Array4::<f32>::zeros((1, 3, 320, 320));
    for (x, y, pixel) in resized.enumerate_pixels() {
        let [r, g, b] = pixel.0;
        input[[0, 0, y as usize, x as usize]] = r as f32 / 255.0;
        input[[0, 1, y as usize, x as usize]] = g as f32 / 255.0;
        input[[0, 2, y as usize, x as usize]] = b as f32 / 255.0;
    }

    let session_mutex = get_session()?;
    let mut session = session_mutex.lock().map_err(|e| anyhow!("lock: {}", e))?;

    let input_name = session.inputs[0].name.clone();
    let output_name = session.outputs[0].name.clone();

    let input_value = Value::from_array(input)?;
    let outputs = session.run(ort::inputs![input_name.as_str() => input_value])?;

    let output_array = outputs[output_name.as_str()].try_extract_array::<f32>()?;
    let shape = output_array.shape();
    if shape.len() != 4 {
        return Err(anyhow!("Unexpected output shape: {:?}", shape));
    }
    let mask_h = shape[2];
    let mask_w = shape[3];
    let data: Vec<f32> = output_array.iter().cloned().collect();

    // U²-Net / ISNet output already in [0, 1] via sigmoid. No min/max normalization.
    let mut mask_img = image::GrayImage::new(mask_w as u32, mask_h as u32);
    for y in 0..mask_h {
        for x in 0..mask_w {
            let idx = y * mask_w + x;
            let v = (data[idx].clamp(0.0, 1.0) * 255.0) as u8;
            mask_img.put_pixel(x as u32, y as u32, image::Luma([v]));
        }
    }

    let mask_resized = image::imageops::resize(&mask_img, orig_w, orig_h, FilterType::Triangle);

    let mut out = rgba.clone();
    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let alpha = mask_resized.get_pixel(x, y).0[0];
        pixel.0[3] = alpha;
    }

    Ok(out)
}

pub fn process_image(input_path: &str, output_path: &str, remove_bg: bool) -> Result<()> {
    if !remove_bg {
        std::fs::copy(input_path, output_path)?;
        return Ok(());
    }

    let img = image::open(input_path)?;
    let rgba = img.to_rgba8();
    let out = remove_bg_image(&rgba)?;
    out.save(output_path)?;
    log::info!("Background removed, saved to {}", output_path);
    Ok(())
}