#!/bin/bash

echo "=== Earth Engine Debug Script ==="
echo "This script will help diagnose the freeze issue"
echo ""

# Set debug environment variables
export RUST_LOG=debug
export RUST_BACKTRACE=1
export WGPU_BACKEND=vulkan  # Try vulkan first

echo "Environment variables set:"
echo "  RUST_LOG=$RUST_LOG"
echo "  RUST_BACKTRACE=$RUST_BACKTRACE"
echo "  WGPU_BACKEND=$WGPU_BACKEND"
echo ""

# Check if running in WSL
if grep -q microsoft /proc/version; then
    echo "Detected WSL environment"
    
    # Check for GPU support in WSL
    if command -v nvidia-smi &> /dev/null; then
        echo "NVIDIA GPU tools detected"
        nvidia-smi --query-gpu=name,driver_version --format=csv 2>/dev/null || echo "  Could not query GPU info"
    else
        echo "  No NVIDIA tools found - GPU support may be limited"
    fi
    
    # Check for Mesa/software rendering
    if command -v glxinfo &> /dev/null; then
        echo ""
        echo "OpenGL info:"
        glxinfo 2>/dev/null | grep -E "OpenGL vendor|OpenGL renderer|direct rendering" || echo "  Could not get OpenGL info"
    fi
fi

echo ""
echo "Building in debug mode..."
cargo build 2>&1 | tail -20

if [ $? -ne 0 ]; then
    echo "Build failed!"
    exit 1
fi

echo ""
echo "Starting application with timeout..."
echo "If it freezes, it will be killed after 30 seconds"
echo "----------------------------------------"

# Run with timeout
timeout 30s cargo run 2>&1 | tee debug_output.log

EXIT_CODE=$?

echo ""
echo "----------------------------------------"

if [ $EXIT_CODE -eq 124 ]; then
    echo "Application was killed due to timeout (freeze detected)"
    echo ""
    echo "Last 50 lines of output:"
    tail -50 debug_output.log
    
    echo ""
    echo "Trying with different backend (OpenGL)..."
    export WGPU_BACKEND=gl
    echo "WGPU_BACKEND=$WGPU_BACKEND"
    
    timeout 30s cargo run 2>&1 | tee debug_output_gl.log
    
    if [ $? -eq 124 ]; then
        echo ""
        echo "Also freezes with OpenGL backend"
    fi
else
    echo "Application exited with code: $EXIT_CODE"
fi

echo ""
echo "Debug logs saved to:"
echo "  - debug_output.log"
echo "  - debug_output_gl.log (if OpenGL was tried)"
echo ""
echo "Check for panic logs:"
ls -la logs/panic.log 2>/dev/null || echo "  No panic log found"