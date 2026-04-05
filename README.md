<div align="center">

# gundi

### Unearth your forgotten code debt

TODO scanner with git blame age tracking

[![GitHub stars](https://img.shields.io/github/stars/iamkorun/gundi?style=flat-square)](https://github.com/iamkorun/gundi/stargazers)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/gundi?style=flat-square)](https://crates.io/crates/gundi)

<br />

[Installation](#installation) · [Usage](#usage) · [CI Integration](#ci-integration) · [Contributing](#contributing)

</div>

---

## The Problem

Every codebase accumulates TODO comments. They start as quick reminders, then quietly age into forgotten relics — some from developers who left years ago. You have no idea how many there are, who wrote them, or how long they have been rotting.

## The Solution

**gundi** scans your codebase for `TODO`, `FIXME`, `HACK`, `BUG`, and `XXX` comments, then runs `git blame` on each one to reveal the author, date, and age in days. Ancient debt (>90 days) glows red. Aging debt (>30 days) turns yellow. You see exactly what is rotting and who left it there.

Use it locally to audit your project, or drop it into CI to block merges when code debt gets too old.

---

## Demo

<!-- Replace with actual recording: asciinema, VHS tape, or GIF -->

```
$ gundi .

TYPE   FILE                                     LINE   AUTHOR               AGE          TEXT
--------------------------------------------------------------------------------------------------------------
TODO   src/parser.rs                            42     alice                 142d ago     refactor this loop
FIXME  src/handler.rs                           87     bob                   63d ago      handle timeout case
HACK   tests/integration.rs                     15     charlie               8d ago       temporary workaround
TODO   src/config.rs                            201    alice                 310d ago     support YAML format

Total: 4 items
```

Items older than 90 days appear in **red**. Items older than 30 days appear in **yellow**.

---

## Quick Start

```sh
cargo install gundi
cd your-project
gundi .
```

---

## Installation

### From crates.io

```sh
cargo install gundi
```

### From source

```sh
git clone https://github.com/iamkorun/gundi.git
cd gundi
cargo install --path .
```

---

## Usage

### Basic scan

```sh
gundi .
```

### Filter by comment type

```sh
gundi --type todo,fixme .
```

### Filter by author

```sh
gundi --author alice .
```

### Show only old debt (>60 days)

```sh
gundi --older-than 60 .
```

### Summary mode

```sh
gundi --summary .
```

```
Code Debt Summary: 12 total items

By Type:
  TODO     7
  FIXME    3
  HACK     2

By Author:
  alice                4
  bob                  5
  charlie              3

Age range: 8d - 310d
```

### JSON output (pipe to jq, dashboards, etc.)

```sh
gundi --json .
```

### Markdown output (for reports, PRs)

```sh
gundi --md .
```

### Fast mode (skip git blame)

```sh
gundi --no-blame .
```

### Combine filters

```sh
gundi --type todo --author alice --older-than 90 .
```

---

## CI Integration

Add gundi to your GitHub Actions workflow to block PRs that introduce ancient debt:

```yaml
# .github/workflows/debt-check.yml
name: Code Debt Check

on: [pull_request]

jobs:
  debt-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # full history needed for git blame

      - name: Install gundi
        run: cargo install gundi

      - name: Check code debt
        run: gundi --fail-on 90 .
```

`--fail-on 90` exits with code 1 if any TODO/FIXME/HACK/BUG/XXX comment is older than 90 days. Adjust the threshold to match your team's policy.

---

## Features

- **Fast parallel scanning** — uses rayon for multi-threaded file walking, respects `.gitignore`
- **Git blame enrichment** — author, date, and age in days for every comment
- **Color-coded output** — red for ancient debt (>90d), yellow for aging debt (>30d)
- **Flexible filters** — by comment type, author, or minimum age
- **Multiple output formats** — colored table (default), JSON, Markdown
- **CI gate** — `--fail-on N` exits non-zero when debt exceeds your threshold
- **Summary mode** — quick overview of debt distribution by type and author
- **No-blame mode** — skip git blame for faster scans when you just want a count

---

## Contributing

Contributions are welcome. Please open an issue first to discuss what you would like to change.

1. Fork the repo
2. Create your branch (`git checkout -b feat/my-feature`)
3. Commit your changes (`git commit -m 'feat: add my feature'`)
4. Push to the branch (`git push origin feat/my-feature`)
5. Open a Pull Request

---

## License

[MIT](LICENSE)

---

<div align="center">

[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-☕-orange?style=flat-square)](https://buymeacoffee.com/iamkorun)

Made with Rust.

</div>
