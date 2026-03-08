#!/bin/bash
#
# Helper script to update IMAGE_RENDERING.md with changelog entry
# for changes to image_renderer.rs
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

DOCS_FILE="docs/IMAGE_RENDERING.md"

# Check if docs file exists
if [ ! -f "$DOCS_FILE" ]; then
    echo -e "${RED}Error: $DOCS_FILE not found${NC}"
    exit 1
fi

# Get current version from Cargo.toml
VERSION=$(grep "^version" Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
TODAY=$(date +%Y-%m-%d)

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Image Rendering Documentation Update${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "Version: ${GREEN}$VERSION${NC}"
echo -e "Date: ${GREEN}$TODAY${NC}"
echo ""

# Show what changed
echo -e "${YELLOW}Changes detected in src/ui/image_renderer.rs:${NC}"
echo ""
if git diff HEAD --name-only | grep -q "src/ui/image_renderer.rs"; then
    git diff HEAD --stat src/ui/image_renderer.rs
else
    git diff --cached --stat src/ui/image_renderer.rs 2>/dev/null || echo "  (No staged changes - showing last commit)"
    git show --stat HEAD -- src/ui/image_renderer.rs | head -10
fi
echo ""

# Interactive prompt for changelog entry
echo -e "${YELLOW}Enter changelog summary (one line, or press Enter to open editor):${NC}"
read -r CHANGE_SUMMARY

if [ -z "$CHANGE_SUMMARY" ]; then
    # Create a template entry
    TEMP_FILE=$(mktemp)
    cat > "$TEMP_FILE" << EOF
### v${VERSION} (${TODAY})

- [Your change description here]
  - What changed: [brief description]
  - Why: [reason for change]
  - Impact: [effect on users/behavior]

EOF
    
    # Open in default editor
    ${EDITOR:-nano} "$TEMP_FILE"
    
    # Read the content
    CHANGE_ENTRY=$(cat "$TEMP_FILE")
    rm "$TEMP_FILE"
else
    # Use the provided summary
    CHANGE_ENTRY="### v${VERSION} (${TODAY})

- ${CHANGE_SUMMARY}

"
fi

# Find the Changelog section and insert after the header
if grep -q "## Changelog" "$DOCS_FILE"; then
    # Create backup
    cp "$DOCS_FILE" "${DOCS_FILE}.bak"
    
    # Insert after "## Changelog" line
    awk -v entry="$CHANGE_ENTRY" '
        /^## Changelog/ {
            print
            print ""
            print entry
            next
        }
        { print }
    ' "$DOCS_FILE" > "${DOCS_FILE}.tmp"
    
    mv "${DOCS_FILE}.tmp" "$DOCS_FILE"
    rm -f "${DOCS_FILE}.bak"
    
    echo ""
    echo -e "${GREEN}✓ Documentation updated successfully!${NC}"
    echo ""
    echo -e "${BLUE}Updated file:${NC} $DOCS_FILE"
    echo ""
    echo -e "${YELLOW}Don't forget to stage the changes:${NC}"
    echo "    git add $DOCS_FILE"
    echo ""
else
    echo -e "${RED}Error: Could not find '## Changelog' section in $DOCS_FILE${NC}"
    exit 1
fi
