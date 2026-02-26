#!/usr/bin/env bash

set -e

OWNER="haiyewei"
REPO="tdlr"
LOCATION="/usr/local/bin"

echo_green() {
    echo -e "\033[32m$1\033[0m"
}
echo_red() {
    echo -e "\033[31m$1\033[0m"
}
echo_blue() {
    echo -e "\033[34m$1\033[0m"
}

PROXY=""
VERSION=""

# flags: --proxy --version
while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
        --proxy)
            PROXY="https://mirror.ghproxy.com/"
            echo_blue "Using GitHub proxy: $PROXY"
            shift
            ;;
        --version)
            VERSION="$2"
            shift
            shift
            ;;
        *)
            echo "Unknown flag: $key"
            exit 1
            ;;
    esac
done

# Set OS based on system
case $(uname -s) in
    Linux)
        OS="Linux"
        ;;
    Darwin)
        OS="MacOS"
        ;;
    *)
        echo_red "Unsupported OS: $(uname -s)"
        exit 1
        ;;
esac

# Set download ARCH based on system architecture
case $(uname -m) in
    x86_64)
        ARCH="64bit"
        ;;
    arm64|aarch64*)
        ARCH="arm64"
        ;;
    *)
        echo_red "Unsupported architecture: $(uname -m)"
        exit 1
        ;;
esac

# get latest version
if [ -z "$VERSION" ]; then
    echo_blue "Fetching latest version..."
    VERSION=$(curl --silent "https://api.github.com/repos/$OWNER/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    if [ -z "$VERSION" ]; then
        echo_red "Failed to fetch latest version"
        exit 1
    fi
fi
echo_blue "Target version: $VERSION"

# build download URL
URL=${PROXY}https://github.com/$OWNER/$REPO/releases/download/$VERSION/${REPO}_${OS}_$ARCH.tar.gz
echo_blue "Downloading $REPO from $URL"

# Create temporary directory for extraction
TMP_DIR=$(mktemp -d)
trap 'rm -rf "$TMP_DIR"' EXIT

# download and extract
if command -v wget >/dev/null 2>&1; then
    wget -q --show-progress -O "$TMP_DIR/package.tar.gz" "$URL"
elif command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$TMP_DIR/package.tar.gz" "$URL"
else
    echo_red "Neither wget nor curl found. Please install one of them."
    exit 1
fi

tar -xzf "$TMP_DIR/package.tar.gz" -C "$TMP_DIR"

if [ ! -f "$TMP_DIR/$REPO" ]; then
    echo_red "Binary $REPO not found in the downloaded package."
    exit 1
fi

echo_blue "Installing to $LOCATION..."

# Move to location, use sudo if necessary
if [ -w "$LOCATION" ]; then
    mv "$TMP_DIR/$REPO" "$LOCATION/$REPO"
    chmod +x "$LOCATION/$REPO"
else
    echo_blue "Requires root privileges to install to $LOCATION. Prompting for sudo..."
    sudo mv "$TMP_DIR/$REPO" "$LOCATION/$REPO"
    sudo chmod +x "$LOCATION/$REPO"
fi

echo_green "$REPO installed successfully! Location: $LOCATION/$REPO"
echo_green "Run '$REPO --help' to get started"

