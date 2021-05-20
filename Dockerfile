# from https://www.lpalmieri.com/posts/fast-rust-docker-builds/
FROM rust:1.52 as planner
WORKDIR /app
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# To ensure a reproducible build consider pinning 
# the cargo-chef version with `--version X.X.X`
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef 
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust:1.52 as cacher
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:1.52 as builder
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo test --release --target x86_64-unknown-linux-musl