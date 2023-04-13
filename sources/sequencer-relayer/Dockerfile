# build stage
FROM --platform=$BUILDPLATFORM rust:1.68-slim as builder

# install deps needed to build our rust binary.
# libssl-dev and pkg-config are for ssl, protobuf-compiler is required by build.rs to build our protos
RUN apt-get update && \
    apt-get dist-upgrade -y && \
    apt-get install -y libssl-dev pkg-config protobuf-compiler

COPY . /app
WORKDIR /app

RUN cargo build --release

# Prod stage - remove build dependencies
FROM --platform=$BUILDPLATFORM gcr.io/distroless/cc
COPY --from=builder /app/target/release/relayer /app/target/release/relayer
CMD ["/app/target/release/relayer"]
