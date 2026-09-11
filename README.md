<img src="./assets/r2x-logo-full-square.svg" alt="R2X framework logo" align="left" width="192px" height="192px" hspace="10"/>
<img align="left" alt="" width="0" height="192px" hspace="10"/>

#### r2x-cli
> Plugin manager and pipeline runner for model interoperability.
>
[![CI](https://github.com/NatLabRockies/r2x-cli/actions/workflows/build.yml/badge.svg)](https://github.com/NatLabRockies/r2x-cli/actions/workflows/build.yml) [![Latest release](https://img.shields.io/github/v/release/NatLabRockies/r2x-cli?display_name=tag&label=latest%20release&color=0079c2&logo=github)](https://github.com/NatLabRockies/r2x-cli/releases/latest) [![Last commit](https://img.shields.io/github/last-commit/NatLabRockies/r2x-cli?style=flat-square)](https://github.com/NatLabRockies/r2x-cli/commits/main)
<br/>
[![Rust 1.72+](https://img.shields.io/badge/Rust-1.72%2B-dea584?logo=rust&logoColor=white)](docs/development.md) [![Python 3.11+](https://img.shields.io/badge/Python-3.11%2B-3776ab?logo=python&logoColor=white)](docs/development.md) [![Managed with uv](https://img.shields.io/badge/managed%20with-uv-6f42c1)](https://docs.astral.sh/uv/) [![BSD 3-Clause](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](./LICENSE.txt)

<br/>

`r2x-cli` is a Rust CLI that discovers Python plugins, composes them into
repeatable pipelines, and manages the Python runtime with `uv`.
It keeps model data and sidecar artifacts together across process boundaries.

## Quick Start

### Install

macOS and Linux:

```bash
curl -fsSL https://github.com/NatLabRockies/r2x-cli/releases/latest/download/r2x-installer.sh | sh
```

Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/NatLabRockies/r2x-cli/releases/latest/download/r2x-installer.ps1 | iex"
```

Verify the installation:

```bash
r2x --version
```

### Run a pipeline

```bash
mkdir my-r2x-workspace
cd my-r2x-workspace
r2x init
r2x install r2x-reeds
r2x list
r2x run pipeline.yaml --list
r2x run pipeline.yaml <pipeline-name> --dry-run
r2x run pipeline.yaml <pipeline-name>
```

Edit the generated `pipeline.yaml` for your input data and installed plugins.
On the first command that needs Python, `r2x-cli` provisions a managed CPython
runtime and virtual environment through `uv`.

## Documentation

Use the guide that matches your task:

- [Getting started](docs/getting-started.md): install, initialize, and run a pipeline.
- [CLI reference](docs/cli-reference.md): commands, flags, runtime behavior, and help.
- [Plugin management](docs/plugin-management.md): install, inspect, upgrade, and remove plugins.
- [Architecture](docs/architecture.md): Rust crates, runtime boundaries, discovery, and artifacts.
- [Development](docs/development.md): source builds, checks, Python ABIs, and troubleshooting.

See the [documentation index](docs/README.md) for the complete guide map.

## Releases

- [Download the latest r2x-cli release](https://github.com/NatLabRockies/r2x-cli/releases/latest).
- [Browse all releases](https://github.com/NatLabRockies/r2x-cli/releases).

## Development

Install Rust, `uv`, Python 3.11 or newer, and `just`.
Run the aggregate check with:

```bash
just all
```

See [Development](docs/development.md) for targeted formatting, lint, build, and test commands.

## License

BSD-3-Clause.
See [LICENSE.txt](./LICENSE.txt) for the full license text.
