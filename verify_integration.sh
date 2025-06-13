#!/bin/bash

# Integration Verification Script for Earth Engine
# Quick verification of core system integration status

echo "==================== EARTH ENGINE INTEGRATION VERIFICATION ===================="
echo "Verifying core system integration status..."
echo

# Check compilation status first
echo "=== 1. COMPILATION STATUS ==="
echo "Checking if the engine compiles successfully..."

if cargo check --lib --quiet 2>/dev/null; then
    echo "✅ Library compilation: SUCCESS"
    LIB_COMPILES=true
else
    echo "❌ Library compilation: FAILED"
    LIB_COMPILES=false
fi

if cargo check --example integration_test --quiet 2>/dev/null; then
    echo "✅ Integration test compilation: SUCCESS" 
    INTEGRATION_COMPILES=true
else
    echo "❌ Integration test compilation: FAILED"
    INTEGRATION_COMPILES=false
fi

# Run integration tests if compilation succeeded
if [ "$INTEGRATION_COMPILES" = true ]; then
    echo
    echo "=== 2. INTEGRATION TEST RESULTS ==="
    echo "Running comprehensive integration test suite..."
    
    # Run the integration test with timeout
    timeout 30s cargo run --example integration_test > integration_results.log 2>&1
    TEST_EXIT_CODE=$?
    
    if [ $TEST_EXIT_CODE -eq 0 ]; then
        echo "✅ Integration tests completed successfully"
        
        # Parse results
        MOVEMENT_PASSED=$(grep -c "Player Movement Integration: PASSED" integration_results.log)
        SPAWN_PASSED=$(grep -c "Spawn Finder Integration: PASSED" integration_results.log)
        SAVE_PASSED=$(grep -c "Save/Load Integration: PASSED" integration_results.log)
        COORD_PASSED=$(grep -c "System Coordination: PASSED" integration_results.log)
        
        echo "  - Player Movement: $([ $MOVEMENT_PASSED -gt 0 ] && echo "✅ WORKING" || echo "❌ BROKEN")"
        echo "  - Spawn Finder: $([ $SPAWN_PASSED -gt 0 ] && echo "✅ WORKING" || echo "❌ BROKEN")"
        echo "  - Save/Load: $([ $SAVE_PASSED -gt 0 ] && echo "✅ WORKING" || echo "❌ BROKEN")"
        echo "  - System Coordination: $([ $COORD_PASSED -gt 0 ] && echo "✅ WORKING" || echo "❌ BROKEN")"
        
        TOTAL_PASSED=$((MOVEMENT_PASSED + SPAWN_PASSED + SAVE_PASSED + COORD_PASSED))
        
    elif [ $TEST_EXIT_CODE -eq 124 ]; then
        echo "⚠️ Integration tests timed out (possible infinite loop or hang)"
        TOTAL_PASSED=0
    else
        echo "❌ Integration tests failed with exit code: $TEST_EXIT_CODE"
        TOTAL_PASSED=0
    fi
    
else
    echo
    echo "=== 2. INTEGRATION TEST RESULTS ==="
    echo "❌ Cannot run integration tests - compilation failed"
    TOTAL_PASSED=0
fi

# Check for basic examples
echo
echo "=== 3. BASIC FUNCTIONALITY CHECK ==="
echo "Testing basic engine functionality..."

if timeout 5s cargo run --example test_movement_input > basic_test.log 2>&1; then
    echo "✅ Basic movement test: SUCCESS"
    BASIC_WORKING=true
else
    echo "❌ Basic movement test: FAILED"
    BASIC_WORKING=false
fi

# Analyze unwrap count (panic safety)
echo
echo "=== 4. PANIC SAFETY ANALYSIS ==="
echo "Checking for remaining unwrap() calls that can cause panics..."

UNWRAP_COUNT=$(grep -r "\.unwrap()" src/ --include="*.rs" | grep -v "test" | grep -v "example" | wc -l)
if [ $UNWRAP_COUNT -eq 0 ]; then
    echo "✅ Zero unwrap() calls found - excellent panic safety"
    PANIC_SAFE=true
elif [ $UNWRAP_COUNT -lt 10 ]; then
    echo "⚠️ $UNWRAP_COUNT unwrap() calls found - mostly safe"
    PANIC_SAFE=true
