FROM rust:1-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake clang libclang-dev git build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .
RUN cd server && cargo build --release
CMD ["/app/server/target/release/cfsolve-server"]
