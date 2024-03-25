FROM --platform=$BUILDPLATFORM rust:1.76-bookworm AS rust

WORKDIR /build/

ARG TARGETPLATFORM
ARG BUILDPLATFORM
ARG TARGETBINARY
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
      rustup target add aarch64-unknown-linux-gnu; \
      if [ "$BUILDPLATFORM" != "linux/arm64" ]; then \
        dpkg --add-architecture arm64; \
        apt update && apt install -y gcc-aarch64-linux-gnu g++-aarch64-linux-gnu; \
      else \
        apt update; \
      fi; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
      rustup target add x86_64-unknown-linux-gnu; \
      if [ "$BUILDPLATFORM" != "linux/amd64" ]; then \
        dpkg --add-architecture amd64; \
        apt update && apt install -y gcc-x86_64-linux-gnu g++-x86_64-linux-gnu; \
      else \
        apt update; \
      fi; \
    fi; \
    apt install -y libclang-dev clang;
RUN \
  PROTOC_VERSION=24.4; \
  if [ "$BUILDPLATFORM" = "linux/amd64" ]; then \
    PROTOC_PLATFORM="linux-x86_64"; \
  elif [ "$BUILDPLATFORM" = "linux/arm64" ]; then \
    PROTOC_PLATFORM="linux-aarch_64"; \
  fi; \
  PROTOC_PKG=protoc-"$PROTOC_VERSION"-"$PROTOC_PLATFORM".zip; \
  PROTOC_SRC="https://github.com/protocolbuffers/protobuf/releases/download/v"$PROTOC_VERSION"/$PROTOC_PKG"; \
  curl -OL "$PROTOC_SRC"; \
  unzip "$PROTOC_PKG" -d ./protoc/; \
  install -d /usr/local/bin /usr/local/include/google/protobuf/compiler/; \
  install ./protoc/bin/* -Dt /usr/local/bin; \
  install ./protoc/include/google/protobuf/* -Dt /usr/local/include/google/protobuf; \
  install ./protoc/include/google/protobuf/compiler/* -Dt /usr/local/include/google/protobuf/compiler;

FROM rust as builder

ARG BUILDPLATFORM
ARG TARGETPLATFORM
ARG TARGETBINARY
COPY . .

RUN mkdir -p release
RUN \
    --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    if [ "$TARGETPLATFORM" = "linux/arm64" ] && [ "$BUILDPLATFORM" != "linux/arm64" ]; then \
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
        CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
        PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu; \
      TARGET_TRIPLE=aarch64-unknown-linux-gnu; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ] && [ "$BUILDPLATFORM" != "linux/amd64" ]; then \
      export CARGO_TARGET_x86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
        CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
        CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ \
        PKG_CONFIG_SYSROOT_DIR=/usr/x86_64-linux-gnu; \
      TARGET_TRIPLE=x86_64-unknown-linux-gnu; \
    else \
      TARGET_TRIPLE=$(uname -m)-unknown-linux-gnu; \
    fi; \
    export PROTOC=/usr/local/bin/protoc; \
    cargo build --release --target $TARGET_TRIPLE --bin $TARGETBINARY;\
    # Copies the binary from out of the cache directory
    if [ "$TARGETPLATFORM" = "linux/arm64" ]; then ARCH=aarch64; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then ARCH=x86_64; fi; \
    cp target/$ARCH-unknown-linux-gnu/release/$TARGETBINARY release/;

FROM debian:bookworm-slim
ARG TARGETBINARY
ARG TARGETPLATFORM
ARG BUILDPLATFORM
RUN \
    apt update; \
    apt install -y wget ca-certificates; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*;
COPY --from=builder /build/release/$TARGETBINARY /usr/local/bin/$TARGETBINARY
RUN ln -s /usr/local/bin/$TARGETBINARY /usr/local/bin/entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint"]
