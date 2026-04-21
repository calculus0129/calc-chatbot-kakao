FROM rust:1-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/calc-chatbot-server /usr/local/bin/app
ENV PORT=8080
EXPOSE 8080
CMD ["/usr/local/bin/app"]
