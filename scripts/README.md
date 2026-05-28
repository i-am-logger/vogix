# Vogix Scripts

Development and demo scripts for Vogix.

## Demo

### `demo.sh`

Automated demo script with typing effects for recording with OBS or screen capture.

```bash
# Run in test VM or on your system
./scripts/demo.sh
```

**What it does:**
- Simulates typing commands with configurable speed
- Shows theme switching between yoga, sepia, and ocean_depths
- Demonstrates both dark and light variants
- Displays fastfetch color palette for each theme
- Includes explanatory comments and architecture diagram
- Fully automated - runs from start to finish

**Used by:** `vogix-demo` command in test VM

**Recording:** Start OBS, run this script, capture the terminal window

**Configuration:** Edit typing speed and pauses at the top of `demo.sh`

## Theme Development

Theme development scripts have moved to the [vogix16-themes](https://github.com/i-am-logger/vogix16-themes) repository.

See the [vogix16-themes scripts documentation](https://github.com/i-am-logger/vogix16-themes/tree/main/scripts) for:
- `extract-themes.py` - Extract colors from SVG files
- `validate-themes.py` - Validate theme structure  
- `verify-theme-colors.py` - Verify colors match SVGs
- `preview-themes.sh` - Preview theme colors in terminal
