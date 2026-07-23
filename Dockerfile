FROM rust:1.97-alpine3.23

WORKDIR /app

COPY Cargo.lock Cargo.toml .
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build
RUN rm -rf src

COPY src ./src
RUN cargo build

CMD ["./target/debug/music-catalog"]