else
    echo "❌ $UNWRAP_COUNT unwrap() calls found - panic risk exists"
    PANIC_SAFE=false
fi

# Check compilation warnings
echo
echo "=== 5. CODE QUALITY ANALYSIS ==="
echo "Checking for compilation warnings..."

WARNING_COUNT=$(cargo check --message-format=short 2>&1 | grep -c "warning:")
if [ $WARNING_COUNT -eq 0 ]; then
    echo "✅ Zero compilation warnings - excellent code quality"
elif [ $WARNING_COUNT -lt 50 ]; then
    echo "✅ $WARNING_COUNT compilation warnings - acceptable quality"
else
    echo "⚠️ $WARNING_COUNT compilation warnings - code quality needs attention"
fi

# Overall Assessment
echo
echo "==================== OVERALL INTEGRATION ASSESSMENT ===================="

SCORE=0
[ "$LIB_COMPILES" = true ] && SCORE=$((SCORE + 1))
[ "$INTEGRATION_COMPILES" = true ] && SCORE=$((SCORE + 1))
[ $TOTAL_PASSED -ge 3 ] && SCORE=$((SCORE + 2))
[ $TOTAL_PASSED -eq 4 ] && SCORE=$((SCORE + 1))
[ "$BASIC_WORKING" = true ] && SCORE=$((SCORE + 1))
[ "$PANIC_SAFE" = true ] && SCORE=$((SCORE + 1))

echo "Integration Score: $SCORE/7"
echo

case $SCORE in
    7)
        echo "🎉 INTEGRATION STATUS: EXCELLENT"
        echo "All systems are properly integrated and working!"
        echo "The engine is ready for production use."
        ;;
    5-6)
        echo "✅ INTEGRATION STATUS: GOOD"
        echo "Most systems are working well with minor issues."
        echo "Engine is suitable for development and testing."
        ;;
    3-4)
        echo "⚠️ INTEGRATION STATUS: FAIR"
        echo "Some core systems working, but significant issues remain."
        echo "Engine needs work before being fully functional."
        ;;
    1-2)
        echo "❌ INTEGRATION STATUS: POOR"
        echo "Major integration problems detected."
        echo "Engine requires significant fixes before use."
        ;;
    0)
        echo "💥 INTEGRATION STATUS: BROKEN"
        echo "Critical failures prevent basic functionality."
        echo "Engine is not usable in current state."
        ;;
esac

echo
echo "=== DETAILED RESULTS ==="
echo "Compilation: $([ "$LIB_COMPILES" = true ] && echo "✅" || echo "❌") Library, $([ "$INTEGRATION_COMPILES" = true ] && echo "✅" || echo "❌") Tests"
echo "Integration: $TOTAL_PASSED/4 core systems working"
echo "Basic Tests: $([ "$BASIC_WORKING" = true ] && echo "✅ Working" || echo "❌ Broken")"
echo "Panic Safety: $([ "$PANIC_SAFE" = true ] && echo "✅ Safe" || echo "❌ Risky") ($UNWRAP_COUNT unwraps)"
echo "Code Quality: $WARNING_COUNT warnings"

echo
echo "=== NEXT STEPS ==="
if [ $SCORE -ge 5 ]; then
    echo "1. Address any remaining minor issues"
    echo "2. Enhance test coverage"
    echo "3. Performance optimization"
    echo "4. User experience improvements"
elif [ $SCORE -ge 3 ]; then
    echo "1. Fix failing integration tests"
    echo "2. Reduce unwrap() calls for stability"
    echo "3. Address compilation warnings"
    echo "4. Test system coordination"
elif [ $SCORE -ge 1 ]; then
    echo "1. Fix basic compilation issues"
    echo "2. Resolve core system failures"
    echo "3. Implement proper error handling"
    echo "4. Basic functionality restoration"
else
    echo "1. Emergency compilation fixes needed"
    echo "2. Restore basic functionality"
    echo "3. Complete system integration review"
    echo "4. Fundamental architecture check"
fi

echo
echo "Full test logs saved to: integration_results.log, basic_test.log"
echo "==================== VERIFICATION COMPLETE ===================="