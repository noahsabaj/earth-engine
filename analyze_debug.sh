#!/bin/bash

# Earth Engine Debug Analysis Script
# Analyzes captured screenshots and correlates with logs

# Configuration
DEBUG_DIR="$(pwd)/debug"
PHOTOS_DIR="$DEBUG_DIR/photos"
OUTPUT_FILE="$DEBUG_DIR/output.txt"
REPORT_FILE="$DEBUG_DIR/analysis_report.md"
ISSUES_DIR="$DEBUG_DIR/issue_frames"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Analysis thresholds
BLACK_THRESHOLD=98      # % of pixels that are black to consider screen black
BLUE_THRESHOLD=90       # % of pixels that are blue to consider blue screen
DARK_THRESHOLD=50       # % of pixels below this brightness
FLAT_THRESHOLD=95       # % of pixels with same color for flat detection
MIN_EDGE_PIXELS=1000    # Minimum edge pixels for proper rendering

# Create analysis directories
setup_directories() {
    mkdir -p "$ISSUES_DIR"
    echo -e "${GREEN}Created issues directory: $ISSUES_DIR${NC}"
}

# Check if ImageMagick is installed
check_dependencies() {
    if ! command -v convert &> /dev/null; then
        echo -e "${RED}Error: ImageMagick is not installed${NC}"
        echo "Please install it with: sudo apt-get install imagemagick"
        exit 1
    fi
    
    if ! command -v identify &> /dev/null; then
        echo -e "${RED}Error: ImageMagick identify command not found${NC}"
        exit 1
    fi
}

# Analyze a single screenshot
analyze_screenshot() {
    local image_path="$1"
    local image_name=$(basename "$image_path")
    local timestamp="${image_name#screenshot_}"
    timestamp="${timestamp%.png}"
    
    # Get image statistics using ImageMagick
    local stats=$(convert "$image_path" -colorspace Gray -format "%[mean] %[standard-deviation]" info:)
    local mean=$(echo $stats | cut -d' ' -f1)
    local std_dev=$(echo $stats | cut -d' ' -f2)
    
    # Get dominant colors
    local dominant_colors=$(convert "$image_path" -colors 5 -format "%c" histogram:info: | sort -nr | head -5)
    
    # Check for black screen (very low mean brightness)
    local black_percentage=$(convert "$image_path" -threshold 5% -format "%[fx:100*mean]" info:)
    
    # Check for blue screen (dominant blue channel)
    local blue_dominance=$(convert "$image_path" -format "%[fx:100*mean.b/(mean.r+mean.g+mean.b+0.001)]" info:)
    
    # Check for edges (indicates proper geometry rendering)
    local edge_pixels=$(convert "$image_path" -edge 1 -threshold 50% -format "%[fx:sum]" info:)
    
    # Detect issues
    local issues=""
    local severity="OK"
    
    # Black screen detection
    if (( $(echo "$mean < 10" | bc -l) )); then
        issues="${issues}BLACK_SCREEN "
        severity="CRITICAL"
        cp "$image_path" "$ISSUES_DIR/black_${image_name}"
    fi
    
    # Blue screen detection
    if (( $(echo "$blue_dominance > $BLUE_THRESHOLD" | bc -l) )); then
        issues="${issues}BLUE_SCREEN "
        severity="CRITICAL"
        cp "$image_path" "$ISSUES_DIR/blue_${image_name}"
    fi
    
    # Dark screen detection
    if (( $(echo "$mean < $DARK_THRESHOLD" | bc -l) )); then
        issues="${issues}DARK_SCREEN "
        if [ "$severity" = "OK" ]; then
            severity="WARNING"
        fi
    fi
    
    # Flat/no geometry detection
    if (( $(echo "$std_dev < 5" | bc -l) )); then
        issues="${issues}FLAT_RENDER "
        if [ "$severity" != "CRITICAL" ]; then
            severity="ERROR"
        fi
        cp "$image_path" "$ISSUES_DIR/flat_${image_name}"
    fi
    
    # Low edge count (no visible geometry)
    if (( $(echo "$edge_pixels < $MIN_EDGE_PIXELS" | bc -l) )); then
        issues="${issues}NO_GEOMETRY "
        if [ "$severity" != "CRITICAL" ]; then
            severity="ERROR"
        fi
    fi
    
    # Return analysis results
    echo "$timestamp|$severity|$mean|$std_dev|$edge_pixels|$blue_dominance|$issues"
}

# Extract relevant log entries around a timestamp
extract_logs_around_timestamp() {
    local timestamp="$1"
    local context_lines=10
    
    if [ -f "$OUTPUT_FILE" ]; then
        # Convert timestamp to a searchable format
        local search_time=$(echo "$timestamp" | sed 's/_/ /g' | cut -d' ' -f2)
        
        # Extract logs around this time
        grep -B $context_lines -A $context_lines "$search_time" "$OUTPUT_FILE" 2>/dev/null || echo "No logs found"
    else
        echo "No output.txt file found"
    fi
}

