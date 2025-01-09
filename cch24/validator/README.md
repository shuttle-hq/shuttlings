# Shuttle's Christmas Code Hunt 2024 - Validator

Use this binary to run the official tests against your solution to challenges from [Shuttle's Christmas Code Hunt 2024](https://www.shuttle.dev/cch).

## Installation / Upgrading

```sh
cargo install cch24-validator
```

## Usage

```text
Usage: cch24-validator [OPTIONS] <NUMBERS|--all>

Arguments:
  [NUMBERS]...  The challenge numbers to validate

Options:
      --all        Validate all challenges
  -u, --url <URL>  The base URL to test against [default: http://127.0.0.1:8000]
  -h, --help       Print help
  -V, --version    Print version
```

## Examples

```sh
cch24-validator -1
cch24-validator 2 5
cch24-validator --all
```
