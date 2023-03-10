# Rumba

Rumba is [MDN's](https://developer.mozilla.org) new back-end. It supersedes [kuma](https://github.com/mdn/kuma) and
mainly powers [MDN Plus](https://developer.mozilla.org/en-US/plus).

## Quickstart

Before you can start working with Rumba, you need to:

1. Install [git](https://git-scm.com/) and [Rust](https://www.rust-lang.org/).
2. Install additional dependencies:
   - Mac OS `brew install libpq && brew link --force libpq`
   - Ubuntu: `apt install gcc libpq-dev libssl-dev pkg-config`
3. Run a PostgreSQL instance:
   - Mac OS: e.g. [Postgres.app](https://postgresapp.com/)
   - Docker: `docker run --name postgres -p 5432:5432 -e POSTGRES_USER=rumba -e POSTGRES_PASSWORD=rumba -e POSTGRES_DB=mdn -d postgres`).
4. Run an Elastic instance:
   - Docker: `docker run --name elastic -p 9200:9200 -p 9300:9300 -e "discovery.type=single-node" -d elasticsearch:8.3.3`
5. Copy `.settings.dev.toml` to `.settings.toml`.
6. Run `cargo run`.
7. To create an authenticated session navigate to http://localhost:8000/users/fxa/login/authenticate/?next=%2F and login with your firefox staging account
8. To check you are logged in and ready to go navigate to http://localhost:8000/api/v1/whoami you should see your logged in user information.

## Testing

See [tests](./tests/)
