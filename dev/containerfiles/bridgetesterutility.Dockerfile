FROM ubuntu:22.04

# This is a utility image for testing the bridge.
# It contains the Celestia app and the Astria CLI, plus some bash utilities for testing.

# dependencies needed for testing
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    bc \
    jq \
    sed \
    ca-certificates \
    coreutils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /home

ARG CELESTIA_VERSION=v1.9.0
ARG ASTRIA_VERSION=nightly-2024-07-10

# download architecture-specific binaries
ARG TARGETPLATFORM
RUN echo "TARGETPLATFORM: $TARGETPLATFORM"
RUN if [ "$TARGETPLATFORM" = "darwin/arm64" ]; then \
        curl -L "https://github.com/celestiaorg/celestia-app/releases/download/$CELESTIA_VERSION/celestia-app_Darwin_arm64.tar.gz" -o celestia-appd.tar.gz; \
        curl -L "https://github.com/astriaorg/astria-cli-go/releases/download/$ASTRIA_VERSION/astria-go-$ASTRIA_VERSION-darwin-arm64.tar.gz" -o astria-go.tar.gz; \
    elif [ "$TARGETPLATFORM" = "darwin/amd64" ]; then \
        curl -L "https://github.com/celestiaorg/celestia-app/releases/download/$CELESTIA_VERSION/celestia-app_Darwin_x86_64.tar.gz" -o celestia-appd.tar.gz; \
        curl -L "https://github.com/astriaorg/astria-cli-go/releases/download/$ASTRIA_VERSION/astria-go-$ASTRIA_VERSION-darwin-amd64.tar.gz" -o astria-go.tar.gz; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
        curl -L "https://github.com/celestiaorg/celestia-app/releases/download/$CELESTIA_VERSION/celestia-app_Linux_x86_64.tar.gz" -o celestia-appd.tar.gz; \
        curl -L "https://github.com/astriaorg/astria-cli-go/releases/download/$ASTRIA_VERSION/astria-go-$ASTRIA_VERSION-linux-amd64.tar.gz" -o astria-go.tar.gz; \
    else \
        echo "Unsupported architecture"; \
        echo "TARGETPLATFORM: $TARGETPLATFORM"; \
        exit 1; \
    fi

# untar and move to bin
RUN tar -xzvf celestia-appd.tar.gz && mv celestia-appd /usr/local/bin/celestia-appd && \
    tar -xzvf astria-go.tar.gz && mv astria-go /usr/local/bin/astria-go && \
    chmod +x /usr/local/bin/celestia-appd /usr/local/bin/astria-go

CMD ["echo", "This is the bridge tester utility image!"]