# Generate histogram of issues over time
generate_issue_timeline() {
    local analysis_file="$1"
    
    echo -e "\n${CYAN}Issue Timeline:${NC}"
    echo "==============="
    
    # Count issues by type over time
    local total_frames=$(wc -l < "$analysis_file")
    local black_frames=$(grep -c "BLACK_SCREEN" "$analysis_file" || true)
    local blue_frames=$(grep -c "BLUE_SCREEN" "$analysis_file" || true)
    local dark_frames=$(grep -c "DARK_SCREEN" "$analysis_file" || true)
    local flat_frames=$(grep -c "FLAT_RENDER" "$analysis_file" || true)
    local no_geo_frames=$(grep -c "NO_GEOMETRY" "$analysis_file" || true)
    
    echo "Total frames analyzed: $total_frames"
    echo ""
    echo "Issue counts:"
    printf "  %-20s %5d (%5.1f%%)\n" "Black screens:" "$black_frames" $(echo "scale=1; 100*$black_frames/$total_frames" | bc)
    printf "  %-20s %5d (%5.1f%%)\n" "Blue screens:" "$blue_frames" $(echo "scale=1; 100*$blue_frames/$total_frames" | bc)
    printf "  %-20s %5d (%5.1f%%)\n" "Dark screens:" "$dark_frames" $(echo "scale=1; 100*$dark_frames/$total_frames" | bc)
    printf "  %-20s %5d (%5.1f%%)\n" "Flat renders:" "$flat_frames" $(echo "scale=1; 100*$flat_frames/$total_frames" | bc)
    printf "  %-20s %5d (%5.1f%%)\n" "No geometry:" "$no_geo_frames" $(echo "scale=1; 100*$no_geo_frames/$total_frames" | bc)
}

# Find transition points where issues start/stop
find_transitions() {
    local analysis_file="$1"
    
    echo -e "\n${CYAN}Issue Transitions:${NC}"
    echo "=================="
    
    local prev_severity="OK"
    local transition_count=0
    
    while IFS='|' read -r timestamp severity mean std_dev edge_pixels blue_dom issues; do
        if [ "$severity" != "$prev_severity" ]; then
            echo -e "${YELLOW}Transition at $timestamp: $prev_severity -> $severity${NC}"
            if [ -n "$issues" ]; then
                echo "  Issues: $issues"
            fi
            echo "  Image stats: mean=$mean, std_dev=$std_dev, edges=$edge_pixels"
            
            # Extract logs around transition
            echo "  Related logs:"
            extract_logs_around_timestamp "$timestamp" | head -5 | sed 's/^/    /'
            echo ""
            
            ((transition_count++))
        fi
        prev_severity="$severity"
    done < "$analysis_file"
    
    echo "Total transitions: $transition_count"
}

# Identify patterns in rendering issues
identify_patterns() {
    local analysis_file="$1"
    
    echo -e "\n${CYAN}Pattern Analysis:${NC}"
    echo "================="
    
    # Check for cyclic patterns
    local issue_sequence=$(grep -v "OK" "$analysis_file" | cut -d'|' -f7 | tr '\n' ' ')
    
    # Look for common sequences
    echo "Common issue sequences:"
    
    # Check if issues appear in clusters
    local in_issue_cluster=false
    local cluster_start=""
    local cluster_length=0
    local clusters=()
    
    while IFS='|' read -r timestamp severity mean std_dev edge_pixels blue_dom issues; do
        if [ "$severity" != "OK" ]; then
            if [ "$in_issue_cluster" = false ]; then
                cluster_start="$timestamp"
                in_issue_cluster=true
                cluster_length=1
            else
                ((cluster_length++))
            fi
        else
            if [ "$in_issue_cluster" = true ]; then
                clusters+=("$cluster_start|$cluster_length")
                in_issue_cluster=false
            fi
        fi
    done < "$analysis_file"
    
    echo "Issue clusters found: ${#clusters[@]}"
    for cluster in "${clusters[@]}"; do
        IFS='|' read -r start length <<< "$cluster"
        echo "  Cluster at $start: $length consecutive frames with issues"
    done
}

