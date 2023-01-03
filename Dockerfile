FROM rust:1.66.0 AS builder
WORKDIR /tmp/
RUN USER=nobody cargo new --bin cmpdb
WORKDIR /tmp/cmpdb/
COPY Cargo.lock Cargo.toml ./
RUN cargo build --release --locked
RUN find target/release -type f -executable -maxdepth 1 -delete
COPY src/ src/
RUN touch src/main.rs
RUN cargo build --release --locked
RUN cargo install --locked --path .

FROM ubuntu:20.04
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install --no-install-recommends --yes \
	libssl1.1 \
	&& rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/cmpdb /usr/local/bin/cmpdb
USER nobody:nogroup
ENTRYPOINT ["cmpdb"]
