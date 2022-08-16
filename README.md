# Rumba

Rumba is [MDN's](https://developer.mozilla.org) new back-end. It supersedes [kuma](https://github.com/mdn/kuma) and
mainly powers [MDN Plus](https://developer.mozilla.org/en-US/plus).

## Quickstart

Before you can start working with Rumba, you need to:

1. Install [git](https://git-scm.com/) and [Rust](https://www.rust-lang.org/).
2. Install additional dependencies:
   - Mac OS (Intel): `brew install postgres libpq`
   - Mac OS (M1): `arch -arm64 brew install postgres libpq`
   - Ubuntu: `apt install gcc libpq-dev libssl-dev pkg-config`
3. Run a PostgreSQL instance:
   - Mac OS: e.g. [Postgres.app](https://postgresapp.com/)
   - Docker: `docker run --name postgres -p 5432:5432 -e POSTGRES_USER=rumba -e POSTGRES_PASSWORD=rumba -e POSTGRES_DB=mdn -d postgres`).
4. Copy `.settings.dev.toml` to `.settings.toml`.
5. Run `cargo run`.

## Testing

See [tests](./tests/)
