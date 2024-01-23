## ai-test

This module allows to gather AI Help answers for quality assurance.

### Quickstart

To understand how this tool works, run: `cargo run -p ai-test -- test --help`

```
Usage: ai-test test [OPTIONS]

Options:
  -p, --path <PATH>  
  -o, --out <OUT>    
  -h, --help         Print help
```

For example, to request answers for all questions in the [prompts.yaml](./data/prompts.yaml) file, run (from the repository root):

```sh
cargo run -p ai-test -- test -p ai-test/data/prompts.yaml
```

By default, the results are written to the `/tmp/test` directory, unless you specify a different output directory (see above).