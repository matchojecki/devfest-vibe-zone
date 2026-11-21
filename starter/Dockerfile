FROM rust:1-alpine@sha256:8d0e82abfdf9a38071e07cabc5afba4ddbe077631a80e4e742a760c38beadc33 AS build
RUN apk add --no-cache musl-dev gcc
WORKDIR /build
ENV CARGO_BUILD_JOBS=1
COPY Cargo.toml Cargo.lock* ./
COPY src ./src
RUN --mount=type=cache,target=/build/target --mount=type=cache,target=/usr/local/cargo/registry \
    cargo build --release && mkdir -p /runtime/data && chown 10001:10001 /runtime/data \
    && cp target/release/devfest-rust /runtime/devfest-rust

FROM scratch
COPY --from=build /runtime/devfest-rust /devfest-rust
COPY --from=build --chown=10001:10001 /runtime/data /data
ENV PORT=8080 DATA_DIR=/data
USER 10001:10001
VOLUME ["/data"]
EXPOSE 8080
ENTRYPOINT ["/devfest-rust"]
