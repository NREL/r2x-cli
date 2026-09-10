# Development Rules

## Conversational Style

- Keep answers short and concise.
- Be direct and avoid filler.
- Answer the user's question before making edits or running commands.
- When responding to feedback or an analysis, explicitly say whether you agree or disagree before describing the change.
- Do not use emojis in commits, issues, pull requests, or code.

## Project Context

- The product and repository are `r2x-cli`.
- `r2x` names the broader model interoperability domain in prose.
- Preserve the official `R2X` spelling only when matching an external brand or repository name.
- Use `r2x-cli` for the CLI product, repository, documentation, releases, and implementation as a whole.
- Keep `r2x` unchanged for the executable command, shell commands, command output, paths, environment variables, and technical identifiers.
- Use actual package names such as `r2x-core` and `r2x-reeds` for model interoperability packages.
- Keep r2x-cli documentation focused on the generic CLI product.
- Do not add application-specific translation guides, parser configuration, or application configuration to the core CLI documentation.
- Keep this terminology in `AGENTS.md`; do not recreate `CONTEXT.md` unless explicitly requested.

## Design Philosophy

- Avoid backward-compatibility layers, fallbacks, and migrations when a new workflow replaces obsolete behavior.
- Study established Rust and CLI patterns before designing a new approach.
- Choose the simplest implementation that fully meets the current requirements.
- Avoid speculative abstractions, configuration, and indirection.
- Grow the system in small, working layers.
- Keep crates and modules modular with clearly separated concerns.
- Prefer established, well-maintained dependencies when they reduce complexity or improve reliability.
- Check existing dependencies and public APIs before adding code or a new dependency.
- Make architectural decisions for the long term instead of accepting temporary paths intended for later replacement.
- Preserve unrelated user changes in this dirty working tree.

## Code Quality

- Read the relevant files and surrounding code before editing.
- Follow the repository's Rust edition, minimum supported Rust version, compiler lints, and Clippy configuration.
- Keep the minimum supported Rust version at Rust 1.72 unless the project requirements explicitly change.
- Treat the workspace's `unsafe_code = "forbid"` policy as mandatory.
- Use typed errors and explicit control flow for expected failures.
- Do not use `unwrap` or `expect` in production code, and do not introduce avoidable panic paths.
- Do not disable, suppress, or weaken a compiler or linter rule to make a change pass.
- Refactor code so configured checks pass without hiding diagnostics.
- Use public APIs rather than private implementation details.
- Add comments for non-obvious design decisions, constraints, and trade-offs.
- Keep user-facing CLI output, diagnostics, and exit behavior consistent with existing commands.
- Ask before removing intentional functionality or code unless the user has explicitly requested its removal.

## Testing

- Write behavioral tests against public crate or CLI interfaces.
- Prefer tests that verify observable behavior and survive implementation refactors.
- Add unit tests for focused crate behavior and integration tests for command or cross-crate behavior.
- When changing a test, run that test and iterate until it passes.
- Run the narrowest relevant check first, then the broader repository checks when practical.
- Use the repository's managed Python setup for tests that exercise PyO3 or the Python runtime.

Canonical checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
just test
```

The repository task shortcuts are:

```bash
just fmt
just build
just test
just clippy
just clippy-strict
just all
```

## Documentation

- Do not use bold text in Markdown or HTML.
- Keep each prose sentence on its own Markdown line, while preserving valid headings, tables, code fences, and list structure.
- Document implemented behavior, not history or speculative future behavior.
- Update task-focused documentation when a user-facing feature or command changes.
- Keep examples runnable and consistent with the current `r2x` command surface.

## Git

- Inspect `git status` before editing and preserve changes made by other sessions.
- Stage only explicit paths that were changed in this session.
- Never use `git add -A` or `git add .`.
- Never run `git reset --hard`, `git checkout .`, `git clean`, `git stash`, or `git commit --no-verify`.
- Do not overwrite, revert, or clean unrelated user work.
- If committing, use the repository's existing conventional prefixes such as `feat:`, `fix:`, `docs:`, and `chore:`.
- Do not commit unless the user requests it.
