# build stage
FROM --platform=$BUILDPLATFORM lukemathwalker/cargo-chef:latest-rust-bullseye AS chef
WORKDIR /build/

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
    libprotobuf-dev \
    protobuf-compiler \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# install buf cli
RUN BIN="/usr/local/bin" && \
    VERSION="1.17.0" && \
    curl -sSL \
    "https://github.com/bufbuild/buf/releases/download/v${VERSION}/buf-$(uname -s)-$(uname -m)" \
    -o "${BIN}/buf" && \
    chmod +x "${BIN}/buf"

# install zig
RUN curl -L "https://ziglang.org/download/0.10.1/zig-linux-$(uname -m)-0.10.1.tar.xz" | tar -J -x -C /usr/local && \
    ln -s "/usr/local/zig-linux-$(uname -m)-0.10.1/zig" /usr/local/bin/zig

# install zigbuild
RUN cargo install --locked cargo-zigbuild

# install targets
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /build/recipe.json recipe.json

ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    "linux/arm64") target="aarch64-unknown-linux-gnu" ;; \
    "linux/amd64") target="x86_64-unknown-linux-gnu" ;; \
    esac \
    && printf "$target" > target_triple \
    && rustup target add "$target" \
    && cargo chef cook --zigbuild \
    --release \
    --target "$target" \
    --recipe-path recipe.json
COPY . .
RUN cargo zigbuild --release \
    --target $(cat ./target_triple) \
    --bin conductor
# replace this with `--out` or `--out-dir` once stable
RUN mkdir -p target/release \
    && cp target/$(cat ./target_triple)/release/conductor target/release/

# TODO - remove debug tag once this is stable
FROM gcr.io/distroless/cc:debug
WORKDIR /app/
EXPOSE 2450
COPY --from=builder /build/target/release/conductor /usr/local/bin/conductor
ENTRYPOINT ["/usr/local/bin/conductor"]
