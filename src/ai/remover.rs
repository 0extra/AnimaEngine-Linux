use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;

use anyhow::{anyhow, Result};
use image::{imageops::FilterType, DynamicImage, RgbaImage};
use ndarray::Array4;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;

static SESSION: OnceLock<Mutex<Session>> = OnceLock::new();
static MODEL_FILE: OnceLock<String> = OnceLock::new();
static INPUT_SIZE: OnceLock<u32> = OnceLock::new();
static MODELS_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

pub fn configure(model: &str, input_size: Option<u32>, models_dir: Option<&str>) {
    let filename = match model {
        "u2netp" => "u2netp.onnx",
        "silueta" => "silueta.onnx",
        "isnet-general-use" => "isnet-general-use.onnx",
        _ => "u2net.onnx",
    };
    let _ = MODEL_FILE.set(filename.to_string());

    let default_size = match model {
        "isnet-general-use" => 1024,
        _ => 320,
    };
    let _ = INPUT_SIZE.set(input_size.unwrap_or(default_size));

    let _ = MODELS_DIR.set(models_dir.map(PathBuf::from));

    log::info!(
        "AI remover configured: model={} input={} dir={:?}",
        filename,
        input_size.unwrap_or(default_size),
        models_dir
    );
}

fn current_model_file() -> &'static str {
    MODEL_FILE
        .get()
        .map(|s| s.as_str())
        .unwrap_or("u2net.onnx")
}

fn current_input_size() -> u32 {
    *INPUT_SIZE.get().unwrap_or(&320)
}

fn find_model() -> Option<PathBuf> {
    let filename = current_model_file();

    if let Ok(p) = env::var("ANIMA_MODEL_PATH") {
        let pb = PathBuf::from(p);
        if pb.exists() {
            log::info!("Using model from ANIMA_MODEL_PATH: {}", pb.display());
            return Some(pb);
        } else {
            log::warn!("ANIMA_MODEL_PATH set but not found: {}", pb.display());
        }
    }

    if let Some(Some(dir)) = MODELS_DIR.get() {
        let p = dir.join(filename);
        if p.exists() {
            log::info!("Using model from models_dir: {}", p.display());
            return Some(p);
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    candidates.push(PathBuf::from("models").join(filename));

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("models").join(filename));
            if let Some(up1) = dir.parent() {
                if let Some(up2) = up1.parent() {
                    candidates.push(up2.join("models").join(filename));
                }
            }
        }
    }

    candidates.push(PathBuf::from("/usr/share/anima-linux/models").join(filename));
    candidates.push(PathBuf::from("/usr/local/share/anima-linux/models").join(filename));

    if let Some(data_dir) = dirs::data_dir() {
        candidates.push(data_dir.join("anima-linux/models").join(filename));
    }
    if let Some(cfg) = dirs::config_dir() {
        candidates.push(cfg.join("anima-linux/models").join(filename));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".local/share/anima-linux/models").join(filename));
    }

    for c in &candidates {
        if c.exists() {
            log::info!("Found model at: {}", c.display());
            return Some(c.clone());
        }
    }

    None
}

fn get_session() -> Result<&'static Mutex<Session>> {
    if let Some(s) = SESSION.get() {
        return Ok(s);
    }

    let model_path = find_model().ok_or_else(|| {
        anyhow!(
            "Model not found. Looked in:\n\
             - $ANIMA_MODEL_PATH\n\
             - ./models/{}\n\
             - <exe_dir>/models/{}\n\
             - /usr/share/anima-linux/models/{}\n\
             - ~/.local/share/anima-linux/models/{}\n\
             \n\
             Download it with:\n\
             mkdir -p models && \\\n\
             wget https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx -P models/",
            current_model_file(),
            current_model_file(),
            current_model_file(),
            current_model_file()
        )
    })?;

    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file(&model_path)?;

    let _ = SESSION.set(Mutex::new(session));

    SESSION
        .get()
        .ok_or_else(|| anyhow!("SESSION init race, retry"))
}

pub fn remove_bg_image(rgba: &RgbaImage) -> Result<RgbaImage> {
    let (orig_w, orig_h) = rgba.dimensions();
    let input_size = current_input_size();

    let dynamic = DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
    let resized = image::imageops::resize(&dynamic, input_size, input_size, FilterType::Triangle);

    let mut input = Array4::<f32>::zeros((1, 3, input_size as usize, input_size as usize));
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

    let mut mask_img = image::GrayImage::new(mask_w as u32, mask_h as u32);
    for y in 0..mask_h {
        for x in 0..mask_w {
            let idx = y * mask_w + x;
            let v = if data[idx].is_finite() {
                (data[idx].clamp(0.0, 1.0) * 255.0) as u8
            } else {
                0
            };
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