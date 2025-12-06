# Stage 1: Builder
# Use a specific Rust version with Debian slim for a stable build environment
FROM rust:1.91-slim-bullseye AS builder

# Set the working directory inside the container
WORKDIR /app

# Install musl-tools for static compilation
RUN apt-get update && apt-get install -y musl-tools pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
# Copy only Cargo.toml and Cargo.lock first to leverage Docker cache

# Add the musl target for static linking
RUN rustup target add x86_64-unknown-linux-musl

# Copy only Cargo.toml and Cargo.lock first to leverage Docker cache
# This layer changes less often than source code
COPY Cargo.toml Cargo.lock ./

# Build dependencies only. This layer is highly cacheable.
# If Cargo.toml and Cargo.lock haven't changed, this step will be skipped.
RUN cargo fetch --locked --target x86_64-unknown-linux-musl

# Copy all source code
COPY src ./src

# Build the release binary with musl target
# --release for optimizations and smaller size
# --locked to ensure reproducible builds based on Cargo.lock
# --target for static linking with musl libc
RUN CARGO_INCREMENTAL=0 \
    RUSTFLAGS="-C strip=debuginfo -C target-feature=+aes,+sse2,+ssse3" \
    cargo build --release --locked --target x86_64-unknown-linux-musl

# Stage 2: Runner
# Start from scratch for the smallest possible final image
FROM scratch

# Copy only the compiled binary from the builder stage
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/miou .

# Define the command to run your application
CMD ["./miou", "--config", "/config/miou.toml", "--data", "/data"]
