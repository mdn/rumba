FROM rust:bookworm

WORKDIR /usr/src/rumba

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

RUN mkdir .cargo

RUN cargo vendor > .cargo/config

COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libpq5 ca-certificates \
 && rm -rf /var/lib/apt/lists/*

RUN groupadd rumba && useradd -g rumba rumba

WORKDIR /app/
COPY --from=0 /usr/src/rumba/target/release/rumba .
RUN chown -R rumba:rumba /app
USER rumba

CMD ["./rumba"]
