# Step 1: Build the application
FROM rust:1.52  as builder

WORKDIR app

# RUN apk add --update musl-dev
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl

COPY . .
RUN cargo test --target x86_64-unknown-linux-musl
RUN cargo test --release --target x86_64-unknown-linux-musl

# Step 2: Create the reusable builder container image
FROM rust:1.52

WORKDIR app

# RUN apk add --update musl-dev
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl

COPY --from=builder /app/target /app/target