FROM --platform=$BUILDPLATFORM rust:1.61.0-buster AS build

ARG TARGETARCH

RUN apt-get update && apt-get install -y build-essential curl musl-tools upx pkg-config libssl-dev

WORKDIR /root/Flowly

ADD . .

RUN rustup install nightly && rustup default nightly && \
    case "$TARGETARCH" in \
    "amd64") \
        RUST_TARGET="x86_64-unknown-linux-musl" \
        MUSL="x86_64-linux-musl" \
        ;; \
    "arm64") \
        RUST_TARGET="aarch64-unknown-linux-musl" \
        MUSL="aarch64-linux-musl" \
        ;; \
    *) \
        echo "Doesn't support $TARGETARCH architecture" \
        exit 1 \
        ;; \
    esac && \
    wget -qO- "https://musl.cc/$MUSL-cross.tgz" | tar -xzC /root/ && \
    CC=/root/$MUSL-cross/bin/$MUSL-gcc && \
    rustup target add $RUST_TARGET && \
    PKG_CONFIG_ALLOW_CROSS=1 RUSTFLAGS="-C linker=$CC" CC=$CC cargo build --target "$RUST_TARGET" --release && \
    mv target/$RUST_TARGET/release/Flowly target/release/ && \
    upx -9 target/release/Flowly

FROM alpine:3.14 AS Flowly

COPY --from=build /root/Flowly/target/release/Flowly /usr/bin
ENTRYPOINT [ "Flowly" ]
