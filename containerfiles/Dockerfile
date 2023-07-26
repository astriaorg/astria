# build stage
FROM --platform=$BUILDPLATFORM lukemathwalker/cargo-chef:0.1.61-rust-bookworm AS chef

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
    apt install -y libclang-dev clang;

# install targets
FROM chef AS planner
ARG TARGETBINARY
COPY . .
RUN cargo chef prepare --bin crates/$TARGETBINARY --recipe-path recipe.json

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
    cargo chef cook --release --bin $TARGETBINARY --target $TARGET_TRIPLE --recipe-path recipe.json;
    
COPY . .

RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then export ROCKSDB_LIB_DIR=/usr/lib/aarch64-linux-gnu; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then export ROCKSDB_LIB_DIR=/usr/lib/x86_64-linux-gnu; fi; \
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
    cargo build --release --target $TARGET_TRIPLE --bin $TARGETBINARY;

# replace this with `--out` or `--out-dir` once stable
RUN mkdir -p target/release
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then ARCH=aarch64; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then ARCH=x86_64; fi; \
    cp target/$ARCH-unknown-linux-gnu/release/$TARGETBINARY target/release/

FROM debian:bookworm-slim
ARG TARGETBINARY
ARG TARGETPLATFORM
ARG BUILDPLATFORM
RUN if [ "$TARGETPLATFORM" = "linux/arm64" ]; then \
      if [ "$BUILDPLATFORM" != "linux/arm64" ]; then \
        dpkg --add-architecture arm64; \
      fi; \
      apt update; \
      apt install -y librocksdb-dev:arm64; \
    elif [ "$TARGETPLATFORM" = "linux/amd64" ]; then \
      if [ "$BUILDPLATFORM" != "linux/amd64" ]; then \
        dpkg --add-architecture amd64; \
      fi; \
      apt update; \
      apt install -y librocksdb-dev:amd64; \
    fi; \
    apt install -y wget ca-certificates; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*;
COPY --from=builder /build/target/release/$TARGETBINARY /usr/local/bin/$TARGETBINARY
RUN ln -s /usr/local/bin/$TARGETBINARY /usr/local/bin/entrypoint
ENTRYPOINT ["/usr/local/bin/entrypoint"]
