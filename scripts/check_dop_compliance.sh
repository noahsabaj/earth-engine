#!/bin/bash
set -e

# Earth Engine DOP Compliance Checker
# Sprint 37: DOP Reality Check
# This script automatically detects OOP anti-patterns and enforces DOP compliance

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Earth Engine DOP Compliance Check${NC}"
echo -e "${BLUE}=====================================${NC}"

# Initialize counters
VIOLATIONS=0
TOTAL_CHECKS=0

# Function to check for violations
check_pattern() {
    local pattern="$1"
    local description="$2"
    local files="$3"
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    
    echo -e "\n${YELLOW}Checking: ${description}${NC}"
    
    if [ -n "$files" ]; then
        echo -e "${RED}❌ VIOLATION: ${description}${NC}"
        echo "$files"
        VIOLATIONS=$((VIOLATIONS + 1))
        return 1
    else
        echo -e "${GREEN}✅ PASSED${NC}"
        return 0
    fi
}

# Function to count occurrences
count_pattern() {
    local pattern="$1"
    local description="$2"
    
    local count=$(grep -r "$pattern" src --include="*.rs" | wc -l)
    echo -e "${BLUE}📊 ${description}: ${count}${NC}"
}

# Change to project root
cd "$(dirname "$0")/.."

echo -e "\n${BLUE}Scanning src/ directory for OOP anti-patterns...${NC}"

# 1. Check for methods with self parameter (excluding constructors)
echo -e "\n${YELLOW}1. Checking for methods with &self or &mut self parameters...${NC}"
SELF_METHODS=$(grep -r "fn [^(]*([^)]*&[[:space:]]*mut[[:space:]]*self" src --include="*.rs" | grep -v "fn new\|fn default\|fn clone\|fn fmt\|fn eq\|fn ne\|fn drop" | head -20)
check_pattern "" "Methods with &self or &mut self (excluding constructors)" "$SELF_METHODS"

# 2. Check for trait objects (dynamic dispatch)
echo -e "\n${YELLOW}2. Checking for trait objects (Box<dyn Trait>)...${NC}"
TRAIT_OBJECTS=$(grep -r "Box<dyn " src --include="*.rs")
check_pattern "" "Trait objects (Box<dyn Trait>)" "$TRAIT_OBJECTS"

# 3. Check for builder patterns
echo -e "\n${YELLOW}3. Checking for builder patterns...${NC}"
BUILDERS=$(grep -r "fn build(self)" src --include="*.rs")
check_pattern "" "Builder patterns (fn build(self))" "$BUILDERS"

