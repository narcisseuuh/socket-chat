FROM debian:bullseye-slim AS builder

# Install Rust and other dependencies
RUN apt-get update && apt-get install -y \
    curl \
    build-essential \
    libsqlite3-dev \
    && curl https://sh.rustup.rs -sSf | sh -s -- -y \
    && . $HOME/.cargo/env

# Source the environment to make cargo available
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

WORKDIR /usr/src/app

# Install runtime dependencies
RUN apt-get update && apt-get install -y libsqlite3-0 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/rust-socket-chat .

EXPOSE 8080

# Run the application
CMD ["./rust-socket-chat"]