#!/bin/bash

# donwload binaries for macos arm64
# https://github.com/cometbft/cometbft/releases/download/v0.37.3/cometbft_0.37.3_darwin_arm64.tar.gz
package_urls=(
    "https://github.com/cometbft/cometbft/releases/download/v0.37.4/cometbft_0.37.4_darwin_arm64.tar.gz"
    "https://github.com/astriaorg/astria/releases/download/sequencer-v0.9.0/astria-sequencer-aarch64-apple-darwin.tar.gz"
    "https://github.com/astriaorg/astria/releases/download/composer-v0.4.0/astria-composer-aarch64-apple-darwin.tar.gz"
    "https://github.com/astriaorg/astria/releases/download/conductor-v0.12.0/astria-conductor-aarch64-apple-darwin.tar.gz"
)
package_names=(
    "cometbft"
    "astria-sequencer"
    "astria-composer"
    "astria-conductor"
)

# mkdir bin
# cd bin
# Iterate through all URLs in the array
for ((index=0; index<${#package_urls[@]}; index++)); do
    filename="${package_names[$index]}"
    url="${package_urls[$index]}"
    
    # Check if the file exists
    if [ ! -f "$filename" ]; then
        echo "File $filename does not exist. Downloading from $url..."

        # mkdir $filename
        # cd $filename
        curl -L $url > ${filename}.tar.gz
        tar -xvzf ${filename}.tar.gz
        echo "Download of $filename completed."
        # cd ..  
    else
        echo "File $filename already exists."
    fi
done

# cd ..
