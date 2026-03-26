#!/bin/bash
# Build script for Android targets
# Requires: Android NDK, Rust targets installed

set -e

NDK_PATH="${ANDROID_NDK_HOME:-}"
TARGET="${1:-all}"

if [ -z "$NDK_PATH" ]; then
    echo "Error: ANDROID_NDK_HOME not set"
    echo "Example: export ANDROID_NDK_HOME=/path/to/android-ndk-r26b"
    exit 1
fi

# Detect host OS
case "$(uname -s)" in
    Linux*)  HOST_TAG="linux-x86_64";;
    Darwin*) HOST_TAG="darwin-x86_64";;
    *)       echo "Unsupported OS"; exit 1;;
esac

TOOLCHAIN_BIN="$NDK_PATH/toolchains/llvm/prebuilt/$HOST_TAG/bin"
if [ ! -d "$TOOLCHAIN_BIN" ]; then
    echo "Error: NDK toolchain not found at: $TOOLCHAIN_BIN"
    exit 1
fi

export PATH="$TOOLCHAIN_BIN:$PATH"

# Targets to build
TARGETS=(
    "aarch64-linux-android"    # arm64-v8a
    "armv7-linux-androideabi"  # armeabi-v7a
    "x86_64-linux-android"     # x86_64
    "i686-linux-android"       # x86
)

if [ "$TARGET" != "all" ]; then
    TARGETS=("$TARGET")
fi

# Install targets
for t in "${TARGETS[@]}"; do
    echo "Checking target: $t"
    rustup target add "$t" 2>/dev/null || true
done

# Build each target
for t in "${TARGETS[@]}"; do
    echo ""
    echo "=== Building for $t ==="
    cargo build --release --target "$t" --features android
done

# Copy outputs to jniLibs structure
OUTPUT_DIR="target/android/jniLibs"
mkdir -p "$OUTPUT_DIR"

declare -A ABI_MAP=(
    ["aarch64-linux-android"]="arm64-v8a"
    ["armv7-linux-androideabi"]="armeabi-v7a"
    ["x86_64-linux-android"]="x86_64"
    ["i686-linux-android"]="x86"
)

for t in "${TARGETS[@]}"; do
    ABI="${ABI_MAP[$t]}"
    SRC_LIB="target/$t/release/libtdlr_core.so"
    DST_DIR="$OUTPUT_DIR/$ABI"
    
    if [ -f "$SRC_LIB" ]; then
        mkdir -p "$DST_DIR"
        cp "$SRC_LIB" "$DST_DIR/libtdlr.so"
        echo "Copied: $DST_DIR/libtdlr.so"
    fi
done

echo ""
echo "Build complete! Libraries are in: $OUTPUT_DIR"
