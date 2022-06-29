FROM rust:latest

WORKDIR /usr/src/rumba

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN mkdir .cargo

RUN cargo vendor > .cargo/config

COPY . .
RUN cargo build --release

FROM debian:10-slim

RUN apt-get update && apt-get install -y \
    libpq5 ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /root/
COPY --from=0 /usr/src/rumba/target/release/rumba .
COPY --from=0 /usr/src/rumba/.settings.toml .

CMD ["./rumba"]  