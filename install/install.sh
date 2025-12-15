#!/usr/bin/env bash

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

# Check if script is run as root
if [[ $EUID -ne 0 ]]; then
   echo_red "This script must be run as root"
   exit 1
fi

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
    VERSION=$(curl --silent "https://api.github.com/repos/$OWNER/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
fi
echo_blue "Target version: $VERSION"

# build download URL
URL=${PROXY}https://github.com/$OWNER/$REPO/releases/download/$VERSION/${REPO}_${OS}_$ARCH.tar.gz
echo_blue "Downloading $REPO from $URL"

# download and extract
wget -q --show-progress -O - "$URL" | tar -xz && \
  mv $REPO $LOCATION/$REPO && \
  chmod +x $LOCATION/$REPO && \
  echo_green "$REPO installed successfully! Location: $LOCATION/$REPO" && \
  echo_green "Run '$REPO --help' to get started"
