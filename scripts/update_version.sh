#!/bin/bash
# Update version across all files

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new_version>"
    echo "Example: $0 0.27.0"
    exit 1
fi

NEW_VERSION=$1

# Validate version format (basic check)
if ! [[ $NEW_VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo "Error: Invalid version format. Expected: MAJOR.MINOR.PATCH[-TAG]"
    exit 1
fi

echo "Updating version to $NEW_VERSION..."

# Update VERSION file
echo "$NEW_VERSION" > VERSION
echo "✓ Updated VERSION file"

# Update Cargo.toml
sed -i "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml
echo "✓ Updated Cargo.toml"

# Update README.md
sed -i "s/\*\*Version: [0-9]\+\.[0-9]\+\.[0-9]\+[^*]*\*\*/\*\*Version: $NEW_VERSION\*\*/" README.md
echo "✓ Updated README.md"

# Show changes
echo -e "\nChanges made:"
git diff --name-only

echo -e "\nTo complete version update:"
echo "1. Review changes: git diff"
echo "2. Commit: git commit -am \"Bump version to $NEW_VERSION\""
echo "3. Tag: git tag -a v$NEW_VERSION -m \"Version $NEW_VERSION\""
echo "4. Push: git push origin main --tags"