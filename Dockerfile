FROM rust:alpine
COPY . .
RUN apk add --no-cache libgcc musl-dev
RUN cargo install --path .
EXPOSE 8080
EXPOSE 5000
ENTRYPOINT ["/usr/local/cargo/bin/flotte-user-management"]