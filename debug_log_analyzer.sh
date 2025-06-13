#!/bin/bash

# Earth Engine Debug Log Analyzer
# Analyzes output.txt for common rendering issues

DEBUG_DIR="$(pwd)/debug"
OUTPUT_FILE="$DEBUG_DIR/output.txt"
LOG_REPORT="$DEBUG_DIR/log_analysis.md"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Common error patterns
declare -A ERROR_PATTERNS=(
    ["GPU_INIT"]="GPU.*initialization|failed.*create.*device|adapter.*not.*found"
    ["SHADER_ERROR"]="shader.*compilation.*failed|WGSL.*error|pipeline.*error"
    ["BUFFER_ERROR"]="buffer.*creation.*failed|out.*of.*memory|allocation.*failed"
    ["MESH_ERROR"]="mesh.*generation.*failed|vertex.*buffer.*empty|no.*vertices"
    ["CHUNK_ERROR"]="chunk.*loading.*failed|chunk.*not.*found|invalid.*chunk"
    ["CAMERA_ERROR"]="camera.*inside.*terrain|camera.*position.*invalid|view.*matrix"
    ["RENDER_ERROR"]="render.*pass.*failed|command.*buffer.*error|submission.*failed"
    ["PANIC"]="panic|PANIC|thread.*panicked"
    ["CULLING"]="frustum.*culling|all.*chunks.*culled|nothing.*to.*render"
    ["TEXTURE"]="texture.*not.*found|atlas.*error|texture.*binding.*failed"
)

