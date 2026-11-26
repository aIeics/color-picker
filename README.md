# Color Picker

A lightweight desktop color picker written in Rust. Pick a color from anywhere on the screen or from an image, and copy it as HEX, RGB or HSL.

Status: early base. Picking a color from the screen works on Windows; picking from an image is not implemented yet.

The UI uses the bundled Manrope font and the same dark theme as the Clock app (font license in `assets/fonts/Manrope-OFL.txt`).

## Build

Requires Rust 1.95 or newer.

```sh
cargo run                 # debug build
cargo build --release     # target/release/color-picker.exe
```

## Use

1. Click **Pick from screen**.
2. Within two seconds, move the cursor over the color you want.
3. The color shows in the preview. Copy it as HEX, RGB or HSL, or type a hex value (for example `#3B82F6`).

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
