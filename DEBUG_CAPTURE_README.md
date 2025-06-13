# Earth Engine Debug Capture System

A comprehensive screenshot and logging system for debugging Earth Engine rendering issues. This system captures Windows screenshots while running the game in WSL.

## Features

- **Automatic Windows screenshot capture** every 0.25 seconds
- **Game output logging** (stdout and stderr) to `debug/output.txt`
- **Organized file structure** with timestamped screenshots
- **Clean session management** - automatically clears old debug data
- **Graceful shutdown** with Ctrl+C

## Prerequisites

1. **PowerShell** must be accessible from WSL (usually pre-installed)
2. **npm** and Earth Engine dependencies installed
3. **Windows display** where the game is visible

## File Structure

```
earth-engine/
├── capture_debug.sh      # Main capture script
├── stop_capture.sh       # Helper to stop capture
├── view_debug.sh         # View debug information
└── debug/                # Created when running
    ├── photos/           # Screenshot files
    ├── output.txt        # Game console output
    ├── screenshot.ps1    # Auto-generated PowerShell script
    └── capture.pid       # Process ID file
```

## Usage

### Start Capture System

```bash
./capture_debug.sh
```

This will:
1. Clear any existing debug data
2. Start the Earth Engine game (or attach to existing process)
3. Begin capturing screenshots every 0.25 seconds
4. Log all game output to `debug/output.txt`

### Stop Capture System

**Option 1: Ctrl+C**
Press `Ctrl+C` in the terminal running the capture script.

**Option 2: Stop script**
```bash
./stop_capture.sh
```

### View Debug Information

```bash
./view_debug.sh
```

Shows:
- Number of screenshots captured
- First and last screenshot timestamps
- Last 10 lines of game output
- Debug file locations

## Screenshot Files

Screenshots are saved as:
```
debug/photos/screenshot_YYYYMMDD_HHMMSS_MS.png
```

Example: `screenshot_20241206_143052_125.png`

## Tips

1. **Performance**: The 0.25s capture interval captures 4 screenshots per second. Adjust `CAPTURE_INTERVAL` in the script if needed.

2. **Storage**: Screenshots can accumulate quickly. Each session clears the previous debug data automatically.

3. **Viewing Screenshots**: Use Windows Explorer to navigate to the photos directory and view as a slideshow.

4. **Game Already Running**: If Earth Engine is already running, the script will ask if you want to attach to it.

5. **Output Monitoring**: You can tail the output file in another terminal:
   ```bash
   tail -f debug/output.txt
   ```

## Troubleshooting

### "Failed to capture screenshot"
- Ensure PowerShell is accessible from WSL
- Check Windows permissions for screenshot capture
- Make sure the game window is visible

### No screenshots appearing
- Check if `debug/photos/` directory exists
- Verify PowerShell execution: `powershell.exe -Command "echo test"`
- Look for errors in the terminal output

### Game won't start
- Check if another instance is already running: `pgrep -f earth-engine`
- Verify npm dependencies are installed: `npm install`
- Check `debug/output.txt` for startup errors

## Customization

Edit `capture_debug.sh` to modify:
- `CAPTURE_INTERVAL`: Time between screenshots (default: 0.25 seconds)
- `DEBUG_DIR`: Location of debug files (default: `./debug`)
- Screenshot format or quality (modify PowerShell script section)