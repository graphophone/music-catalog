FROM rust:1.97-alpine3.23 AS build

WORKDIR /app

COPY . .
RUN cargo build

FROM alpine:3.23

COPY --from=build /app/target/debug/music-catalog .

CMD ["./music-catalog"]