# Stage 1: Builder
# Use a specific Rust version with alpine
FROM rust:1.91-alpine AS chef
RUN cargo install cargo-chef

# Set the working directory inside the container
WORKDIR /app

# Stage 1.1: Planner
# Create a lightweight planner stage to prepare the build
FROM chef AS planner
COPY src ./src
COPY assets ./assets
COPY Cargo.toml Cargo.lock ./
RUN cargo chef prepare --bin miou --recipe-path recipe.json

# Stage 1.2: Builder
# Build the actual application
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Install musl-tools, openssl and certificates
RUN set -x && \
    apk update && apk add --no-cache musl-dev openssl-dev openssl-libs-static ca-certificates

# statically link against openssl
ENV OPENSSL_STATIC=1

# Add the musl target for static linking
RUN rustup target add $( sh -c 'uname -m' )-unknown-linux-musl

# Build dependencies only to leverage Docker cache
RUN cargo chef cook --release --locked --target $( sh -c 'uname -m' )-unknown-linux-musl --bin miou --recipe-path recipe.json

COPY src ./src
COPY assets ./assets
COPY Cargo.toml Cargo.lock ./

# Build the release binary with musl target
# --release for optimizations and smaller size
# --locked to ensure reproducible builds based on Cargo.lock
# --target for static linking with musl libc
RUN cargo build --release --locked --target $( sh -c 'uname -m' )-unknown-linux-musl --bin miou

# Copy because the final stage does not know the target
RUN mkdir /app/target/final && cp /app/target/$( sh -c 'uname -m' )-unknown-linux-musl/release/miou /app/target/final/miou

# Stage 2: Runner
# Start from scratch for the smallest possible final image
FROM scratch

# Copy only the compiled binary from the builder stage
COPY --from=builder /app/target/final/miou .

# Copy CA certificates for SSL/TLS support
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

# Copy the default configuration file into the container
COPY ./miou.docker.yml /config/miou.yml

# Define the command to run your application
CMD ["./miou", "--config", "/config/miou.yml", "--data", "/data"]
