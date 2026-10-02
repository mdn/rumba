# Rumba

Rumba is [MDN's](https://developer.mozilla.org) new back-end. It supersedes [kuma](https://github.com/mdn/kuma) and
mainly powers [MDN Plus](https://developer.mozilla.org/en-US/plus).

## Quickstart

Before you can start working with Rumba, you need to:

1. Install [git](https://git-scm.com/) and [Rust](https://www.rust-lang.org/).
2. Install additional dependencies:
   - Mac OS `brew install libpq && brew link --force libpq`
   - Ubuntu: `apt install gcc libpq-dev libssl-dev pkg-config`
3. Run a [PostgreSQL](#postgresql) instance.
4. Run an [Elasticsearch](#elasticsearch) instance, and [index](#search-index) some documents.
5. Copy `.settings.dev.toml` to `.settings.toml`.
6. Run `cargo run`.
7. To create an authenticated session navigate to http://localhost:8000/users/fxa/login/authenticate/?next=%2F and login with your firefox staging account
8. To check you are logged in and ready to go navigate to http://localhost:8000/api/v1/whoami you should see your logged in user information.

### PostgreSQL

Rumba expects a database `mdn` owned by the user `rumba` (password `rumba`) on `127.0.0.1:5432` (see `[db]` in `.settings.dev.toml`).
It runs the migrations on startup, so the database can start out empty.

With Docker:

```sh
docker run --name rumba-postgres -d \
  -p 5432:5432 \
  -e POSTGRES_USER=rumba \
  -e POSTGRES_PASSWORD=rumba \
  -e POSTGRES_DB=mdn \
  postgres
```

On macOS, you can use [Postgres.app](https://postgresapp.com/) instead, and create the user and database:

```sh
psql -c "CREATE USER rumba WITH PASSWORD 'rumba';"
psql -c "CREATE DATABASE mdn OWNER rumba;"
```

### Elasticsearch

Rumba uses Elasticsearch 9, reachable at http://localhost:9200 (see `[search]` in `.settings.dev.toml`).

With Docker (security disabled, so it accepts plain HTTP without credentials):

```sh
docker run --name rumba-elastic -d \
  -p 9200:9200 \
  -e discovery.type=single-node \
  -e xpack.security.enabled=false \
  -e ES_JAVA_OPTS="-Xms1g -Xmx1g" \
  elasticsearch:9.1.0
```

Check that the cluster is up (status `green` or `yellow`):

```sh
curl http://localhost:9200/_cluster/health
```

### Search index

The search endpoint (`/api/v1/search`) queries the `mdn_docs` index, which the [deployer](https://github.com/mdn/dex/tree/main/deployer-js) in [mdn/dex](https://github.com/mdn/dex) populates from a build of [mdn/content](https://github.com/mdn/content).
Assuming both repositories are checked out next to Rumba:

1. Build the content (writes `index.json` files to `build/`):

   ```sh
   cd ../content
   npm install
   npm run build
   ```

2. Index the build:

   ```sh
   cd ../dex/deployer-js
   npm install
   node main.js search-index ../../content/build --url http://localhost:9200
   ```

   To index only a subset, pass a subdirectory of the build instead (e.g. `../../content/build/en-us/docs/web/css`).
   Each run creates a new index and moves the `mdn_docs` alias to it, so you can re-run it at any time.

3. With Rumba running, check that search returns results:

   ```sh
   curl "http://localhost:8000/api/v1/search?q=flexbox"
   ```

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
