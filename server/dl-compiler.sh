#!/usr/bin/env bash
# Obtain a copy of the Zirco compiler.
# This uses files from the Zirco tarball to prepare a Zircon sysroot.

set -e

# Function to detect platform and architecture
detect_platform_arch() {
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    arch=$(uname -m)
    
    case "$os" in
        linux*)
            platform="linux"
            ;;
        darwin*)
            platform="macos"
            ;;
        *)
            echo "Unsupported platform: $os"
            exit 1
            ;;
    esac
    
    case "$arch" in
        x86_64)
            architecture="x64"
            ;;
        aarch64|arm64)
            architecture="arm64"
            ;;
        *)
            echo "Unsupported architecture: $arch"
            exit 1
            ;;
    esac
    
    echo "${platform}-${architecture}"
}

platform_arch=$(detect_platform_arch)
zircon_filename="zrc-$platform_arch.tar.gz"
zircon_url="https://github.com/zirco-lang/zrc/releases/download/nightly/$zircon_filename"
source_url="https://github.com/zirco-lang/zrc/archive/refs/tags/nightly.zip"

rm -rf zrc-nightly
mkdir zrc-nightly
cd zrc-nightly

echo "Downloading zircon package from $zircon_url..."
curl -fSL -o "$zircon_filename" "$zircon_url"
tar -xzf "$zircon_filename"
rm "$zircon_filename"
