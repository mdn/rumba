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

## Database migrations

Rumba uses [Diesel](https://diesel.rs/) to manage the PostgreSQL schema. Migrations live in [`migrations/`](./migrations/) and run automatically on startup (unless `skip_migrations = true` is set in `.settings.toml`).

To work with migrations manually, install the Diesel CLI with PostgreSQL support only:

```sh
cargo install diesel_cli --no-default-features --features postgres
```

The CLI reads the connection string from `DATABASE_URL` (or `--database-url`), e.g. the `db.uri` from your `.settings.toml`:

```sh
export DATABASE_URL=postgres://rumba:rumba@127.0.0.1:5432/mdn
```

Common commands:

```sh
# Generate a new migration (creates `migrations/<timestamp>_<name>/{up,down}.sql`)
diesel migration generate <name>

# Run all pending migrations (also regenerates `src/db/schema.rs`, see `diesel.toml`)
diesel migration run

# Roll back the latest migration
diesel migration revert

# Verify that the latest migration can be rolled back and reapplied
diesel migration redo
```

Do not edit `src/db/schema.rs` by hand. Commit the regenerated file together with the migration.

## Formatting & Linting

All changes to Rumba are required to be formatted with [Rustfmt](https://doc.rust-lang.org/stable/clippy/index.html) (`cargo fmt --all`) and free of [Clippy](https://doc.rust-lang.org/stable/clippy/index.html) linting errors or warnings (`cargo clippy --all --all-features -- -D warnings`).

To avoid committing unformatted or unlinted changes, we recommend setting up a pre-commit [Git hook](https://git-scm.com/book/en/v2/Customizing-Git-Git-Hooks) in your local repository checkout:

```sh
touch .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
cat <<EOF >> .git/hooks/pre-commit
#!/usr/bin/env bash

echo "Running cargo fmt..."
cargo fmt --all -- --check

echo "Running cargo clippy..."
cargo clippy --all --all-features -- -D warnings
EOF
```

## Testing

See [tests](./tests/)
