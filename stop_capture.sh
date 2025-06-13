#!/bin/bash

# Helper script to stop the capture system

DEBUG_DIR="$(pwd)/debug"
PID_FILE="$DEBUG_DIR/capture.pid"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

if [ -f "$PID_FILE" ]; then
    CAPTURE_PID=$(cat "$PID_FILE")
    if kill -0 "$CAPTURE_PID" 2>/dev/null; then
        echo -e "${YELLOW}Stopping capture process (PID: $CAPTURE_PID)...${NC}"
        kill -SIGINT "$CAPTURE_PID"
        echo -e "${GREEN}Capture system stopped${NC}"
    else
        echo -e "${RED}No active capture process found${NC}"
        rm -f "$PID_FILE"
    fi
else
    echo -e "${RED}No capture PID file found${NC}"
fi

# Also try to stop any earth-engine processes
GAME_PID=$(pgrep -f "earth-engine" | head -n 1)
if [ ! -z "$GAME_PID" ]; then
    echo -e "${YELLOW}Stopping Earth Engine (PID: $GAME_PID)...${NC}"
    kill "$GAME_PID"
fi