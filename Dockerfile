# from https://www.lpalmieri.com/posts/fast-rust-docker-builds/
FROM rust:1.52 as planner
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef --version 0.1.20
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust:1.52 as cacher
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install cargo-chef --version 0.1.20
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

FROM rust:1.52 as builder
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /app
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN cargo test --release --target x86_64-unknown-linux-musl