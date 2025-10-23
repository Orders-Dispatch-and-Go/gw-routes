FROM gcc:15 AS cpp-builder
WORKDIR /build

RUN wget https://github.com/Kitware/CMake/releases/download/v4.1.2/cmake-4.1.2-linux-x86_64.sh \
    && chmod +x cmake-4.1.2-linux-x86_64.sh \
    && ./cmake-4.1.2-linux-x86_64.sh --skip-license --prefix=/usr/local \
    && rm cmake-4.1.2-linux-x86_64.sh 

COPY comparator /build

RUN cmake -S . -B build -DCMAKE_BUILD_TYPE=Release -DBUILD_TESTING=off \
    && cmake --build build


FROM rust:1.90-slim AS rust-builder
WORKDIR /build

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN mkdir src \
    && echo "fn main(){}" > src/main.rs \
    && echo "fn dummy(){}" > src/lib.rs \
    && cargo build --release 

COPY src src
COPY build.rs build.rs
COPY --from=cpp-builder /build/build/lib/libcomparatorlib.a libcomparatorlib.a

RUN touch src/main.rs \
    && touch src/lib.rs \
    && cargo build --release


FROM ubuntu:24.04
WORKDIR /app

COPY --from=rust-builder /build/target/release/gw-routes .

ENTRYPOINT [ "/app/gw-routes" ]
