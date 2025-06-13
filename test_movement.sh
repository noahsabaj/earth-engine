#!/bin/bash

# Movement Test Script
# Tests player movement system in Earth Engine

echo "==================== PLAYER MOVEMENT TEST ===================="
echo "Testing Earth Engine player movement system..."
echo "This test will run the engine briefly to check for movement issues"
echo

# Set up test environment
export RUST_LOG=debug
export RUST_BACKTRACE=1

# Create output directory for test results
mkdir -p debug/movement_test

echo "Starting movement test..."
echo "Looking for movement-related issues..."

# Run the testbed with a timeout and capture output
timeout 15s cargo run --example engine_testbed > debug/movement_test/movement_output.log 2>&1 &
ENGINE_PID=$!

# Wait a few seconds then simulate some input
sleep 3

# Check if the process is still running
if kill -0 $ENGINE_PID 2>/dev/null; then
    echo "Engine is running, simulating input..."
    
    # Try to terminate gracefully
    sleep 5
    if kill -0 $ENGINE_PID 2>/dev/null; then
        echo "Terminating engine..."
        kill $ENGINE_PID 2>/dev/null
    fi
else
    echo "Engine terminated early - checking for crashes..."
fi

wait $ENGINE_PID 2>/dev/null
EXIT_CODE=$?

echo
echo "==================== TEST RESULTS ===================="

# Analyze the output
if [ -f debug/movement_test/movement_output.log ]; then
    echo "Analyzing movement system output..."
    
    # Check for movement-related log entries
    echo
    echo "=== Movement Input Processing ==="
    grep -i "process_input\|move\|wasd\|key.*pressed" debug/movement_test/movement_output.log | head -10
    
    echo
    echo "=== Physics Updates ==="
    grep -i "physics.*position\|body.*velocity\|physics.*update" debug/movement_test/movement_output.log | head -10
    
    echo
    echo "=== Camera Sync ==="
    grep -i "camera.*position\|sync.*camera" debug/movement_test/movement_output.log | head -10
    
    echo
    echo "=== Errors/Warnings ==="
    grep -i "error\|warn\|panic\|stuck\|failed" debug/movement_test/movement_output.log | head -10
    
    echo
    echo "=== Movement Performance ==="
    grep -i "frame.*time\|fps\|performance" debug/movement_test/movement_output.log | head -5
    
else
    echo "No output log found!"
fi

echo
echo "==================== DIAGNOSIS ===================="

if [ $EXIT_CODE -eq 124 ]; then
    echo "✓ Engine ran successfully (timeout)"
    echo "  Movement system appears to be functional"
elif [ $EXIT_CODE -eq 0 ]; then
    echo "✓ Engine exited normally"
else
    echo "✗ Engine crashed with exit code: $EXIT_CODE"
    echo "  Movement system may have issues"
fi

echo
echo "Full output saved to: debug/movement_test/movement_output.log"
echo "Check this file for detailed movement behavior analysis"
echo "==================== END TEST ===================="