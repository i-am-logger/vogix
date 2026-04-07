#!/usr/bin/env bash
# Vogix demo — theme switching across apps, shader, and hardware
# Start: yoga | Tour: gruvbox, catppuccin, dracula, nord | End: yoga

TYPING_SPEED=0.025
COMMAND_PAUSE=0.6
RESULT_PAUSE=1.2

type_text() {
  local text="$1"
  for ((i = 0; i < ${#text}; i++)); do
    echo -n "${text:i:1}"
    sleep "$TYPING_SPEED"
  done
  echo ""
}

show() { echo "$@"; }
pause() { sleep "${1:-$RESULT_PAUSE}"; }

run_cmd() {
  echo ""
  type_text "  \$ $1"
  pause "$COMMAND_PAUSE"
  eval "$1" 2>&1 | sed 's/^/  /'
  pause "$RESULT_PAUSE"
}

narrate() {
  echo ""
  type_text "  $1"
}

section() {
  clear
  echo ""
  echo "  ─────────────────────────────────────────────────────────"
  type_text "  $1"
  echo "  ─────────────────────────────────────────────────────────"
  echo ""
  pause 0.5
}

main() {
  clear
  echo ""
  echo ""
  type_text "  vogix"
  echo ""
  pause 1
  type_text "  a runtime UX subsystem for NixOS."
  pause 1
  type_text "  one command changes every surface — apps, shader, hardware."
  pause 1
  type_text "  no rebuild. no restart."
  pause 3

  # What is vogix
  section "what is vogix?"
  narrate "vogix manages the visual experience across your entire system."
  narrate "terminal colors, editor themes, status bar, screen shader —"
  narrate "and physical hardware: cooler ring, keyboard LEDs, memory RGB."
  pause 1
  narrate "everything follows the theme. everything changes together."
  pause 3

  # Start with yoga
  section "yoga — the default theme"
  narrate "yoga is a monochromatic theme with conventional functional colors."
  narrate "green for success, red for danger, blue for links."
  narrate "the base palette is warm and neutral."
  pause 1
  run_cmd "vogix theme set --theme yoga --variant night"
  run_cmd "vogix theme status"
  pause 2

  # Explain what changed
  narrate "that single command just updated:"
  narrate "  alacritty, wezterm, btop, bat, ripgrep, helix"
  narrate "  the monochromatic screen shader"
  narrate "  the Kraken cooler ring"
  narrate "  the keyboard backlight"
  narrate "  the DDR5 memory RGB"
  pause 3

  # Light variant
  section "polarity — dark to light"
  narrate "every theme has variants. yoga has night and day."
  narrate "switching polarity inverts the monochromatic scale."
  narrate "functional colors adapt for contrast against the new background."
  pause 1
  run_cmd "vogix theme set --variant day"
  pause 3

  # Gruvbox
  section "gruvbox"
  narrate "earthy, warm, retro. a different monochromatic hue entirely."
  narrate "the ring, keyboard, and RAM shift to match."
  pause 1
  run_cmd "vogix theme set --theme gruvbox --variant dark"
  pause 3

  # Catppuccin
  section "catppuccin"
  narrate "soft pastels. the same architecture, different palette."
  pause 1
  run_cmd "vogix theme set --theme catppuccin --variant mocha"
  pause 3

  # Dracula
  section "dracula"
  narrate "high contrast. purples and greens against deep background."
  pause 1
  run_cmd "vogix theme set --theme dracula"
  pause 3

  # Nord
  section "nord"
  narrate "arctic blue. cold and clean."
  pause 1
  run_cmd "vogix theme set --theme nord"
  pause 3

  # Back to yoga
  section "back to yoga"
  narrate "full circle. every surface returns."
  pause 1
  run_cmd "vogix theme set --theme yoga --variant night"
  pause 3

  # How it works
  section "how it works"
  narrate "vogix pre-generates themed configs for every app at build time."
  narrate "at runtime, switching themes updates a single symlink."
  narrate "apps detect the change and reload — no process restart."
  pause 1
  narrate "hardware gets color commands with palette values resolved"
  narrate "from the current theme. liquidctl for the cooler ring,"
  narrate "OpenRGB for keyboard and memory."
  pause 1
  narrate "the shader generates a monochromatic GLSL tint from the"
  narrate "theme's base palette hue. applied via Hyprland screen_shader."
  pause 3

  # Design
  section "the vogix16 design system"
  narrate "16 colors. 8 monochromatic shades (background to foreground)."
  narrate "8 functional colors (success, warning, danger, active, link...)."
  pause 1
  narrate "functional colors are designed to be vibrant against the"
  narrate "monochromatic backdrop. they carry meaning, not decoration."
  pause 1
  narrate "hardware surfaces — ring, keyboard, RAM — get the monochromatic"
  narrate "base. they are atmosphere, not accent."
  pause 3

  # End
  clear
  echo ""
  echo ""
  type_text "  vogix"
  echo ""
  pause 1
  type_text "  one command. every surface."
  pause 1
  type_text "  github.com/i-am-logger/vogix"
  echo ""
  pause 5
}

main
