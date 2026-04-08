# NumNum

A notebook calculator for people who think in text. Type expressions in plain language, see results live. Units convert automatically, currencies update from the web, and everything stays in a single pane you can scroll through like a scratch pad.

Built with [GPUI](https://github.com/zed-industries/zed) (Zed's GPU-accelerated UI framework) and Rust.

## What it does

```
rent = 45000 INR                              ₹ 45,000
groceries = 12000 INR                         ₹ 12,000
utilities = rent * 5%                         ₹ 2,250
total = rent + groceries + utilities          ₹ 59,250
total in USD                                  $709.28

5 km * 3 km                                   15 km²
60 km / 2 hours                                30 km/h
220 volts * 10 amps                            2,200 W

20% of what is 11.68 cm                        58.4 cm
500 kcal in kJ                                 2,092 kJ
```

**Arithmetic** with operator precedence, parentheses, variables, and assignments.

**170+ currencies** with live exchange rates. Prefix symbols (`$`, `₹`, `€`, `£`, `¥`) and ISO codes (`USD`, `INR`, `GBP`) both work. Rates update from the Open Exchange Rates API on startup, with a hardcoded fallback for offline use.

**100+ units** across length, mass, time, temperature, area, volume, data, angular, typography, power, energy, voltage, current, resistance, and frequency. Compound units from multiplication and division (`km/h`, `kg/L`, `W`). Shorthands like `mph`, `kmh`, `kWh`. The word `per` works as a division operator.

**Percent operations**: `20% of 500`, `20% on 500`, `20% off 500`, `15% of what is 75`, inline `500 + 10%`.

**Aggregation**: `sum`, `average`, `prev` across lines.

**Representations**: `255 in hex`, `10 in binary`, scientific notation.

**Number formatting**: US (`1,234,567.89`), Indian (`12,34,567.89`), and European (`1.234.567,89`) styles.

## Features

- Syntax highlighting with theme-aware colors
- Autocomplete for functions, units, currencies, keywords, and variables
- 8 bundled color themes (Catppuccin Mocha/Latte, Tokyo Night/Day, Rose Pine Moon/Dawn, Zed One Dark/Light)
- Custom titlebar option with macOS-style traffic light buttons
- Ctrl+scroll to change font size
- Click-to-copy results (full precision or display format)
- Settings pane for font, precision, appearance, themes, number format, diagnostics
- Persistent settings and window size across sessions
- Dark/light/auto appearance modes (follows system on Linux via XDG Desktop Portal)

## Building from source

### Requirements

- Rust 1.85+ (edition 2024)
- A C/C++ compiler
- CMake
- Platform-specific libraries (see below)

### Linux (Debian/Ubuntu)

```sh
sudo apt install \
  build-essential cmake clang mold \
  libasound2-dev libfontconfig-dev libssl-dev \
  libwayland-dev libx11-xcb-dev libxkbcommon-x11-dev \
  libzstd-dev libsqlite3-dev libvulkan1 libva-dev \
  libglib2.0-dev

cargo build --release
```

### Linux (Fedora)

```sh
sudo dnf install \
  gcc g++ cmake clang mold \
  alsa-lib-devel fontconfig-devel openssl-devel \
  wayland-devel libxcb-devel libxkbcommon-x11-devel \
  libzstd-devel sqlite-devel vulkan-loader libva-devel \
  glib2-devel

cargo build --release
```

### Linux (Arch)

```sh
sudo pacman -S \
  gcc clang cmake mold \
  alsa-lib fontconfig openssl \
  wayland libxcb libxkbcommon-x11 \
  zstd sqlite vulkan-icd-loader libva \
  glib2

cargo build --release
```

### FreeBSD

```sh
sudo pkg install cmake llvm git alsa-lib libX11 sqlite3
cargo build --release
```

### macOS

Requires Xcode (for Metal shader compilation and system frameworks).

```sh
xcode-select --install
brew install cmake
cargo build --release
```

### Windows

Requires Visual Studio 2022 (or Build Tools) with the "Desktop development with C++" workload and a Windows 10/11 SDK.

```sh
cargo build --release
```

If the build fails looking for `fxc.exe` (HLSL shader compiler), set `GPUI_FXC_PATH` to your Windows SDK bin directory.

### Running

```sh
cargo run --release
```

The binary is at `target/release/numnum`.

## Configuration

Settings live in `~/.config/numnum/settings.toml` (Linux/FreeBSD), `~/Library/Application Support/numnum/settings.toml` (macOS), or `%APPDATA%/numnum/settings.toml` (Windows).

Themes are TOML files in the `themes/` subdirectory of the config folder. Drop any `.toml` theme file there and it shows up in the settings dropdown.

## License

NumNum is licensed under the [GNU General Public License v2.0](LICENSE).

The vendored GPUI crates (under `crates/`) are licensed under the [Apache License 2.0](LICENSE-APACHE), copyright Zed Industries, Inc.
