# Earth Engine Debug Analysis Tools

This suite of tools helps diagnose rendering issues in Earth Engine by analyzing captured screenshots and logs.

## Overview

The debug analysis system consists of several scripts that work together:

1. **capture_debug.sh** - Captures screenshots and game output
2. **analyze_debug.sh** - Analyzes screenshots for rendering issues
3. **view_debug_analysis.sh** - Interactive viewer for analysis results
4. **debug_log_analyzer.sh** - Analyzes log files for errors and patterns

## Quick Start

### 1. Capture Debug Data
First, capture the rendering issue:
```bash
./capture_debug.sh
# Let the game run and reproduce the issue
# Press Ctrl+C to stop capture
```

This creates:
- `debug/photos/` - Screenshots taken every 0.25 seconds
- `debug/output.txt` - Game console output

### 2. Analyze Screenshots
Run the analysis to identify rendering issues:
```bash
./analyze_debug.sh
```

This detects:
- **Black screens** - Completely black frames (GPU init failure?)
- **Blue screens** - Sky-only rendering (no terrain)
- **Dark screens** - Very low brightness (lighting issues?)
- **Flat renders** - No depth/variation (mesh generation failure?)
- **Missing geometry** - No visible edges (empty vertex buffers?)

Creates:
- `debug/analysis_report.md` - Detailed analysis report
- `debug/issue_frames/` - Copies of problematic frames

### 3. Analyze Logs
Check the game logs for errors:
```bash
./debug_log_analyzer.sh
```

This searches for:
- GPU initialization errors
- Shader compilation failures
- Buffer allocation issues
- Mesh generation problems
- Camera positioning errors
- Panic/crash messages

Creates:
- `debug/log_analysis.md` - Log analysis report

### 4. View Results
Interactive viewer for analysis results:
```bash
./view_debug_analysis.sh
```

Options:
1. View analysis report
2. List issue frames by type
3. Create visual montages
4. Generate issue timeline
5. Export to Windows
6. View frame details
7. Compare frames

## Understanding Results

### Analysis Report Structure

The analysis report includes:

1. **Summary** - Overview of frames analyzed and issues found
2. **Critical Findings** - Frames with severe rendering problems
3. **Issue Timeline** - Statistical breakdown of issue types
4. **State Transitions** - When rendering state changes occur
5. **Pattern Analysis** - Clusters and sequences of issues
6. **Recommendations** - Suggested fixes based on findings

### Common Issue Patterns

#### Black Screen at Start
- **Symptom**: First few frames are black
- **Likely Cause**: GPU initialization delay
- **Check**: Log for GPU adapter selection

#### Blue Screen After Movement
- **Symptom**: Screen turns blue when moving
- **Likely Cause**: Camera outside terrain, sky-only render
- **Check**: Camera position and chunk loading

#### Flickering Black/Normal
- **Symptom**: Alternating black and normal frames
- **Likely Cause**: Double buffering issue or render timing
- **Check**: Frame presentation and swap chain

#### Progressive Darkness
- **Symptom**: Screen gets darker over time
- **Likely Cause**: Lighting calculation accumulation error
- **Check**: Lighting system and time-of-day

### Correlating Screenshots with Logs

The analysis tools timestamp everything for correlation:

1. Screenshot: `screenshot_20250106_143022_123.png`
   - Format: `screenshot_YYYYMMDD_HHMMSS_MS.png`

2. Find corresponding log entries:
   ```bash
   grep "14:30:22" debug/output.txt
   ```

3. Look for events around that time:
   - Chunk loads/unloads
   - Mesh generation
   - Error messages
   - State changes

## Advanced Usage

### Custom Analysis Thresholds

Edit thresholds in `analyze_debug.sh`:
```bash
BLACK_THRESHOLD=98      # % pixels for black screen
BLUE_THRESHOLD=90       # % pixels for blue screen  
DARK_THRESHOLD=50       # Brightness threshold
FLAT_THRESHOLD=95       # % same color for flat
MIN_EDGE_PIXELS=1000    # Minimum edges for geometry
```

### Batch Processing

Analyze multiple debug sessions:
```bash
for dir in debug_session_*; do
    ./analyze_debug.sh "$dir"
done
```

### Export for Sharing

Export results to Windows:
```bash
# Creates C:\tmp\earth_engine_debug\
./view_debug_analysis.sh
# Select option 5
```

### Creating Video from Captures

Create a video from screenshots:
```bash
ffmpeg -framerate 4 -pattern_type glob -i 'debug/photos/*.png' \
       -c:v libx264 -pix_fmt yuv420p debug_capture.mp4
```

## Troubleshooting

### "ImageMagick not installed"
```bash
sudo apt-get install imagemagick
```

### "No screenshots found"
- Ensure `capture_debug.sh` ran successfully
- Check Windows permissions for PowerShell
- Verify `debug/photos/` directory exists

### Analysis takes too long
- Reduce number of screenshots analyzed
- Adjust capture interval in `capture_debug.sh`
- Use sampling: `find debug/photos -name "*.png" | sort | awk 'NR%4==0'`

## Tips

1. **Reproduce Consistently**: Try to reproduce the issue in the same way each capture
2. **Capture Transitions**: Include both working and broken states
3. **Check Timestamps**: Issues often correlate with specific game events
4. **Multiple Captures**: Run several captures to identify patterns
5. **Clean Between Runs**: Remove old debug data before new captures

## File Locations

After running all tools:
```
debug/
├── photos/                    # All captured screenshots
├── issue_frames/              # Problematic frames
│   ├── black_*.png           # Black screen frames
│   ├── blue_*.png            # Blue screen frames
│   └── flat_*.png            # Flat render frames
├── output.txt                # Game console output
├── analysis_report.md        # Screenshot analysis
├── log_analysis.md           # Log analysis
├── issue_timeline.txt        # ASCII timeline
├── black_montage.png         # Visual montages
├── blue_montage.png
└── flat_montage.png
```