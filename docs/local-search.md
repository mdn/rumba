# Local search

Index MDN content into a local Elasticsearch and query it through a local Rumba, to test search changes end to end.

## Prerequisites

- Checkouts of [mdn/content](https://github.com/mdn/content) and [mdn/dex](https://github.com/mdn/dex) next to this repository.
- Services from `docker-compose.yml`:

  ```sh
  docker compose up -d --wait
  ```

## Build content

The Dex deployer installs `rari`, which builds the en-US content into a local output directory:

```sh
export CONTENT_ROOT=/path/to/mdn/content/files
export DEX_ROOT=/path/to/mdn/dex
export BUILD_OUT_ROOT=/tmp/mdn-search-build

npm --prefix "$DEX_ROOT/deployer-js" ci
BASE_URL=https://developer.mozilla.org \
LIVE_SAMPLES_BASE_URL=https://live.mdnplay.dev \
"$DEX_ROOT/deployer-js/node_modules/.bin/rari" build --no-basic --content
```

## Index

```sh
node "$DEX_ROOT/deployer-js/main.js" search-index "$BUILD_OUT_ROOT" --url http://localhost:9200
```

This creates the `mdn_docs` index and points its alias at it. Re-run it after rebuilding content or changing the Dex indexing code.

## Query

Start Rumba with `.settings.dev.toml`, which points search at `http://localhost:9200`:

```sh
MDN_SETTINGS=.settings.dev.toml cargo run
```

Then search through the API:

```sh
curl 'http://localhost:8000/api/v1/search?q=flexbox'
```

## Stop

```sh
docker compose stop  # keep containers and their data
docker compose down  # remove containers; the compose file has no volumes, so the index is lost and must be rebuilt
```
