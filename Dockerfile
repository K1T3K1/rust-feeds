FROM rust:1.85 AS builder

WORKDIR /usr/src/rust-feeds

COPY . .

RUN cargo install --path .

CMD ["rust-feeds"]
