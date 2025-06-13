#!/bin/bash

# Earth Engine Debug Analysis Viewer
# Interactive viewer for debug analysis results

# Configuration
DEBUG_DIR="$(pwd)/debug"
ISSUES_DIR="$DEBUG_DIR/issue_frames"
REPORT_FILE="$DEBUG_DIR/analysis_report.md"
PHOTOS_DIR="$DEBUG_DIR/photos"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Show menu
show_menu() {
    echo -e "\n${CYAN}Earth Engine Debug Analysis Viewer${NC}"
    echo "==================================="
    echo "1) View analysis report"
    echo "2) List issue frames by type"
    echo "3) Create comparison montage"
    echo "4) Generate issue timeline graph"
    echo "5) Export issue frames to Windows"
    echo "6) View specific frame details"
    echo "7) Compare before/after frames"
    echo "8) Exit"
    echo ""
    read -p "Select option: " choice
    
    case $choice in
        1) view_report ;;
        2) list_issues ;;
        3) create_montage ;;
        4) generate_timeline ;;
        5) export_issues ;;
        6) view_frame_details ;;
        7) compare_frames ;;
        8) exit 0 ;;
        *) echo -e "${RED}Invalid option${NC}" ;;
    esac
}

# View analysis report
view_report() {
    if [ -f "$REPORT_FILE" ]; then
        echo -e "\n${CYAN}Analysis Report:${NC}"
        echo "================"
        cat "$REPORT_FILE"
        read -p "Press Enter to continue..."
    else
        echo -e "${RED}Report not found. Run analyze_debug.sh first.${NC}"
        read -p "Press Enter to continue..."
    fi
}

# List issues by type
list_issues() {
    if [ ! -d "$ISSUES_DIR" ]; then
        echo -e "${RED}No issues directory found${NC}"
        read -p "Press Enter to continue..."
        return
    fi
    
    echo -e "\n${CYAN}Issue Frames by Type:${NC}"
    echo "===================="
    
    for issue_type in black blue flat; do
        local count=$(find "$ISSUES_DIR" -name "${issue_type}_*.png" 2>/dev/null | wc -l)
        if [ $count -gt 0 ]; then
            echo -e "\n${YELLOW}${issue_type^^} frames ($count):${NC}"
            ls -1 "$ISSUES_DIR/${issue_type}_"*.png 2>/dev/null | head -10
            if [ $count -gt 10 ]; then
                echo "... and $((count-10)) more"
            fi
        fi
    done
    
    read -p "Press Enter to continue..."
}

