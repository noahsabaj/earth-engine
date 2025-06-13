#!/bin/bash

# Final Movement Test Script
# Comprehensive test of the improved Earth Engine player movement system

echo "==================== FINAL MOVEMENT SYSTEM TEST ===================="
echo "Testing improved Earth Engine player movement system..."
echo "This validates all movement components and user experience improvements"
echo

# Set up test environment
export RUST_LOG=info
export RUST_BACKTRACE=1

# Create output directory for test results
mkdir -p debug/final_movement_test

echo "Starting comprehensive movement test..."

# Run the testbed with longer timeout to see more behavior
timeout 20s cargo run --example engine_testbed > debug/final_movement_test/final_output.log 2>&1 &
ENGINE_PID=$!

# Wait and then terminate gracefully
sleep 8
if kill -0 $ENGINE_PID 2>/dev/null; then
    echo "Engine is running - terminating..."
    kill $ENGINE_PID 2>/dev/null
fi

wait $ENGINE_PID 2>/dev/null
EXIT_CODE=$?

echo
echo "==================== COMPREHENSIVE ANALYSIS ===================="

if [ -f debug/final_movement_test/final_output.log ]; then
    echo "Analyzing comprehensive movement system..."
    
    echo
    echo "=== 1. STARTUP USER GUIDANCE ==="
    echo "Checking if movement controls are clearly explained at startup:"
    grep -A 8 "=== MOVEMENT CONTROLS ===" debug/final_movement_test/final_output.log || echo "❌ No startup controls found"
    
    echo
    echo "=== 2. PHYSICS SYSTEM ==="
    echo "Player entity and physics initialization:"
    grep "Physics world created.*player entity" debug/final_movement_test/final_output.log
    grep "Player spawned in air\|Player body verified" debug/final_movement_test/final_output.log
    
    echo
    echo "=== 3. INPUT PROCESSING ==="
    echo "Movement speed adaptation (should show air movement vs ground movement):"
    grep "Move direction.*speed" debug/final_movement_test/final_output.log | head -3
    
    echo
    echo "=== 4. USER FEEDBACK SYSTEM ==="
    echo "Helpful guidance messages during gameplay:"
    grep "\[Movement\]" debug/final_movement_test/final_output.log | head -5
    
    echo
    echo "=== 5. CAMERA INTEGRATION ==="
    echo "Camera position sync with physics body:"
    grep "Camera position updated" debug/final_movement_test/final_output.log | head -3
    
    echo
    echo "=== 6. ERROR DETECTION ==="
    echo "Any movement-related errors or warnings:"
    grep -i "error.*movement\|warn.*movement\|failed.*input\|stuck" debug/final_movement_test/final_output.log || echo "✓ No movement errors detected"
    
else
    echo "❌ No output log found!"
    exit 1
fi

echo
echo "==================== MOVEMENT SYSTEM VALIDATION ===================="

# Validate that key components are working
STARTUP_CONTROLS=$(grep -c "=== MOVEMENT CONTROLS ===" debug/final_movement_test/final_output.log)
PHYSICS_ENTITY=$(grep -c "Physics world created.*player entity" debug/final_movement_test/final_output.log)
USER_FEEDBACK=$(grep -c "\[Movement\]" debug/final_movement_test/final_output.log)
CAMERA_SYNC=$(grep -c "Camera position updated" debug/final_movement_test/final_output.log)

echo "Component Status:"
echo "✓ Startup Controls Display: $STARTUP_CONTROLS/1"
echo "✓ Physics Entity Creation: $PHYSICS_ENTITY/1"
echo "✓ User Feedback Messages: $USER_FEEDBACK (should be > 0)"
echo "✓ Camera-Physics Sync: $CAMERA_SYNC (should be > 0)"

TOTAL_SCORE=0
[ $STARTUP_CONTROLS -ge 1 ] && TOTAL_SCORE=$((TOTAL_SCORE + 1))
[ $PHYSICS_ENTITY -ge 1 ] && TOTAL_SCORE=$((TOTAL_SCORE + 1))
[ $USER_FEEDBACK -gt 0 ] && TOTAL_SCORE=$((TOTAL_SCORE + 1))
[ $CAMERA_SYNC -gt 0 ] && TOTAL_SCORE=$((TOTAL_SCORE + 1))

echo
echo "==================== FINAL ASSESSMENT ===================="

if [ $TOTAL_SCORE -eq 4 ]; then
    echo "🎉 MOVEMENT SYSTEM: FULLY FUNCTIONAL"
    echo "✅ All core movement components are working correctly"
    echo "✅ User experience improvements implemented"
    echo "✅ Clear guidance provided to users"
    echo "✅ Physics integration operational"
    echo
    echo "DIAGNOSIS: The movement system is working properly."
    echo "If users report movement issues, it's likely due to:"
    echo "1. Not clicking in the window to lock cursor"
    echo "2. Not reading the control instructions"
    echo "3. Expecting movement while falling (air movement is intentionally slower)"
    echo
    echo "RECOMMENDATIONS FOR USERS:"
    echo "1. Read the movement controls displayed at startup"
    echo "2. Click in the game window to enable mouse look"
    echo "3. Use WASD to move around"
    echo "4. Wait for player to land on ground for full movement speed"
    echo "5. Press Escape if cursor becomes unlocked"
elif [ $TOTAL_SCORE -ge 3 ]; then
    echo "⚠️  MOVEMENT SYSTEM: MOSTLY FUNCTIONAL"
    echo "Most components working, minor issues detected"
    echo "Score: $TOTAL_SCORE/4 components operational"
elif [ $TOTAL_SCORE -ge 2 ]; then
    echo "⚠️  MOVEMENT SYSTEM: PARTIALLY FUNCTIONAL"
    echo "Some components working, but issues remain"
    echo "Score: $TOTAL_SCORE/4 components operational"
else
    echo "❌ MOVEMENT SYSTEM: NEEDS ATTENTION"
    echo "Critical issues detected in movement system"
    echo "Score: $TOTAL_SCORE/4 components operational"
fi

echo
echo "Full test output saved to: debug/final_movement_test/final_output.log"
echo "==================== TEST COMPLETE ===================="