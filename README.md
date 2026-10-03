# Anima-Linux

An open-source alternative to [Anima Engine](https://store.steampowered.com/app/2617260/Anima_Engine/) for Linux. Overlay GIFs, videos, images, and animated WebP files on top of all windows with click-through support, plus AI-powered background removal.

Built for Wayland compositors (Hyprland, Sway, DriftWM, KDE Plasma 6) with fallback to X11.

## Features

- **Always-on-top overlay** via `wlr-layer-shell` (Hyprland, Sway, DriftWM, KDE Plasma 6)
- **Multiple formats** — GIF, PNG, JPG, WebP (static & animated), MP4, WebM, MKV, MOV, AVI
- **AI background removal** — powered by [ISNet](https://github.com/xuebinqin/U-2-Net) (q8 quantized, ~44 MB)
- **Per-frame AI for GIF** — removes background from every frame, keeps animation
- **Per-frame AI for video** — extracts frames, processes them, re-encodes to WebM
- **Drag & drop positioning** — move overlays anywhere on screen with your mouse
- **Size slider** — 32 to 800 px
- **Speed slider** — 0.25x to 3.0x (video, GIF, WebP)
- **Library with previews** — dark themed, styled after Anima Engine
- **Progress dialog** — for long-running AI tasks on GIF/video

## Requirements

- Arch Linux (or any distro with GTK4 + GStreamer 1.28+)
- Rust 1.75+
- GStreamer with the following plugins:
  - `gstreamer`
  - `gst-plugins-base`
  - `gst-plugins-good`
  - `gst-plugins-bad`
  - `gst-plugins-ugly`
  - `gst-libav`
  - `gst-plugin-gtk4` (provides `gtk4paintablesink`)
- ONNX Runtime (system library)

### Install on Arch Linux

```bash
sudo pacman -S --needed \
  gtk4 gtk4-layer-shell rust \
  gstreamer gst-plugins-base gst-plugins-good gst-plugins-bad \
  gst-plugins-ugly gst-libav gst-plugin-gtk4 \
  onnxruntime-cpu pkg-config gobject-introspection cairo
For NVIDIA GPU acceleration, replace onnxruntime-cpu with onnxruntime-cuda.

Build
bash
git clone https://github.com/yourusername/Anima-Linux.git
cd Anima-Linux
cargo build --release
First build takes 5–15 minutes because of GStreamer and ONNX Runtime bindings.

AI Model Setup
Download the ISNet model (~44 MB, quantized):

bash
cd Anima-Linux
python3 -c "
from huggingface_hub import hf_hub_download
import shutil, os
p = hf_hub_download(
    repo_id='SacredNoir/isnet-general-use-onnx',
    filename='isnet-general-use-q8.onnx'
)
shutil.copy(p, 'models/isnet-general-use.onnx')
"
Or use the U2-Net model as a fallback (larger, slower):

bash
wget https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx -P models/
Place the model in models/isnet-general-use.onnx and it will be picked up automatically.

Usage
bash
cargo run --release
# or after building:
./target/release/anima-linux
Click Add from PC to load files into your library

Select an item from the grid — its preview appears on the right

Adjust Size and Speed sliders

Check Remove background (AI) if you want the background removed

Click Launch animation — the overlay appears on top of all windows

Drag the overlay with your mouse to position it anywhere

Use Active overlays list on the right to close individual overlays or all at once

Hyprland Configuration
Add these rules to ~/.config/hypr/hyprland.conf for correct overlay behavior:

text
windowrulev2 = float, class:^(com.github.anima-linux)$
windowrulev2 = pin, class:^(com.github.anima-linux)$
windowrulev2 = noinitialfocus, class:^(com.github.anima-linux)$
DriftWM Configuration
Add to your DriftWM config:

toml
[[window_rules]]
app_id = "com.github.anima-linux"
widget = true
Project Structure
text
src/
├── main.rs              — entry point
├── app.rs               — app bootstrap
├── ui/
│   └── main_window.rs   — main GUI (library, previews, controls)
├── overlay/
│   ├── layer_surface.rs — wlr-layer-shell integration
│   └── renderer.rs      — image/GIF/WebP/video rendering
├── ai/
│   ├── remover.rs       — ISNet inference via ONNX Runtime
│   ├── gif_remover.rs   — per-frame GIF processing
│   └── video_remover.rs — per-frame video processing (VP8/WebM)
├── config/
│   └── parser.rs        — TOML config
└── utils/
    ├── gif.rs           — GIF frame decoder
    ├── webp.rs          — animated WebP decoder
    └── video.rs         — GStreamer pipeline
Known Limitations
AI for video is slow — about 1–2 seconds per frame on CPU. A 10-second video at 30 fps takes ~5 minutes.

AI is not available on GNOME Wayland — wlr-layer-shell is not supported by Mutter. Overlays will appear as regular windows.

Click-through is currently not implemented — overlays capture mouse clicks. Planned for a future release.

Library is in-memory — it does not persist between launches. Planned for a future release.

License
MIT — see LICENSE.

Acknowledgements
ISNet by Xuebin Qin — background removal model

GStreamer — video decoding/encoding

gtk4-rs — Rust bindings for GTK4

Inspired by Anima Engine
