# Shuttle's Christmas Code Hunt 2023 - Validator

Use this binary to run the official tests against your solution to challenges from [Shuttle's Christmas Code Hunt 2023](https://www.shuttle.dev/cch).

## Installation / Upgrading

```sh
cargo install cch23-validator
```

## Usage

```text
Usage: cch23-validator [OPTIONS] <NUMBERS|--all>

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
cch23-validator -1
cch23-validator 6 7
cch23-validator --all
```
