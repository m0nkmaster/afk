#!/usr/bin/env bash
# Validate that AGENTS.md architecture section matches actual source structure
# Exit 0 if structure is valid, 1 if there are discrepancies

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
AGENTS_MD="$ROOT_DIR/AGENTS.md"
SRC_DIR="$ROOT_DIR/src"

# Colours for output (disabled if not a terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

errors=0
warnings=0

echo "Validating AGENTS.md structure against source tree..."
echo

# Check AGENTS.md exists
if [[ ! -f "$AGENTS_MD" ]]; then
    echo -e "${RED}ERROR: AGENTS.md not found at $AGENTS_MD${NC}"
    exit 1
fi

# Extract the architecture code block from AGENTS.md
# Find ## Architecture, then extract content between ``` markers
architecture_block=$(awk '
    /^## Architecture/ { in_arch = 1; next }
    /^## / && in_arch { exit }
    in_arch && /^```/ { 
        if (in_block) { exit } 
        else { in_block = 1; next }
    }
    in_block { print }
' "$AGENTS_MD")

if [[ -z "$architecture_block" ]]; then
    echo -e "${RED}ERROR: Could not find architecture code block in AGENTS.md${NC}"
    exit 1
fi

# Create temp files for comparison
doc_paths_file=$(mktemp)
actual_paths_file=$(mktemp)
trap 'rm -f "$doc_paths_file" "$actual_paths_file"' EXIT

# Extract file paths from the tree structure using awk
# Parse the tree format to reconstruct full paths
echo "$architecture_block" | awk '
BEGIN { 
    depth = 0
    split("", dirs)  # directory stack
    dirs[0] = "src"
}
{
    # Skip empty lines
    if (NF == 0) next
    
    # Count leading tree characters to determine depth
    # Each level is 4 chars: "│   " or "    "
    line = $0
    d = 0
    while (match(line, /^(│   |    )/)) {
        d++
        line = substr(line, 5)
    }
    
    # Remove tree chars (├── or └──) and comments
    gsub(/^[│ ]*[├└]── /, "", line)
    gsub(/ *#.*/, "", line)
    gsub(/^ +| +$/, "", line)  # trim
    
    if (line == "" || line == "src/") next
    
    if (match(line, /\/$/)) {
        # Directory - update stack
        dir_name = substr(line, 1, length(line)-1)
        dirs[d+1] = dir_name
        # Clear deeper levels
        for (i = d+2; i <= depth+1; i++) delete dirs[i]
        depth = d
    } else {
        # File - print full path
        path = ""
        for (i = 0; i <= d; i++) {
            if (dirs[i] != "") {
                if (path != "") path = path "/"
                path = path dirs[i]
            }
        }
        if (path != "") print path "/" line
        else print line
    }
}
' | grep '\.rs$' | sort -u > "$doc_paths_file"

# Get actual paths from src/
find "$SRC_DIR" -type f -name "*.rs" | sed "s|$ROOT_DIR/||" | sort -u > "$actual_paths_file"

echo "Checking documented files exist..."
missing=0
while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    if [[ ! -f "$ROOT_DIR/$path" ]]; then
        echo -e "${RED}  MISSING: $path (documented but does not exist)${NC}"
        errors=$((errors + 1))
        missing=$((missing + 1))
    fi
done < "$doc_paths_file"
if [[ $missing -eq 0 ]]; then
    echo "  All documented files exist"
fi

echo
echo "Checking for undocumented files..."
undoc=0
while IFS= read -r path; do
    [[ -z "$path" ]] && continue
    if ! grep -qxF "$path" "$doc_paths_file"; then
        echo -e "${YELLOW}  UNDOCUMENTED: $path${NC}"
        warnings=$((warnings + 1))
        undoc=$((undoc + 1))
    fi
done < "$actual_paths_file"
if [[ $undoc -eq 0 ]]; then
    echo "  All source files are documented"
fi

doc_count=$(wc -l < "$doc_paths_file" | tr -d ' ')
actual_count=$(wc -l < "$actual_paths_file" | tr -d ' ')

echo
echo "Summary:"
echo "  Documented files: $doc_count"
echo "  Actual files: $actual_count"

if [[ $errors -gt 0 ]]; then
    echo -e "${RED}  Errors: $errors (documented files that don't exist)${NC}"
fi

if [[ $warnings -gt 0 ]]; then
    echo -e "${YELLOW}  Warnings: $warnings (undocumented files)${NC}"
fi

if [[ $errors -eq 0 && $warnings -eq 0 ]]; then
    echo -e "${GREEN}  Structure is fully in sync!${NC}"
fi

echo

# Exit with error only for missing files (hard errors)
# Undocumented files are warnings only
if [[ $errors -gt 0 ]]; then
    echo -e "${RED}FAILED: Documentation references non-existent files${NC}"
    exit 1
fi

if [[ $warnings -gt 0 ]]; then
    echo -e "${YELLOW}WARNING: Some source files are not documented in AGENTS.md${NC}"
    echo "Consider updating the Architecture section in AGENTS.md"
fi

echo -e "${GREEN}PASSED: All documented files exist${NC}"
exit 0
