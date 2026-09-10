<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/r2x-logo-white.svg">
  <source media="(prefers-color-scheme: light)" srcset="./assets/r2x-logo-full-color.svg">
  <img alt="R2X" src="./assets/r2x-logo-full-color.svg" width="360">
</picture>

<p>Plugin manager and pipeline runner for the r2x power systems modeling ecosystem.</p>

[![CI](https://github.com/NatLabRockies/r2x-cli/actions/workflows/build.yml/badge.svg)](https://github.com/NatLabRockies/r2x-cli/actions/workflows/build.yml)
[![Release](https://github.com/NatLabRockies/r2x-cli/actions/workflows/release.yml/badge.svg?event=push)](https://github.com/NatLabRockies/r2x-cli/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-BSD--3--Clause-blue)](./LICENSE.txt)

</div>

`r2x-cli` discovers Python plugins, chains them into pipelines, and manages the
runtime needed to translate power system models from one format to another.
The `r2x` command is written in Rust and runs plugins in a managed Python
environment.

## Quick start

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

Edit the generated `pipeline.yaml` for your input data and installed plugins,
then validate and run a named pipeline:

```bash
r2x run pipeline.yaml --list
r2x run pipeline.yaml <pipeline-name> --dry-run
r2x run pipeline.yaml <pipeline-name>
```

On the first command that needs Python, `r2x-cli` uses `uv` to provision a
managed CPython runtime and the r2x virtual environment. A system Python
executable is not required. If `uv` is not installed, `r2x-cli` offers to
install it with the official installer.

## Documentation

The [documentation index](docs/README.md) routes readers to task-focused
guides for r2x-cli:

- [Getting started](docs/getting-started.md): install, initialize, and run a pipeline.
- [CLI reference](docs/cli-reference.md): common commands, shared flags, runtime commands, and system inspection.
- [Plugin management](docs/plugin-management.md): install, inspect, upgrade, and remove plugins.
- [Architecture](docs/architecture.md): crates, runtime boundaries, and plugin discovery.
- [Development](docs/development.md): source builds, tests, linting, and troubleshooting.
- [Torc plugin streams](docs/torc-plugin-streams.md): compose live and durable plugin boundaries.

## Updates

Standalone installer users can update to the latest release with:

```bash
r2x self update
```

Users who installed with Cargo, Homebrew, or another package manager should
use that package manager's update command.

## Ecosystem

r2x-cli orchestrates independently published r2x packages:

- [r2x-core](https://github.com/NatLabRockies/r2x-core): shared plugin framework.
- [R2X](https://github.com/NatLabRockies/R2X): translation plugins and model packages.
- [r2x-reeds](https://github.com/NatLabRockies/r2x-reeds): ReEDS parsing and transforms.
- [r2x-plexos](https://github.com/NatLabRockies/r2x-plexos): PLEXOS parsing and export.
- [r2x-sienna](https://github.com/NatLabRockies/r2x-sienna): Sienna parsing and export.
- [infrasys](https://github.com/NatLabRockies/infrasys): system storage and time-series management.

## License

BSD-3-Clause. See [LICENSE.txt](./LICENSE.txt) for the full text.