# Create visual montage of issues
create_montage() {
    if ! command -v montage &> /dev/null; then
        echo -e "${RED}ImageMagick montage not available${NC}"
        read -p "Press Enter to continue..."
        return
    fi
    
    echo -e "\n${CYAN}Creating issue montage...${NC}"
    
    for issue_type in black blue flat; do
        local frames=($(find "$ISSUES_DIR" -name "${issue_type}_*.png" 2>/dev/null | head -9))
        if [ ${#frames[@]} -gt 0 ]; then
            echo "Creating ${issue_type} montage..."
            montage "${frames[@]}" -tile 3x3 -geometry 320x240+5+5 \
                    -background black -label '%f' \
                    "$DEBUG_DIR/${issue_type}_montage.png"
            echo -e "${GREEN}Created: $DEBUG_DIR/${issue_type}_montage.png${NC}"
        fi
    done
    
    read -p "Press Enter to continue..."
}

# Generate timeline graph
generate_timeline() {
    echo -e "\n${CYAN}Generating timeline visualization...${NC}"
    
    # Create a simple ASCII timeline
    local timeline_file="$DEBUG_DIR/issue_timeline.txt"
    
    echo "Issue Timeline (each character = 1 frame)" > "$timeline_file"
    echo "========================================" >> "$timeline_file"
    echo "Legend: . = OK, B = Black, b = Blue, D = Dark, F = Flat, X = Multiple" >> "$timeline_file"
    echo "" >> "$timeline_file"
    
    # Process screenshots in order
    find "$PHOTOS_DIR" -name "*.png" -type f | sort | while read -r screenshot; do
        local name=$(basename "$screenshot")
        local issue_char="."
        
        # Check what issues this frame has
        if [ -f "$ISSUES_DIR/black_$name" ]; then
            issue_char="B"
        elif [ -f "$ISSUES_DIR/blue_$name" ]; then
            issue_char="b"
        elif [ -f "$ISSUES_DIR/flat_$name" ]; then
            issue_char="F"
        fi
        
        printf "%s" "$issue_char" >> "$timeline_file"
    done
    
    echo "" >> "$timeline_file"
    
    # Show the timeline
    cat "$timeline_file"
    
    echo -e "\n${GREEN}Timeline saved to: $timeline_file${NC}"
    read -p "Press Enter to continue..."
}

# Export issues to Windows-accessible location
export_issues() {
    echo -e "\n${CYAN}Exporting issue frames...${NC}"
    
    # Create Windows-accessible directory
    local export_dir="/mnt/c/tmp/earth_engine_debug"
    mkdir -p "$export_dir"
    
    # Copy issue frames
    if [ -d "$ISSUES_DIR" ]; then
        cp -r "$ISSUES_DIR" "$export_dir/"
        echo -e "${GREEN}Issue frames exported to: C:\\tmp\\earth_engine_debug\\issue_frames${NC}"
    fi
    
    # Copy report
    if [ -f "$REPORT_FILE" ]; then
        cp "$REPORT_FILE" "$export_dir/"
        echo -e "${GREEN}Report exported to: C:\\tmp\\earth_engine_debug\\analysis_report.md${NC}"
    fi
    
    # Copy montages
    for montage in "$DEBUG_DIR"/*_montage.png; do
        if [ -f "$montage" ]; then
            cp "$montage" "$export_dir/"
        fi
    done
    
    read -p "Press Enter to continue..."
}

# View detailed frame information
view_frame_details() {
    read -p "Enter frame filename (e.g., screenshot_20250106_143022_123.png): " frame
    
    local frame_path="$PHOTOS_DIR/$frame"
    if [ ! -f "$frame_path" ]; then
        echo -e "${RED}Frame not found${NC}"
        read -p "Press Enter to continue..."
        return
    fi
    
    echo -e "\n${CYAN}Frame Details: $frame${NC}"
    echo "========================"
    
    # Get image properties
    identify -verbose "$frame_path" | grep -E "(Geometry|Colorspace|Channel statistics|Mean|Standard deviation)" | head -20
    
    # Check if it's an issue frame
    for issue_type in black blue flat; do
        if [ -f "$ISSUES_DIR/${issue_type}_$frame" ]; then
            echo -e "\n${RED}This frame has issue: ${issue_type^^}${NC}"
        fi
    done
    
    read -p "Press Enter to continue..."
}

# Compare before/after frames
compare_frames() {
    echo -e "\n${CYAN}Frame Comparison${NC}"
    echo "================"
    
    # List recent transitions
    local transitions=($(find "$ISSUES_DIR" -name "*.png" -exec basename {} \; | cut -d'_' -f2- | sort -u | head -10))
    
    if [ ${#transitions[@]} -eq 0 ]; then
        echo -e "${RED}No issue frames found${NC}"
        read -p "Press Enter to continue..."
        return
    fi
    
    echo "Recent issue frames:"
    for i in "${!transitions[@]}"; do
        echo "$((i+1))) ${transitions[$i]}"
    done
    
    read -p "Select frame number: " selection
    
    if [ $selection -lt 1 ] || [ $selection -gt ${#transitions[@]} ]; then
        echo -e "${RED}Invalid selection${NC}"
        read -p "Press Enter to continue..."
        return
    fi
    
    local selected_frame="${transitions[$((selection-1))]}"
    local base_name="screenshot_$selected_frame"
    
    # Find all versions of this frame
    echo -e "\n${CYAN}Versions of $base_name:${NC}"
    ls -la "$PHOTOS_DIR/$base_name" 2>/dev/null || echo "Original not found"
    ls -la "$ISSUES_DIR"/*"_$base_name" 2>/dev/null || echo "No issue versions"
    
    # Create side-by-side comparison if possible
    if command -v montage &> /dev/null && [ -f "$PHOTOS_DIR/$base_name" ]; then
        local issue_version=$(find "$ISSUES_DIR" -name "*_$base_name" | head -1)
        if [ -n "$issue_version" ]; then
            montage "$PHOTOS_DIR/$base_name" "$issue_version" \
                    -tile 2x1 -geometry +5+5 \
                    -label "Original vs Issue" \
                    "$DEBUG_DIR/comparison_$selected_frame.png"
            echo -e "\n${GREEN}Comparison saved to: $DEBUG_DIR/comparison_$selected_frame.png${NC}"
        fi
    fi
    
    read -p "Press Enter to continue..."
}

# Main loop
main() {
    while true; do
        clear
        show_menu
    done
}

# Check if debug directory exists
if [ ! -d "$DEBUG_DIR" ]; then
    echo -e "${RED}Debug directory not found at $DEBUG_DIR${NC}"
    echo "Please run capture_debug.sh first"
    exit 1
fi

# Run main loop
main