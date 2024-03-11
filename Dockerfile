# Use the official Rust image as a builder
FROM rust:bookworm as builder

# Create a new empty shell project
RUN USER=root cargo new --bin long_running_db_notification
WORKDIR /long_running_db_notification

# Copy the Cargo.toml and Cargo.lock files and your source code
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src

# Build your application for release
RUN cargo build --release

# Use the Debian  image for the runtime environment
FROM debian:bookworm-slim

RUN apt-get update && apt install -y openssl ca-certificates wget && \
    rm -rf /var/lib/apt/lists/*

RUN wget https://github.com/Yelp/dumb-init/releases/download/v1.2.5/dumb-init_1.2.5_amd64.deb
RUN dpkg -i dumb-init_*.deb

# Copy the binary from the builder stage
COPY --from=builder /long_running_db_notification/target/release/long_running_db_notification .

ENTRYPOINT ["dumb-init", "--"]

CMD ["./long_running_db_notification"]