analyze_logs() {
    if [ ! -f "$OUTPUT_FILE" ]; then
        echo -e "${RED}No output.txt found at $OUTPUT_FILE${NC}"
        exit 1
    fi
    
    echo -e "${CYAN}Analyzing logs...${NC}"
    
    # Start report
    cat > "$LOG_REPORT" << EOF
# Earth Engine Log Analysis Report
Generated: $(date)
Log file: $OUTPUT_FILE

## Summary
Log size: $(du -h "$OUTPUT_FILE" | cut -f1)
Total lines: $(wc -l < "$OUTPUT_FILE")

## Error Analysis

EOF
    
    # Check each error pattern
    for error_type in "${!ERROR_PATTERNS[@]}"; do
        local pattern="${ERROR_PATTERNS[$error_type]}"
        local count=$(grep -iE "$pattern" "$OUTPUT_FILE" | wc -l)
        
        if [ $count -gt 0 ]; then
            echo "### $error_type Issues ($count occurrences)" >> "$LOG_REPORT"
            echo "" >> "$LOG_REPORT"
            
            # Get first few examples
            echo "Examples:" >> "$LOG_REPORT"
            echo '```' >> "$LOG_REPORT"
            grep -iE "$pattern" "$OUTPUT_FILE" | head -5 >> "$LOG_REPORT"
            echo '```' >> "$LOG_REPORT"
            echo "" >> "$LOG_REPORT"
            
            # Print to terminal
            echo -e "${RED}Found $error_type issues: $count occurrences${NC}"
        fi
    done
    
    # Check for performance issues
    echo "## Performance Analysis" >> "$LOG_REPORT"
    echo "" >> "$LOG_REPORT"
    
    # Look for FPS drops
    local fps_lines=$(grep -iE "fps|FPS|frame.*time" "$OUTPUT_FILE" | wc -l)
    if [ $fps_lines -gt 0 ]; then
        echo "### Frame Rate Information" >> "$LOG_REPORT"
        echo '```' >> "$LOG_REPORT"
        grep -iE "fps|FPS|frame.*time" "$OUTPUT_FILE" | tail -10 >> "$LOG_REPORT"
        echo '```' >> "$LOG_REPORT"
        echo "" >> "$LOG_REPORT"
    fi
    
    # Check for warnings
    local warning_count=$(grep -iE "warn|WARN|warning|WARNING" "$OUTPUT_FILE" | wc -l)
    if [ $warning_count -gt 0 ]; then
        echo "### Warnings ($warning_count)" >> "$LOG_REPORT"
        echo '```' >> "$LOG_REPORT"
        grep -iE "warn|WARN|warning|WARNING" "$OUTPUT_FILE" | head -10 >> "$LOG_REPORT"
        if [ $warning_count -gt 10 ]; then
            echo "... and $((warning_count-10)) more warnings" >> "$LOG_REPORT"
        fi
        echo '```' >> "$LOG_REPORT"
        echo "" >> "$LOG_REPORT"
    fi
    
    # Extract initialization sequence
    echo "## Initialization Sequence" >> "$LOG_REPORT"
    echo '```' >> "$LOG_REPORT"
    head -50 "$OUTPUT_FILE" | grep -iE "init|start|create|load" >> "$LOG_REPORT"
    echo '```' >> "$LOG_REPORT"
    echo "" >> "$LOG_REPORT"
    
    # Look for state transitions
    echo "## State Transitions" >> "$LOG_REPORT"
    echo "" >> "$LOG_REPORT"
    
    # Check for chunk loading patterns
    local chunk_loads=$(grep -iE "chunk.*load|loading.*chunk" "$OUTPUT_FILE" | wc -l)
    local chunk_unloads=$(grep -iE "chunk.*unload|unloading.*chunk" "$OUTPUT_FILE" | wc -l)
    echo "- Chunks loaded: $chunk_loads" >> "$LOG_REPORT"
    echo "- Chunks unloaded: $chunk_unloads" >> "$LOG_REPORT"
    
    # Check for mesh generation
    local mesh_gen=$(grep -iE "mesh.*generat|building.*mesh|vertices.*generated" "$OUTPUT_FILE" | wc -l)
    echo "- Mesh generations: $mesh_gen" >> "$LOG_REPORT"
    
    # Add recommendations
    echo "" >> "$LOG_REPORT"
    echo "## Recommendations" >> "$LOG_REPORT"
    echo "" >> "$LOG_REPORT"
    
    # Generate recommendations based on findings
    if grep -qiE "${ERROR_PATTERNS[GPU_INIT]}" "$OUTPUT_FILE"; then
        echo "1. **GPU Initialization Issues**: Check WebGPU support and adapter selection" >> "$LOG_REPORT"
    fi
    
    if grep -qiE "${ERROR_PATTERNS[SHADER_ERROR]}" "$OUTPUT_FILE"; then
        echo "2. **Shader Compilation Errors**: Review WGSL shader code for syntax errors" >> "$LOG_REPORT"
    fi
    
    if grep -qiE "${ERROR_PATTERNS[MESH_ERROR]}" "$OUTPUT_FILE"; then
        echo "3. **Mesh Generation Problems**: Verify chunk data and meshing algorithm" >> "$LOG_REPORT"
    fi
    
    if grep -qiE "${ERROR_PATTERNS[CAMERA_ERROR]}" "$OUTPUT_FILE"; then
        echo "4. **Camera Issues**: Check spawn position and camera initialization" >> "$LOG_REPORT"
    fi
    
    echo -e "\n${GREEN}Log analysis complete!${NC}"
    echo -e "Report saved to: ${CYAN}$LOG_REPORT${NC}"
}

# Show quick summary
show_summary() {
    echo -e "\n${CYAN}Quick Summary:${NC}"
    echo "=============="
    
    if [ -f "$OUTPUT_FILE" ]; then
        # Get last few lines to see current state
        echo -e "\n${YELLOW}Last 10 log entries:${NC}"
        tail -10 "$OUTPUT_FILE"
        
        # Check if game crashed
        if grep -qiE "panic|PANIC|thread.*panicked" "$OUTPUT_FILE"; then
            echo -e "\n${RED}⚠️  PANIC DETECTED - Game crashed!${NC}"
            echo "Last panic:"
            grep -iE "panic|PANIC|thread.*panicked" "$OUTPUT_FILE" | tail -1
        fi
        
        # Check for critical errors
        local critical_errors=$(grep -iE "error|ERROR|fail|FAIL" "$OUTPUT_FILE" | wc -l)
        if [ $critical_errors -gt 0 ]; then
            echo -e "\n${RED}Found $critical_errors error messages${NC}"
        fi
    fi
}

# Main execution
main() {
    echo -e "${GREEN}Earth Engine Debug Log Analyzer${NC}"
    echo "==============================="
    
    if [ ! -d "$DEBUG_DIR" ]; then
        echo -e "${RED}Debug directory not found at $DEBUG_DIR${NC}"
        exit 1
    fi
    
    analyze_logs
    show_summary
}

main "$@"