#!/bin/sh
set -e

INSTALL_DIR="$HOME/.local"
BIN_DIR="$INSTALL_DIR/bin"
ICON_DIR="$INSTALL_DIR/share/icons/hicolor/scalable/apps"
DESKTOP_DIR="$INSTALL_DIR/share/applications"
APP_NAME="numnum"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

info()  { printf "${CYAN}[info]${NC} %s\n" "$1"; }
ok()    { printf "${GREEN}[ok]${NC}   %s\n" "$1"; }
warn()  { printf "${YELLOW}[warn]${NC} %s\n" "$1"; }
err()   { printf "${RED}[err]${NC}  %s\n" "$1"; }

ask() {
    printf "${CYAN}[?]${NC} %s " "$1"
    read -r answer
    echo "$answer"
}

# ── Build ───────────────────────────────────────────────────────────────

info "Building numnum (release)..."
cd "$SCRIPT_DIR"

if ! command -v cargo >/dev/null 2>&1; then
    err "cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi

cargo build --release
ok "Build complete"

# ── Install binary ──────────────────────────────────────────────────────

info "Installing to $INSTALL_DIR..."
mkdir -p "$BIN_DIR"
cp "target/release/$APP_NAME" "$BIN_DIR/$APP_NAME"
chmod +x "$BIN_DIR/$APP_NAME"
ok "Binary installed to $BIN_DIR/$APP_NAME"

# ── Install icons ───────────────────────────────────────────────────────

mkdir -p "$ICON_DIR"
cp "assets/icons/numnum-dark.svg" "$ICON_DIR/numnum.svg"
ok "Icon installed"

# ── Desktop entry ───────────────────────────────────────────────────────

mkdir -p "$DESKTOP_DIR"
cat > "$DESKTOP_DIR/numnum.desktop" << EOF
[Desktop Entry]
Name=NumNum
Comment=A text editor that does math
Exec=$BIN_DIR/numnum
Icon=numnum
Terminal=false
Type=Application
Categories=Utility;Calculator;
Keywords=calculator;math;units;currency;conversion;
StartupWMClass=numnum
EOF
ok "Desktop entry created"

# Update desktop database if available
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
fi

# ── Window manager rules ────────────────────────────────────────────────

printf "\n"
info "Window manager setup (optional)"
printf "  1) Hyprland\n"
printf "  2) Niri\n"
printf "  3) Neither / skip\n"
wm_choice=$(ask "Which window manager? [1/2/3]")

case "$wm_choice" in
    1)
        # Hyprland
        HYPR_CONF="$HOME/.config/hypr/hyprland.conf"
        if [ ! -f "$HYPR_CONF" ]; then
            warn "Hyprland config not found at $HYPR_CONF, skipping"
        elif grep -q "numnum" "$HYPR_CONF" 2>/dev/null; then
            ok "Hyprland already has numnum rules, skipping"
        else
            printf "\n"
            info "Will add to $HYPR_CONF:"
            printf "  ${YELLOW}windowrule = float on, match:class numnum${NC}\n"
            printf "  ${YELLOW}windowrule = border_size 0, match:class numnum${NC}\n"
            printf "\n"
            confirm=$(ask "Add these rules? [y/N]")
            case "$confirm" in
                y|Y|yes|Yes)
                    printf "\n# NumNum calculator\nwindowrule = float on, match:class numnum\nwindowrule = border_size 0, match:class numnum\n" >> "$HYPR_CONF"
                    ok "Hyprland rules added"
                    ;;
                *)
                    info "Skipped"
                    ;;
            esac
        fi
        ;;
    2)
        # Niri
        NIRI_CONF="$HOME/.config/niri/config.kdl"
        if [ ! -f "$NIRI_CONF" ]; then
            warn "Niri config not found at $NIRI_CONF, skipping"
        elif grep -q "numnum" "$NIRI_CONF" 2>/dev/null; then
            ok "Niri already has numnum rules, skipping"
        else
            printf "\n"
            info "Will add to $NIRI_CONF:"
            printf "  ${YELLOW}window-rule {${NC}\n"
            printf "  ${YELLOW}    match app-id=\"numnum\"${NC}\n"
            printf "  ${YELLOW}    open-floating true${NC}\n"
            printf "  ${YELLOW}    border { off }${NC}\n"
            printf "  ${YELLOW}    focus-ring { off }${NC}\n"
            printf "  ${YELLOW}}${NC}\n"
            printf "\n"
            confirm=$(ask "Add these rules? [y/N]")
            case "$confirm" in
                y|Y|yes|Yes)
                    cat >> "$NIRI_CONF" << 'NIRI_RULES'

// NumNum calculator
window-rule {
    match app-id="numnum"
    open-floating true
    border {
        off
    }
    focus-ring {
        off
    }
}
NIRI_RULES
                    ok "Niri rules added"
                    ;;
                *)
                    info "Skipped"
                    ;;
            esac
        fi
        ;;
    *)
        info "Skipped window manager setup"
        ;;
esac

# ── Done ────────────────────────────────────────────────────────────────

printf "\n"
ok "NumNum installed!"
info "Run with: numnum"
info "Make sure $BIN_DIR is in your PATH"

if ! echo "$PATH" | grep -q "$BIN_DIR"; then
    warn "$BIN_DIR is not in your PATH"
    info "Add to your shell config: export PATH=\"\$HOME/.local/bin:\$PATH\""
fi