# Generate detailed report
generate_report() {
    local analysis_file="$1"
    
    cat > "$REPORT_FILE" << EOF
# Earth Engine Debug Analysis Report
Generated: $(date)

## Summary
Debug capture location: $DEBUG_DIR
Screenshots analyzed: $(find "$PHOTOS_DIR" -name "*.png" 2>/dev/null | wc -l)
Issues found: $(find "$ISSUES_DIR" -name "*.png" 2>/dev/null | wc -l)

## Critical Findings

EOF

    # Add critical issues
    local critical_count=$(grep -c "CRITICAL" "$analysis_file" || true)
    if [ $critical_count -gt 0 ]; then
        echo "### Critical Issues Found" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        grep "CRITICAL" "$analysis_file" | while IFS='|' read -r timestamp severity mean std_dev edge_pixels blue_dom issues; do
            echo "- **$timestamp**: $issues" >> "$REPORT_FILE"
            echo "  - Mean brightness: $mean" >> "$REPORT_FILE"
            echo "  - Blue dominance: ${blue_dom}%" >> "$REPORT_FILE"
        done
        echo "" >> "$REPORT_FILE"
    fi
    
    # Add timeline
    echo "## Issue Timeline" >> "$REPORT_FILE"
    generate_issue_timeline "$analysis_file" >> "$REPORT_FILE"
    
    # Add transitions
    echo "" >> "$REPORT_FILE"
    echo "## State Transitions" >> "$REPORT_FILE"
    find_transitions "$analysis_file" >> "$REPORT_FILE"
    
    # Add patterns
    echo "" >> "$REPORT_FILE"
    echo "## Pattern Analysis" >> "$REPORT_FILE"
    identify_patterns "$analysis_file" >> "$REPORT_FILE"
    
    # Add recommendations
    cat >> "$REPORT_FILE" << EOF

## Recommendations

Based on the analysis:

EOF
    
    if [ $critical_count -gt 0 ]; then
        if grep -q "BLACK_SCREEN" "$analysis_file"; then
            echo "1. **Black screen issues detected**: Check GPU initialization and render pipeline setup" >> "$REPORT_FILE"
        fi
        if grep -q "BLUE_SCREEN" "$analysis_file"; then
            echo "2. **Blue screen issues detected**: Likely sky/clear color rendering without terrain" >> "$REPORT_FILE"
        fi
    fi
    
    if grep -q "FLAT_RENDER" "$analysis_file"; then
        echo "3. **Flat rendering detected**: Check mesh generation and vertex buffer population" >> "$REPORT_FILE"
    fi
    
    if grep -q "NO_GEOMETRY" "$analysis_file"; then
        echo "4. **Missing geometry detected**: Verify chunk loading and culling systems" >> "$REPORT_FILE"
    fi
}

# Main analysis function
main() {
    echo -e "${GREEN}Earth Engine Debug Analysis System${NC}"
    echo "===================================="
    
    # Check dependencies
    check_dependencies
    
    # Check if debug directory exists
    if [ ! -d "$DEBUG_DIR" ]; then
        echo -e "${RED}Error: Debug directory not found at $DEBUG_DIR${NC}"
        echo "Please run capture_debug.sh first to capture debug data"
        exit 1
    fi
    
    # Check if photos exist
    local photo_count=$(find "$PHOTOS_DIR" -name "*.png" 2>/dev/null | wc -l)
    if [ $photo_count -eq 0 ]; then
        echo -e "${RED}Error: No screenshots found in $PHOTOS_DIR${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}Found $photo_count screenshots to analyze${NC}"
    
    # Setup directories
    setup_directories
    
    # Create temporary analysis file
    local temp_analysis=$(mktemp)
    
    # Analyze each screenshot
    echo -e "\n${YELLOW}Analyzing screenshots...${NC}"
    local count=0
    find "$PHOTOS_DIR" -name "*.png" -type f | sort | while read -r screenshot; do
        ((count++))
        printf "\rAnalyzing: %d/%d" "$count" "$photo_count"
        analyze_screenshot "$screenshot" >> "$temp_analysis"
    done
    echo ""
    
    # Generate timeline
    generate_issue_timeline "$temp_analysis"
    
    # Find transitions
    find_transitions "$temp_analysis"
    
    # Identify patterns
    identify_patterns "$temp_analysis"
    
    # Generate report
    echo -e "\n${YELLOW}Generating report...${NC}"
    generate_report "$temp_analysis"
    
    # Cleanup
    rm -f "$temp_analysis"
    
    # Summary
    echo -e "\n${GREEN}Analysis complete!${NC}"
    echo "==================="
    echo -e "Report saved to: ${CYAN}$REPORT_FILE${NC}"
    echo -e "Issue frames copied to: ${CYAN}$ISSUES_DIR${NC}"
    
    # Show quick summary
    local issue_count=$(find "$ISSUES_DIR" -name "*.png" 2>/dev/null | wc -l)
    if [ $issue_count -gt 0 ]; then
        echo -e "\n${RED}Found $issue_count frames with issues${NC}"
        echo "Issue types:"
        ls "$ISSUES_DIR" | cut -d'_' -f1 | sort | uniq -c
    else
        echo -e "\n${GREEN}No significant issues detected${NC}"
    fi
}

# Run main function
main "$@"