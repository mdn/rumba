## ai-test

This module allows to gather AI Help answers for quality assurance.

### Quickstart

To understand how this tool works, run: `cargo run -p ai-test -- test --help`

```
Usage: ai-test test [OPTIONS]

Options:
  -p, --path <PATH>      Path to YAML file with list of lists (initial question + follow-up questions)
  -o, --out <OUT>        Path to directory to write the test results as `1.json`, `1.md`, etc
      --no-subscription  Perform test as free Core user without subscription
  -h, --help             Print help
```

For example, to request answers for all questions in the [prompts.yaml](./data/prompts.yaml) file, run (from the repository root):

```sh
cargo run -p ai-test -- test -p ai-test/data/prompts.yaml
```

By default, the results are written to the `/tmp/test` directory, unless you specify a different output directory (see above).