#!/bin/bash

# Helper script to view debug information

DEBUG_DIR="$(pwd)/debug"
PHOTOS_DIR="$DEBUG_DIR/photos"
OUTPUT_FILE="$DEBUG_DIR/output.txt"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}Earth Engine Debug Information${NC}"
echo "=============================="

# Check if debug directory exists
if [ ! -d "$DEBUG_DIR" ]; then
    echo -e "${YELLOW}No debug directory found. Run ./capture_debug.sh first.${NC}"
    exit 1
fi

# Count screenshots
if [ -d "$PHOTOS_DIR" ]; then
    PHOTO_COUNT=$(find "$PHOTOS_DIR" -name "*.png" 2>/dev/null | wc -l)
    echo -e "${BLUE}Screenshots captured:${NC} $PHOTO_COUNT"
    
    if [ $PHOTO_COUNT -gt 0 ]; then
        # Show first and last screenshots
        FIRST=$(ls -1 "$PHOTOS_DIR"/*.png 2>/dev/null | head -n 1)
        LAST=$(ls -1 "$PHOTOS_DIR"/*.png 2>/dev/null | tail -n 1)
        echo -e "${BLUE}First screenshot:${NC} $(basename "$FIRST")"
        echo -e "${BLUE}Last screenshot:${NC} $(basename "$LAST")"
        
        # Calculate capture duration
        if [ "$FIRST" != "$LAST" ]; then
            FIRST_TIME=$(basename "$FIRST" | cut -d'_' -f2-3 | tr '_' ' ')
            LAST_TIME=$(basename "$LAST" | cut -d'_' -f2-3 | tr '_' ' ')
            echo -e "${BLUE}Capture duration:${NC} from $FIRST_TIME to $LAST_TIME"
        fi
    fi
else
    echo -e "${YELLOW}No photos directory found${NC}"
fi

echo

# Check output file
if [ -f "$OUTPUT_FILE" ]; then
    OUTPUT_SIZE=$(wc -l < "$OUTPUT_FILE")
    echo -e "${BLUE}Game output lines:${NC} $OUTPUT_SIZE"
    
    if [ $OUTPUT_SIZE -gt 0 ]; then
        echo -e "\n${BLUE}Last 10 lines of game output:${NC}"
        echo "------------------------------"
        tail -n 10 "$OUTPUT_FILE"
    fi
else
    echo -e "${YELLOW}No output file found${NC}"
fi

echo -e "\n${GREEN}Debug files location:${NC} $DEBUG_DIR"

# Check if capture is still running
PID_FILE="$DEBUG_DIR/capture.pid"
if [ -f "$PID_FILE" ]; then
    CAPTURE_PID=$(cat "$PID_FILE")
    if kill -0 "$CAPTURE_PID" 2>/dev/null; then
        echo -e "${YELLOW}Capture is still running (PID: $CAPTURE_PID)${NC}"
    fi
fi