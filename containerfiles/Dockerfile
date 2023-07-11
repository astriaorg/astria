# build stage
FROM --platform=$BUILDPLATFORM lukemathwalker/cargo-chef:latest-rust-bookworm AS chef

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
      apt install -y librocksdb-dev:arm64; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
      rustup target add x86_64-unknown-linux-gnu; \
      if [ "$BUILDPLATFORM" != "linux/amd64" ]; then \
        dpkg --add-architecture amd64; \
        apt update && apt install -y gcc-x86_64-linux-gnu g++-x86_64-linux-gnu; \
      else \
        apt update; \
      fi; \
      apt install -y librocksdb-dev:amd64; \
    fi; \
    apt install -y libclang-dev clang protobuf-compiler ca-certificates;

# install buf cli
RUN BIN="/usr/local/bin" && \
    VERSION="1.17.0" && \
    curl -sSL \
    "https://github.com/bufbuild/buf/releases/download/v${VERSION}/buf-$(uname -s)-$(uname -m)" \
    -o "${BIN}/buf" && \
    chmod +x "${BIN}/buf"

# install targets
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /build/recipe.json recipe.json

ARG BUILDPLATFORM
ARG TARGETPLATFORM
ARG TARGETBINARY
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then export ROCKSDB_LIB_DIR=/usr/lib/aarch64-linux-gnu; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then export ROCKSDB_LIB_DIR=/usr/lib/x86_64-linux-gnu; fi; \
    if [ "$TARGETPLATFORM" = "linux/arm64" ] && [ "$BUILDPLATFORM" != "linux/arm64" ]; then \
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
        CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
        PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu; \
      cargo chef cook --release --target aarch64-unknown-linux-gnu --recipe-path recipe.json; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ] && [ "$BUILDPLATFORM" != "linux/amd64" ]; then \
      export CARGO_TARGET_x86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
        CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
        CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ \
        PKG_CONFIG_SYSROOT_DIR=/usr/x86_64-linux-gnu; \
      cargo chef cook --release --target x86_64-unknown-linux-gnu --recipe-path recipe.json; \
    else \
      cargo chef cook --release --target $(uname -m)-unknown-linux-gnu --recipe-path recipe.json; \
    fi;
    
COPY . .

RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then export ROCKSDB_LIB_DIR=/usr/lib/aarch64-linux-gnu; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then export ROCKSDB_LIB_DIR=/usr/lib/x86_64-linux-gnu; fi; \
    if [ "$TARGETPLATFORM" = "linux/arm64" ] && [ "$BUILDPLATFORM" != "linux/arm64" ]; then \
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
        CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ \
        PKG_CONFIG_SYSROOT_DIR=/usr/aarch64-linux-gnu; \
      cargo build --release --target aarch64-unknown-linux-gnu --bin $TARGETBINARY; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ] && [ "$BUILDPLATFORM" != "linux/amd64" ]; then \
      export CARGO_TARGET_x86_64_UNKNOWN_LINUX_GNU_LINKER=x86_64-linux-gnu-gcc \
        CC_x86_64_unknown_linux_gnu=x86_64-linux-gnu-gcc \
        CXX_x86_64_unknown_linux_gnu=x86_64-linux-gnu-g++ \
        PKG_CONFIG_SYSROOT_DIR=/usr/x86_64-linux-gnu; \
      cargo build --release --target x86_64-unknown-linux-gnu --bin $TARGETBINARY; \
    else \
      cargo build --release --target $(uname -m)-unknown-linux-gnu --bin $TARGETBINARY; \
    fi;

# replace this with `--out` or `--out-dir` once stable
RUN mkdir -p target/release
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then ARCH=aarch64; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then ARCH=x86_64; fi; \
    cp target/$ARCH-unknown-linux-gnu/release/$TARGETBINARY target/release/

FROM debian:bookworm-slim
ARG TARGETBINARY
WORKDIR /app/
EXPOSE 2450
COPY --from=chef /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /build/target/release/$TARGETBINARY /usr/local/bin/$TARGETBINARY
ENTRYPOINT ["/usr/local/bin/$TARGETBINARY"]
