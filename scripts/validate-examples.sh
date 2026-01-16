#!/usr/bin/env bash
# Validate that code examples in documentation are valid
# Checks:
# 1. Shell command examples reference valid afk subcommands
# 2. Cargo commands are syntactically valid
# Exit 0 if all valid, 1 if there are errors

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

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

echo "Validating code examples in documentation..."
echo

# Build afk to get valid subcommands
if ! cargo build --release --quiet 2>/dev/null; then
    echo -e "${RED}ERROR: Failed to build afk${NC}"
    exit 1
fi

AFK_BIN="$ROOT_DIR/target/release/afk"

# Get valid afk subcommands from help output
valid_subcommands=$("$AFK_BIN" --help 2>/dev/null | awk '/^Commands:$/,/^$/' | grep -E '^\s+\w+' | awk '{print $1}' | sort -u)

# Add common aliases and built-in options
valid_subcommands="$valid_subcommands
help
--help
-h
--version
-V"

# Function to check if an afk subcommand is valid
is_valid_subcommand() {
    local cmd="$1"
    echo "$valid_subcommands" | grep -qxF "$cmd"
}

# Function to extract and validate bash code blocks from a markdown file
validate_file() {
    local file="$1"
    local in_bash_block=0
    local line_num=0
    local block_start=0
    local block_content=""
    
    while IFS= read -r line || [[ -n "$line" ]]; do
        ((line_num++)) || true
        
        # Check for start of bash/shell code block
        if [[ "$line" =~ ^\`\`\`(bash|shell|sh)$ ]]; then
            in_bash_block=1
            block_start=$line_num
            block_content=""
            continue
        fi
        
        # Check for end of code block
        if [[ $in_bash_block -eq 1 && "$line" =~ ^\`\`\` ]]; then
            in_bash_block=0
            
            # Validate each command in the block
            while IFS= read -r cmd_line; do
                # Skip empty lines and comments
                [[ -z "$cmd_line" || "$cmd_line" =~ ^[[:space:]]*# ]] && continue
                
                # Skip lines that are just output (don't start with command)
                [[ "$cmd_line" =~ ^[[:space:]]+(Completed|Warning|Error|✓|✗) ]] && continue
                
                # Extract the primary command (first word, handling leading whitespace)
                cmd=$(echo "$cmd_line" | sed 's/^[[:space:]]*//' | awk '{print $1}')
                
                # Skip if empty after processing
                [[ -z "$cmd" ]] && continue
                
                # Check afk commands
                if [[ "$cmd" == "afk" ]]; then
                    # Get the subcommand (second word)
                    subcmd=$(echo "$cmd_line" | sed 's/^[[:space:]]*//' | awk '{print $2}')
                    
                    # Handle flags that come before subcommand
                    if [[ "$subcmd" =~ ^- ]]; then
                        # It's a flag, skip validation (e.g., afk --help)
                        continue
                    fi
                    
                    if [[ -n "$subcmd" && ! "$subcmd" =~ ^[0-9]+$ ]]; then
                        # Check if it's a valid subcommand (not a numeric arg like "afk go 20")
                        if ! is_valid_subcommand "$subcmd"; then
                            echo -e "${RED}  ERROR: $file:$block_start: Invalid afk subcommand '$subcmd'${NC}"
                            echo "    Command: $cmd_line"
                            ((errors++)) || true
                        fi
                    fi
                fi
            done <<< "$block_content"
            
            continue
        fi
        
        # Accumulate content inside bash block
        if [[ $in_bash_block -eq 1 ]]; then
            block_content+="$line"$'\n'
        fi
    done < "$file"
}

# Find and validate all markdown files
docs_files=$(find "$ROOT_DIR" -name "*.md" -type f \
    ! -path "$ROOT_DIR/.git/*" \
    ! -path "$ROOT_DIR/target/*" \
    ! -path "$ROOT_DIR/examples/*" \
    | sort)

file_count=0
for file in $docs_files; do
    rel_path="${file#$ROOT_DIR/}"
    echo "Checking $rel_path..."
    validate_file "$file"
    ((file_count++)) || true
done

echo
echo "Summary:"
echo "  Files checked: $file_count"

if [[ $errors -gt 0 ]]; then
    echo -e "${RED}  Errors: $errors${NC}"
fi

if [[ $warnings -gt 0 ]]; then
    echo -e "${YELLOW}  Warnings: $warnings${NC}"
fi

echo

if [[ $errors -gt 0 ]]; then
    echo -e "${RED}FAILED: Documentation contains invalid code examples${NC}"
    exit 1
fi

echo -e "${GREEN}PASSED: All code examples are valid${NC}"
exit 0
