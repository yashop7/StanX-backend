# Build stage
FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo build --release --workspace

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/backend ./target/release/backend
COPY --from=builder /app/target/release/ws ./target/release/ws
COPY --from=builder /app/target/release/event-listener ./target/release/event-listener

CMD ["./target/release/backend"]
