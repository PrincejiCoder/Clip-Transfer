FROM rust:slim AS build

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates tzdata && \
    rm -rf /var/lib/apt/lists/*

COPY . .

RUN RUSTFLAGS="-C target-cpu=native" cargo build --release && \
    strip target/release/linkdrop

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates tzdata && \
    rm -rf /var/lib/apt/lists/*

COPY --from=build /app/target/release/linkdrop /usr/local/bin/linkdrop

EXPOSE 8080

ENTRYPOINT ["linkdrop"]
