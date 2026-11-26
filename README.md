# Color Picker

A lightweight Windows color picker written in Rust. Pick a color from anywhere on the screen or from an image, edit it, and copy it as HEX, RGB, HSL, HSV, CMYK or OKLCH.

The window is a compact black panel with a custom title bar, in the style of BlurAutoClicker. The tabs on the left of the title bar are Settings, Screen, Image, Editor, History and Palette. Pin keeps the window on top.

## Use

### Pick from the screen

Click **Pick from screen**, or press the hotkey anywhere (**Ctrl + Alt + C** by default, even when the window is hidden in the tray). A loupe next to the cursor shows the zoomed pixels and the HEX value.

- **Click** picks the color. The click doesn't reach the window under the cursor.
- **Esc** or **right-click** cancels. Pressing the hotkey again also cancels.
- **Arrow keys** move the cursor by one pixel (ten with Shift); **Enter** or **Space** picks.
- The **mouse wheel** changes the loupe zoom.

By default the picked color is copied as HEX (change this under **Copy on pick** in Settings). Every pick goes into History.

### Pick from an image

Open the **Image** tab and click **Open**, drop an image file anywhere on the window, or paste with **Ctrl+V** or **Paste**. Pasting works with images (for example a Win+Shift+S screenshot) and with paths to image files. Supported formats: PNG, JPEG, BMP, GIF (first frame) and WebP.

Scroll to zoom, drag to pan, and click a pixel to pick it. **Fit** shows the whole image. The toolbar shows the color and position under the cursor.

### Edit, history and palette

- **Editor**: type a HEX value or move the RGB, HSL or HSV sliders. Every format updates below.
- **History**: the last 48 picked colors. Click one to make it the current color. Right-click to copy it, save it to the palette or remove it.
- **Palette**: colors you saved with **Save to palette** or **Save current** (up to 96). **Clear** asks for a second click.

Click any value next to the swatch to copy it, for example `rgb(255, 0, 0)` or `oklch(62.8% 0.258 29.2)`.

### Settings

- **Pick hotkey**: click the keyboard icon and press a new shortcut (Esc cancels), or remove it with ×. It needs Ctrl, Alt, Shift or Win, except F1–F24. If another app already uses it, the field says so.
- **Sample**: pick a single pixel, or the average of 3×3 or 5×5 pixels.
- **Zoom** (the loupe's), **Copy on pick**, **HEX style** (`#ABCDEF`, `#abcdef`, `ABCDEF` or `abcdef`) and **Show** (which formats appear).
- **Always on top** and **Remember position**.
- **Tray**: **Close to tray** (default) keeps the app running in the tray when you close the window, so the hotkey still works. **Minimize to tray** hides the window when you minimize it instead. **Off** removes the tray icon, and closing quits the app. Click the tray icon to show the window; right-click it for **Pick a color** and **Exit**.
- **Start with Windows** (off by default): the app starts at sign-in, hidden in the tray with the hotkey ready.

Only one copy runs at a time. Starting it again shows the running window.

Settings, the current color, history and palette are saved to `%APPDATA%\color-picker\settings.txt`. Delete that file to reset everything.

## Build

Requires Rust 1.95 or newer.

```sh
cargo run                 # debug build, with a console for logs and panics
cargo build --release     # target/release/color-picker.exe, no console
```

Close the running app (including from the tray) before rebuilding, or Windows keeps the `.exe` locked.

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Notes

The UI uses the bundled Manrope font (license in `assets/fonts/Manrope-OFL.txt`). The icons are drawn in code. The app targets Windows: screen picking, the tray and the Open dialog use the Windows API. Other systems get stubs for those, which haven't been built or tested.