# 4. Check for method chaining patterns
echo -e "\n${YELLOW}4. Checking for method chaining patterns...${NC}"
METHOD_CHAINS=$(grep -r "\.\w\+(\w*)\.\w\+(" src --include="*.rs" | grep -v test | head -10)
if [ -n "$METHOD_CHAINS" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Potential method chaining detected (sample):${NC}"
    echo "$METHOD_CHAINS"
fi

# 5. Check for Array of Structs patterns (anti-DOP)
echo -e "\n${YELLOW}5. Checking for Array of Structs patterns...${NC}"
AOS_PATTERNS=$(grep -r "Vec<[A-Z][a-zA-Z]*>" src --include="*.rs" | grep -v "Vec<f32>\|Vec<u32>\|Vec<i32>\|Vec<u8>\|Vec<String>\|Vec<PathBuf>" | head -10)
if [ -n "$AOS_PATTERNS" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Potential Array of Structs (should be Structure of Arrays):${NC}"
    echo "$AOS_PATTERNS"
fi

# 6. Check for HashMap usage (prefer flat arrays)
echo -e "\n${YELLOW}6. Checking for HashMap usage (prefer flat arrays)...${NC}"
HASHMAPS=$(grep -r "HashMap<" src --include="*.rs" | wc -l)
if [ "$HASHMAPS" -gt 10 ]; then
    echo -e "${YELLOW}⚠️  WARNING: High HashMap usage ($HASHMAPS instances) - consider flat arrays${NC}"
fi

# 7. Check for Vec::push in hot paths (allocation in loops)
echo -e "\n${YELLOW}7. Checking for Vec::push in loops (hot path allocations)...${NC}"
VEC_PUSH_LOOPS=$(grep -A 3 -B 3 "for.*{.*\.push(" src --include="*.rs")
if [ -n "$VEC_PUSH_LOOPS" ]; then
    echo -e "${YELLOW}⚠️  WARNING: Vec::push in loops detected - consider pre-allocation${NC}"
fi

# 8. Statistics gathering
echo -e "\n${BLUE}📊 DOP Compliance Statistics${NC}"
echo -e "${BLUE}=============================${NC}"

count_pattern "impl.*\{" "Total impl blocks"
count_pattern "fn.*\(&.*self" "Methods with self"
count_pattern "struct.*\{" "Total structs"
count_pattern "pub fn [^(]*\([^)]*data:" "Functions accepting data parameters"

# 9. Check for GPU-compatible data layouts
echo -e "\n${YELLOW}9. Checking for GPU-compatible patterns...${NC}"
GPU_BUFFERS=$(grep -r "wgpu::Buffer\|Buffer<" src --include="*.rs" | wc -l)
SOA_PATTERNS=$(grep -r "_x.*_y.*_z\|positions.*velocities" src --include="*.rs" | wc -l)

echo -e "${BLUE}📊 GPU Compatibility:${NC}"
echo -e "   GPU Buffers: $GPU_BUFFERS"
echo -e "   SoA Patterns: $SOA_PATTERNS"

# 10. Performance-critical file analysis
echo -e "\n${YELLOW}10. Analyzing performance-critical files...${NC}"

PERFORMANCE_FILES=(
    "src/renderer/"
    "src/world_gpu/"
    "src/physics_data/"
    "src/particles/"
    "src/lighting/"
)

for dir in "${PERFORMANCE_FILES[@]}"; do
    if [ -d "$dir" ]; then
        methods_count=$(grep -r "fn.*(&.*self" "$dir" --include="*.rs" | wc -l)
        if [ "$methods_count" -gt 0 ]; then
            echo -e "${RED}❌ Performance-critical directory $dir has $methods_count methods with self${NC}"
            VIOLATIONS=$((VIOLATIONS + 1))
        else
            echo -e "${GREEN}✅ $dir is method-free${NC}"
        fi
    fi
done

# 11. Check for proper DOP examples
echo -e "\n${YELLOW}11. Checking for proper DOP patterns...${NC}"

# Look for kernel functions (functions that take data and transform it)
KERNEL_FUNCTIONS=$(grep -r "pub fn [^(]*([^)]*data.*:" src --include="*.rs" | wc -l)
UPDATE_FUNCTIONS=$(grep -r "pub fn update_" src --include="*.rs" | wc -l)
PROCESS_FUNCTIONS=$(grep -r "pub fn process_\|pub fn apply_\|pub fn calculate_" src --include="*.rs" | wc -l)

echo -e "${GREEN}✅ DOP Pattern Analysis:${NC}"
echo -e "   Kernel functions: $KERNEL_FUNCTIONS"
echo -e "   Update functions: $UPDATE_FUNCTIONS"
echo -e "   Process functions: $PROCESS_FUNCTIONS"

# 12. Check for pre-allocated pools
POOL_PATTERNS=$(grep -r "Pool\|with_capacity\|Vec::new()" src --include="*.rs" | wc -l)
echo -e "   Pool/Pre-allocation patterns: $POOL_PATTERNS"

# Final report
echo -e "\n${BLUE}📋 Final DOP Compliance Report${NC}"
echo -e "${BLUE}===============================${NC}"

if [ "$VIOLATIONS" -eq 0 ]; then
    echo -e "${GREEN}✅ NO CRITICAL VIOLATIONS FOUND${NC}"
    echo -e "${GREEN}✅ Earth Engine maintains DOP compliance${NC}"
    echo -e "${GREEN}✅ All $TOTAL_CHECKS critical checks passed${NC}"
    
    # Check if we have good DOP patterns
    if [ "$KERNEL_FUNCTIONS" -gt 50 ]; then
        echo -e "${GREEN}✅ Strong DOP pattern adoption ($KERNEL_FUNCTIONS kernel functions)${NC}"
    else
        echo -e "${YELLOW}⚠️  Consider adding more kernel functions (current: $KERNEL_FUNCTIONS)${NC}"
    fi
    
    exit 0
else
    echo -e "${RED}❌ FOUND $VIOLATIONS CRITICAL VIOLATIONS${NC}"
    echo -e "${RED}❌ Earth Engine DOP compliance FAILED${NC}"
    echo -e "${RED}❌ Must fix violations before merge${NC}"
    
    echo -e "\n${BLUE}💡 Remediation Guide:${NC}"
    echo -e "   1. Replace methods with external kernel functions"
    echo -e "   2. Convert Array of Structs to Structure of Arrays"
    echo -e "   3. Remove trait objects, use data-driven dispatch"
    echo -e "   4. See docs/guides/DOP_ENFORCEMENT.md for patterns"
    
    exit 1
fi