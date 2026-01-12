#!/usr/bin/env bash
#
# afk installer for macOS and Linux
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/m0nkmaster/afk/main/scripts/install.sh | bash -s -- --beta
#
set -euo pipefail

# Configuration
REPO="m0nkmaster/afk"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="afk"

# Colours
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Colour

# Parse arguments
BETA=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --beta)
            BETA=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

info() {
    echo -e "${CYAN}$1${NC}"
}

success() {
    echo -e "${GREEN}$1${NC}"
}

warn() {
    echo -e "${YELLOW}$1${NC}"
}

error() {
    echo -e "${RED}$1${NC}"
    exit 1
}

# Detect OS
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *)       error "Unsupported OS: $os" ;;
    esac
}

# Detect architecture
detect_arch() {
    local arch os
    arch="$(uname -m)"
    os="$(uname -s)"
    
    case "$arch" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        arm64|aarch64)
            echo "arm64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac
}

# Get latest release tag from GitHub API
get_latest_release() {
    local url="https://api.github.com/repos/${REPO}/releases"
    
    if [[ "$BETA" == "true" ]]; then
        # Get latest release including pre-releases
        curl -fsSL "$url" | grep '"tag_name":' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
    else
        # Get latest stable release only
        curl -fsSL "${url}/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    fi
}

# Global temp dir for cleanup trap
TEMP_DIR=""

cleanup() {
    if [[ -n "$TEMP_DIR" && -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi
}
trap cleanup EXIT

# Download and verify binary
download_binary() {
    local version="$1"
    local os="$2"
    local arch="$3"
    local binary_name="afk-${os}-${arch}"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${binary_name}"
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/checksums.sha256"
    
    TEMP_DIR="$(mktemp -d)"
    
    info "Downloading afk ${version} for ${os}-${arch}..."
    
    # Download binary
    if ! curl -fsSL -o "${TEMP_DIR}/${binary_name}" "$download_url"; then
        error "Failed to download binary from ${download_url}"
    fi
    
    # Download checksums
    if ! curl -fsSL -o "${TEMP_DIR}/checksums.sha256" "$checksum_url"; then
        warn "Warning: Could not download checksums for verification"
    else
        # Verify checksum
        info "Verifying checksum..."
        cd "$TEMP_DIR"
        
        # Extract just the line for our binary
        grep "${binary_name}" checksums.sha256 > our_checksum.sha256
        
        if command -v sha256sum &> /dev/null; then
            if ! sha256sum -c our_checksum.sha256 --quiet; then
                error "Checksum verification failed!"
            fi
        elif command -v shasum &> /dev/null; then
            if ! shasum -a 256 -c our_checksum.sha256 --quiet; then
                error "Checksum verification failed!"
            fi
        else
            warn "Warning: No checksum utility found, skipping verification"
        fi
        
        cd - > /dev/null
    fi
    
    # Create install directory
    mkdir -p "$INSTALL_DIR"
    
    # Install binary
    info "Installing to ${INSTALL_DIR}/${BINARY_NAME}..."
    mv "${TEMP_DIR}/${binary_name}" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
}

# Setup shell completions
setup_completions() {
    local shell_name
    shell_name="$(basename "$SHELL")"
    
    case "$shell_name" in
        bash)
            local completion_dir="${HOME}/.local/share/bash-completion/completions"
            mkdir -p "$completion_dir"
            "${INSTALL_DIR}/${BINARY_NAME}" completions bash > "${completion_dir}/afk" 2>/dev/null || true
            ;;
        zsh)
            local completion_dir="${HOME}/.local/share/zsh/site-functions"
            mkdir -p "$completion_dir"
            "${INSTALL_DIR}/${BINARY_NAME}" completions zsh > "${completion_dir}/_afk" 2>/dev/null || true
            ;;
        fish)
            local completion_dir="${HOME}/.config/fish/completions"
            mkdir -p "$completion_dir"
            "${INSTALL_DIR}/${BINARY_NAME}" completions fish > "${completion_dir}/afk.fish" 2>/dev/null || true
            ;;
    esac
}

# Check if install directory is in PATH
check_path() {
    if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
        echo ""
        warn "Note: ${INSTALL_DIR} is not in your PATH"
        echo ""
        echo "Add it to your shell configuration:"
        
        local shell_name
        shell_name="$(basename "$SHELL")"
        
        case "$shell_name" in
            bash)
                echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
                echo "  source ~/.bashrc"
                ;;
            zsh)
                echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
                echo "  source ~/.zshrc"
                ;;
            fish)
                echo "  fish_add_path ~/.local/bin"
                ;;
            *)
                echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
                ;;
        esac
    fi
}

main() {
    echo ""
    info "afk installer"
    echo ""
    
    local os arch version
    os="$(detect_os)"
    arch="$(detect_arch)"
    
    info "Detected: ${os}-${arch}"
    
    version="$(get_latest_release)"
    if [[ -z "$version" ]]; then
        error "Could not determine latest version"
    fi
    
    if [[ "$BETA" == "true" ]]; then
        info "Channel: beta"
    fi
    
    download_binary "$version" "$os" "$arch"
    setup_completions
    
    echo ""
    success "✓ afk ${version} installed successfully!"
    
    check_path
    
    echo ""
    echo "Get started:"
    echo "  afk go            # Zero-config: auto-detect and run"
    echo "  afk --help        # Show all commands"
    echo ""
}

main
