#!/bin/bash

# Earth Engine Debug Capture System
# Captures Windows screenshots and game output for debugging

# Configuration
CAPTURE_INTERVAL=0.25  # seconds between screenshots
DEBUG_DIR="$(pwd)/debug"
PHOTOS_DIR="$DEBUG_DIR/photos"
OUTPUT_FILE="$DEBUG_DIR/output.txt"
SCREENSHOT_PS1="$DEBUG_DIR/screenshot.ps1"
PID_FILE="$DEBUG_DIR/capture.pid"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Stopping capture system...${NC}"
    
    # Kill screenshot capture process
    if [ -f "$PID_FILE" ]; then
        CAPTURE_PID=$(cat "$PID_FILE")
        if kill -0 "$CAPTURE_PID" 2>/dev/null; then
            kill "$CAPTURE_PID"
            echo -e "${GREEN}Screenshot capture stopped${NC}"
        fi
        rm -f "$PID_FILE"
    fi
    
    # Kill game process if it's still running
    if [ ! -z "$GAME_PID" ] && kill -0 "$GAME_PID" 2>/dev/null; then
        echo -e "${YELLOW}Stopping game process...${NC}"
        kill "$GAME_PID"
    fi
    
    echo -e "${GREEN}Capture system stopped${NC}"
    echo -e "${GREEN}Debug files saved to: $DEBUG_DIR${NC}"
    exit 0
}

# Set up trap for clean shutdown
trap cleanup SIGINT SIGTERM

# Function to convert WSL path to Windows path
wsl_to_windows_path() {
    echo "$1" | sed 's|^/home/|C:\\Users\\|' | sed 's|/|\\|g'
}

# Create debug directories
setup_directories() {
    echo -e "${YELLOW}Setting up debug directories...${NC}"
    
    # Remove old debug data
    if [ -d "$DEBUG_DIR" ]; then
        echo "Cleaning old debug data..."
        rm -rf "$DEBUG_DIR"
    fi
    
    # Create fresh directories
    mkdir -p "$PHOTOS_DIR"
    echo -e "${GREEN}Created: $DEBUG_DIR${NC}"
    echo -e "${GREEN}Created: $PHOTOS_DIR${NC}"
}

# Create PowerShell screenshot script
create_screenshot_script() {
    echo -e "${YELLOW}Creating PowerShell screenshot script...${NC}"
    
    cat > "$SCREENSHOT_PS1" << 'EOF'
# PowerShell script to capture Windows screenshots
param(
    [string]$OutputPath
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

# Get screen bounds
$screen = [System.Windows.Forms.Screen]::PrimaryScreen
$bounds = $screen.Bounds

# Create bitmap
$bitmap = New-Object System.Drawing.Bitmap($bounds.Width, $bounds.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)

# Capture screen
$graphics.CopyFromScreen($bounds.X, $bounds.Y, 0, 0, $bounds.Size)

# Save screenshot
$bitmap.Save($OutputPath, [System.Drawing.Imaging.ImageFormat]::Png)

# Cleanup
$graphics.Dispose()
$bitmap.Dispose()
EOF
    
    chmod +x "$SCREENSHOT_PS1"
    echo -e "${GREEN}PowerShell script created${NC}"
}

# Function to capture screenshots
capture_screenshots() {
    echo -e "${YELLOW}Starting screenshot capture (every ${CAPTURE_INTERVAL}s)...${NC}"
    
    # Save PID for cleanup
    echo $$ > "$PID_FILE"
    
    # Convert photos directory to Windows path
    WIN_PHOTOS_DIR=$(wsl_to_windows_path "$PHOTOS_DIR")
    
    COUNTER=0
    while true; do
        # Generate timestamp and filename
        TIMESTAMP=$(date +%Y%m%d_%H%M%S_%3N)
        FILENAME="screenshot_${TIMESTAMP}.png"
        WIN_FILEPATH="${WIN_PHOTOS_DIR}\\${FILENAME}"
        
        # Capture screenshot using PowerShell
        powershell.exe -ExecutionPolicy Bypass -File "$(wslpath -w "$SCREENSHOT_PS1")" -OutputPath "$WIN_FILEPATH" 2>/dev/null
        
        if [ $? -eq 0 ]; then
            COUNTER=$((COUNTER + 1))
            printf "\rCaptured: %d screenshots" "$COUNTER"
        else
            echo -e "\n${RED}Failed to capture screenshot${NC}"
        fi
        
        sleep "$CAPTURE_INTERVAL"
    done
}

# Main execution
main() {
    echo -e "${GREEN}Earth Engine Debug Capture System${NC}"
    echo "=================================="
    
    # Setup
    setup_directories
    create_screenshot_script
    
    # Check if game is already running
    EXISTING_PID=$(pgrep -f "earth-engine" | head -n 1)
    if [ ! -z "$EXISTING_PID" ]; then
        echo -e "${YELLOW}Found existing Earth Engine process (PID: $EXISTING_PID)${NC}"
        read -p "Attach to existing process? (y/n): " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            GAME_PID=$EXISTING_PID
            echo -e "${GREEN}Attached to existing process${NC}"
        else
            echo -e "${YELLOW}Starting new Earth Engine process...${NC}"
            # Start the game and redirect output
            npm start > "$OUTPUT_FILE" 2>&1 &
            GAME_PID=$!
            echo -e "${GREEN}Game started (PID: $GAME_PID)${NC}"
            sleep 2  # Give the game time to start
        fi
    else
        echo -e "${YELLOW}Starting Earth Engine...${NC}"
        # Start the game and redirect output
        npm start > "$OUTPUT_FILE" 2>&1 &
        GAME_PID=$!
        echo -e "${GREEN}Game started (PID: $GAME_PID)${NC}"
        sleep 2  # Give the game time to start
    fi
    
    # Start screenshot capture in background
    capture_screenshots &
    CAPTURE_PID=$!
    echo $CAPTURE_PID > "$PID_FILE"
    
    echo -e "\n${GREEN}Capture system running!${NC}"
    echo "======================="
    echo "Screenshots: $PHOTOS_DIR"
    echo "Game output: $OUTPUT_FILE"
    echo -e "\n${YELLOW}Press Ctrl+C to stop${NC}\n"
    
    # Monitor game process
    if [ ! -z "$GAME_PID" ]; then
        while kill -0 "$GAME_PID" 2>/dev/null; do
            sleep 1
        done
        echo -e "\n${YELLOW}Game process ended${NC}"
        cleanup
    else
        # Just wait for interrupt if we couldn't find/start the game
        wait
    fi
}

# Run main function
main