FROM rust AS build

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./src ./src

RUN cargo build --release

FROM debian:stable-slim
COPY --from=build /target/release/websocket-redis-example .

EXPOSE 8080

CMD ["./websocket-redis-example"]
