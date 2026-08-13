# Integration tests

The tests expect the settings in `.settings.test.toml`:

```sh
MDN_SETTINGS=.settings.test.toml cargo test --all -- --test-threads=1 --nocapture
```
