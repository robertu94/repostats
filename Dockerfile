from rust:1-alpine3.16 as builder
COPY Cargo.toml /build/
COPY src/ /build/src/
WORKDIR /build
ENV CARGO_HOME=/var/cache/cargo
RUN --mount=type=cache,id=apk,target=/var/cache/apk,sharing=locked apk add openssl-dev pkgconfig musl-dev sqlite-dev
RUN --mount=type=cache,id=cargobuild,target=/app/release/deps \
    --mount=type=cache,id=cargoindex,target=/var/cache/cargo \
  RUSTFLAGS=-Ctarget-feature=-crt-static cargo install --path . --target-dir /app

from alpine:3.16
RUN --mount=type=cache,id=apk,target=/var/cache/apk,sharing=locked apk add openssl sqlite-libs libgcc
COPY --from=builder /app/release/repostats /app/repostats
