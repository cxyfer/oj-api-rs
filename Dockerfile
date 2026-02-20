FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src
COPY src/ src/
COPY templates/ templates/
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 python3-pip ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/oj-api-rs /usr/local/bin/
COPY templates/ /app/templates/
COPY static/ /app/static/
COPY references/ /app/references/
WORKDIR /app
EXPOSE 3000
CMD ["oj-api-rs"]
