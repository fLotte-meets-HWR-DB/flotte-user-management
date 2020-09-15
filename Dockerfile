# Build Stage
FROM rust:alpine AS builder
WORKDIR /usr/src/
RUN apk add --no-cache libgcc musl-dev
RUN rustup target add x86_64-unknown-linux-musl

RUN USER=root cargo new flotte-user-management
WORKDIR /usr/src/flotte-user-management
COPY Cargo.toml Cargo.lock ./

COPY msg-rpc ./msg-rpc
RUN cargo build --release
COPY src ./src
RUN cargo install --target x86_64-unknown-linux-musl --path .

# Bundle Stage
FROM scratch
COPY --from=builder /usr/local/cargo/bin/flotte-user-management .

ENTRYPOINT ["./flotte-user-management"]