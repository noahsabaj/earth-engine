#\!/bin/bash
echo "DOP Compliance Check - Sprint 37"
echo "Checking for OOP violations..."
SELF_METHODS=$(grep -r "fn.*(&.*self" src --include="*.rs" 2>/dev/null  < /dev/null |  wc -l)
echo "Methods with self: $SELF_METHODS"
if [ "$SELF_METHODS" -gt 0 ]; then
    echo "❌ DOP violations found"
    exit 1
else
    echo "✅ DOP compliant"
    exit 0
fi
