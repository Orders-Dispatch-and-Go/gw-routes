FROM rust:1.90-slim AS builder
WORKDIR /build

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN mkdir src \
    && echo "fn main(){}" > src/main.rs \
    && echo "fn dummy(){}" > src/lib.rs \
    && cargo build --release 

COPY src src

RUN touch src/main.rs \
    && touch src/lib.rs \
    && cargo build --release


FROM ubuntu:24.04
WORKDIR /app

COPY --from=builder /build/target/release/gw-routes .

ENTRYPOINT [ "/app/gw-routes" ]
