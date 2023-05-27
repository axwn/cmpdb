FROM rust:1.69.0 AS builder
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

FROM redhat/ubi9-minimal
COPY --from=builder /usr/local/cargo/bin/cmpdb /usr/local/bin/cmpdb
USER nobody:nobody
ENTRYPOINT ["cmpdb"]
