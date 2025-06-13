#!/bin/bash
# Test script to debug movement issues

echo "Starting Earth Engine with debug logging..."
echo "Try pressing WASD keys to move and check the logs"
echo "Press Ctrl+C to stop"
echo ""

# Run with debug logging enabled
RUST_LOG=debug ./target/release/earth-engine 2>&1 | grep -E "\[process_input\]|\[render loop\].*Camera pos|\[physics\]" | head -100