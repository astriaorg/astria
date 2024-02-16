#!/bin/bash

# Attempt to identify the operating system
os_name=""
if [ "$(uname)" ]; then
    # Unix-like environment detected (Linux, macOS, etc.)
    os=$(uname -s)
    case "$os" in
        Linux*)  os_name="linux"   ;;
        Darwin*) os_name="darwin"  ;;
        *)       os_name="unknown" ;;
    esac
fi

if [[ "$os_name" == "unknown" ]]; then
    echo "Unknown operating system. Exiting..."
    exit 1
fi

echo "Detected OS: $os_name"

# Attempt to identify the architecture
arch_type=""
if [ "$(uname)" ]; then
    # Unix-like environment detected (Linux, macOS, etc.)
    arch=$(uname -a)
    case "$arch" in
        *arm64*)   arch_type="arm64"   ;;
        *aarch64*) arch_type="aarch64" ;;
        *amd64*)   arch_type="amd64"   ;;
        *)         arch_type="unknown" ;;
    esac
fi

if [[ "$arch_type" == "unknown" ]]; then
    echo "Unknown architecture. Exiting..."
    exit 1
fi

echo "Detected architecture: $arch_type"
