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

# Check if cargo is installed
if ! command -v cargo &> /dev/null
then
    echo_red "cargo could not be found. Please install Rust first."
    exit 1
fi

echo_blue "Compiling $REPO..."
cargo build --release

EXECUTABLE="target/release/${REPO}"

if [ ! -f "$EXECUTABLE" ]; then
    echo_red "Executable not found at $EXECUTABLE. Compilation might have failed."
    exit 1
fi

echo_blue "Installing $REPO to $LOCATION..."

# Attempt to install, use sudo if necessary
if [ -w "$LOCATION" ]; then
    cp "$EXECUTABLE" "$LOCATION/$REPO"
    chmod +x "$LOCATION/$REPO"
    echo_green "$REPO compiled and installed successfully! Location: $LOCATION/$REPO"
    echo_green "Run '$REPO --help' to get started"
else
    echo_blue "Requires root privileges to install to $LOCATION. Prompting for sudo..."
    # Disable exit on error temporarily to handle sudo failure
    set +e
    sudo cp "$EXECUTABLE" "$LOCATION/$REPO" 2>/dev/null
    SUDO_CP_STATUS=$?
    sudo chmod +x "$LOCATION/$REPO" 2>/dev/null
    set -e

    if [ $SUDO_CP_STATUS -eq 0 ]; then
        echo_green "$REPO compiled and installed successfully! Location: $LOCATION/$REPO"
        echo_green "Run '$REPO --help' to get started"
    else
        echo_red "sudo failed or is restricted. Falling back to local user installation..."
        
        # Use ~/.cargo/bin if it exists (since cargo is installed), otherwise ~/.local/bin
        if [ -d "$HOME/.cargo/bin" ]; then
            FALLBACK_LOCATION="$HOME/.cargo/bin"
        else
            FALLBACK_LOCATION="$HOME/.local/bin"
        fi
        
        echo_blue "Installing $REPO to $FALLBACK_LOCATION..."
        mkdir -p "$FALLBACK_LOCATION"
        cp "$EXECUTABLE" "$FALLBACK_LOCATION/$REPO"
        chmod +x "$FALLBACK_LOCATION/$REPO"
        
        echo_green "$REPO compiled and installed successfully! Location: $FALLBACK_LOCATION/$REPO"
        echo_green "Run '$REPO --help' to get started"
    fi
fi
