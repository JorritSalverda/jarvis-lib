FROM rust:1.52 as builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends musl-tools
RUN rustup target add x86_64-unknown-linux-musl
COPY . .
# add following 2 lines after initial build to speed up next builds
COPY --from=jsalverda/jarvis-lib:dlc /app/target target
COPY --from=jsalverda/jarvis-lib:dlc /usr/local/cargo /usr/local/cargo
RUN cargo test --release --target x86_64-unknown-linux-musl
