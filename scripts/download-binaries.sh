#!/bin/bash

# Attempt to identify the operating system
os_name=""
if [ "$(uname)" ]; then
    # Unix-like environment detected (Linux, macOS, etc.)
    os=$(uname -s)
    case "$os" in
        Linux*)     os_name="linux";;
        Darwin*)    os_name="darwin";;
        *)          os_name="unknown";;
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
        *arm64*) arch_type="arm64";;
        *amd64*) arch_type="amd64";;
        *)     arch_type="unknown";;
    esac
fi

if [[ "$arch_type" == "unknown" ]]; then
    echo "Unknown architecture. Exiting..."
    exit 1
fi

echo "Detected architecture: $arch_type"

package_urls=(
    "https://github.com/cometbft/cometbft/releases/download/v0.37.4/cometbft_0.37.4_${os_name}_${arch_type}.tar.gz"
    "replace_me_${os_name}_${arch_type}.tar.gz"
    "replace_me_${os_name}_${arch_type}.tar.gz"
    "replace_me_${os_name}_${arch_type}.tar.gz"
)
package_names=(
    "cometbft"
    "sequencer_###"
    "composer"_###
    "conductor_###"
)

mkdir bin
cd bin
# inedx=0
# Iterate through all URLs in the array
for ((index=0; index<${#package_urls[@]}; index++)); do
    filename="${package_names[$index]}"
    url="${package_urls[$index]}"
    
    # Check if the file exists
    if [ ! -f "$filename" ]; then
        echo "File $filename does not exist. Downloading from $url..."

        mkdir $filename
        cd $filename
        curl -L $url > ${filename}.tar.gz
        tar -xvzf ${filename}.tar.gz
        echo "Download of $filename completed."
        cd ..  
    else
        echo "File $filename already exists."
    fi
    # ((index++))
done

cd ..
