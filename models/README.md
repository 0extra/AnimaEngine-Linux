# AI Models for Background Removal

This project uses **U²-Net** via ONNX Runtime for background removal.

## Download

Run from the project root:

    mkdir -p models
    wget https://github.com/danielgatis/rembg/releases/download/v0.0.0/u2net.onnx -P models/

The model is ~176 MB. Place it in `models/u2net.onnx` and the app will use it automatically.

## License

U²-Net is released under the **Apache License 2.0**, which permits commercial use.
See the [original repository](https://github.com/xuebinqin/U-2-Net) for details.