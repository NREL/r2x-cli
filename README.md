<img src="./assets/r2x-logo-full-color.svg" alt="R2X framework logo" align="left" width="260px" hspace="10" vspace="16"/>
<img align="left" alt="" width="0" height="192px" hspace="10"/>

#### r2x-cli
> Plugin manager and pipeline runner for model interoperability.
>
[![CI](https://github.com/NatLabRockies/r2x-cli/actions/workflows/build.yml/badge.svg)](https://github.com/NatLabRockies/r2x-cli/actions/workflows/build.yml) [![Latest release](https://img.shields.io/github/v/release/NatLabRockies/r2x-cli?display_name=tag&label=latest%20release&color=0079c2&logo=github)](https://github.com/NatLabRockies/r2x-cli/releases/latest) [![BSD 3-Clause](https://img.shields.io/badge/license-BSD--3--Clause-blue.svg)](./LICENSE.txt)
<br/>
[![Rust 1.72+](https://img.shields.io/badge/Rust-1.72%2B-dea584?logo=rust&logoColor=white)](docs/development.md) [![Python 3.11+](https://img.shields.io/badge/Python-3.11%2B-3776ab?logo=python&logoColor=white)](docs/development.md) [![Managed with uv](https://img.shields.io/badge/managed%20with-uv-6f42c1)](https://docs.astral.sh/uv/)

<br/>

<p align="center">
  <a href="#quickstart">Quickstart</a> ·
  <a href="#what-r2x-cli-does">Capabilities</a> ·
  <a href="#rust-workspace">Rust workspace</a> ·
  <a href="#documentation">Documentation</a> ·
  <a href="#development">Development</a>
</p>

`r2x-cli` is a Rust CLI for composing Python plugins into repeatable model
translation pipelines.
It discovers installed plugins, provisions the Python runtime with `uv`, and
keeps pipeline data and sidecar artifacts together across process boundaries.

## Quickstart

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

### Create a workspace

```bash
mkdir my-r2x-workspace
cd my-r2x-workspace
r2x init
r2x install r2x-reeds
r2x list
```

`r2x init` creates a starter `pipeline.yaml`.
Edit it for your input data and installed plugins.

### Inspect and run a pipeline

```bash
r2x run pipeline.yaml --list
r2x run pipeline.yaml <pipeline-name> --dry-run
r2x run pipeline.yaml <pipeline-name>
```

The first command that needs Python uses `uv` to provision a managed CPython
runtime and the r2x virtual environment.
A system Python executable is not required.
If `uv` is not installed, r2x-cli offers to install it with the official
installer.

## What r2x-cli does

| Capability | What it provides |
| --- | --- |
| Plugin discovery | Reads Python package metadata and source with static AST analysis before runtime invocation. |
| Pipeline execution | Chains plugins through named pipelines, Unix pipes, or durable JSON entrypoints. |
| Artifact handoff | Keeps JSON entrypoints beside their time-series and other sidecar directories. |
| Runtime management | Selects a compatible Python ABI and manages the `uv` virtual environment used by plugins. |

## Rust workspace

The command is backed by a focused Rust workspace.
Each package owns one boundary in the CLI and runtime:

| Package | Boundary |
| --- | --- |
| [`r2x`](crates/r2x-cli/) | CLI commands plus the `r2x` launcher and `r2x-runtime` payload. |
| [`r2x-config`](crates/r2x-config/) | Configuration, cache paths, Python versions, and virtual environments. |
| [`r2x-manifest`](crates/r2x-manifest/) | Persistent plugin metadata, package relationships, and runtime bindings. |
| [`r2x-ast`](crates/r2x-ast/) | Static discovery of Python plugin entry points and configuration schemas. |
| [`r2x-python`](crates/r2x-python/) | PyO3 initialization and plugin invocation in the managed runtime. |
| [`r2x-artifacts`](crates/r2x-artifacts/) | JSON, ZIP, sidecar, and durable pipeline artifact handoff. |
| [`r2x-logger`](crates/r2x-logger/) | Verbosity, file logging, progress reporting, and plugin diagnostics. |
| [`r2x-build-support`](crates/r2x-build-support/) | Build-time Python ABI detection and validation. |

See [the architecture guide](docs/architecture.md) for runtime boundaries and
data flow.

## Documentation

| Goal | Start here |
| --- | --- |
| Install r2x-cli and run a first pipeline | [Getting started](docs/getting-started.md) |
| Look up commands, flags, and runtime behavior | [CLI reference](docs/cli-reference.md) |
| Install, inspect, upgrade, or remove plugins | [Plugin management](docs/plugin-management.md) |
| Understand crates, discovery, and artifact boundaries | [Architecture](docs/architecture.md) |
| Build from source and run repository checks | [Development](docs/development.md) |

The [documentation index](docs/README.md) routes readers to the complete set of
user and developer guides.

## Development

Install Rust, `uv`, Python 3.11 or newer, and `just`.
Run the full local check set with:

```bash
just all
```

See [Development](docs/development.md) for source builds, Python ABI selection,
targeted checks, and troubleshooting.

## Model interoperability

r2x-cli orchestrates independently published packages that enable model
interoperability:

- [r2x-core](https://github.com/NatLabRockies/r2x-core): shared plugin framework.
- [R2X](https://github.com/NatLabRockies/R2X): translation plugins and model packages.
- [r2x-reeds](https://github.com/NatLabRockies/r2x-reeds): ReEDS parsing and transforms.
- [r2x-plexos](https://github.com/NatLabRockies/r2x-plexos): PLEXOS parsing and export.
- [r2x-sienna](https://github.com/NatLabRockies/r2x-sienna): Sienna parsing and export.
- [infrasys](https://github.com/NatLabRockies/infrasys): system storage and time-series management.

## Updates

Standalone installer users can update to the latest release with:

```bash
r2x self update
```

Users who installed with Cargo, Homebrew, or another package manager should
use that package manager's update command.

## License

BSD-3-Clause.
See [LICENSE.txt](./LICENSE.txt) for the full license text.
