# Contributing to RuleSift

Thanks for helping. RuleSift is a Rust workspace:

| Path | Contents |
|---|---|
| `crates/rulesift-core` | Library: registry, discovery, parsers, rules, scoring, reports. It never prints and is synchronous |
| `crates/rulesift` | The CLI, MCP server, and `--explain` adapters |
| `rules/builtin/*.yaml` | Built-in rules, embedded at build time |
| `docs/rules/RS-*.md` | One page per rule (a test checks they exist and are complete) |
| `tests/fixtures/` | Harmless fixtures. Use `*.example` / `example.invalid` domains; never live payloads |

Architecture, scope, and the rule list are in [PRD.md](PRD.md). Agent
contributors also follow [AGENTS.md](AGENTS.md).

## Before you open a PR

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

If you changed output or snapshots, review and accept the snapshots:

```sh
INSTA_UPDATE=always cargo test --workspace   # then review `git diff`
```

Regenerate the generated files when you change their inputs, then commit them:

- **Registry** (`docs/supported-tools.md`): `RULESIFT_BLESS=1 cargo test -p rulesift-core --test registry_docs`
- **Pre-commit hooks** (`.pre-commit-hooks.yaml`): `RULESIFT_BLESS=1 cargo test -p rulesift --test init_cmd`
- **Packaged data** (`crates/rulesift-core/data/`, a copy of `rules/builtin/` and `schemas/`): `RULESIFT_BLESS=1 cargo test -p rulesift-core --test embedded_data`

## Adding or changing a rule

1. Edit `rules/builtin/<category>.yaml`, then resync the packaged copy with
   `RULESIFT_BLESS=1 cargo test -p rulesift-core --test embedded_data`.
   Every rule needs inline `examples` with `should_match` and
   `should_not_match`; `cargo test` runs them.
2. Add or update `docs/rules/<ID>.md` with the sections What it detects, Why
   it matters, Example, How to fix, References, and Affected tools.
3. Run the false-positive check on the public corpus. A rule change must not
   push any file to `block`; record new numbers in `docs/false-positives.md`.

   ```sh
   cargo build --release -p rulesift
   scripts/fp-check.sh target/release/rulesift fp-out
   ```

## Adding a tool or file format

Add a row to the registry in `crates/rulesift-core/src/registry.rs`. Each row
has:

- the path globs, kind, and parser family;
- the always-loaded rule;
- a link to the tool's official docs.

Then add a clean file to `tests/fixtures/repos/all-tools/` and one with a
harmless payload to `tests/fixtures/repos/all-tools-positive/`, and regenerate
`docs/supported-tools.md`.

## Commits and PRs

- Use Conventional Commits (`feat(rules): …`, `fix(diff): …`, `docs: …`).
- Keep each PR to one task.
- All user-facing text is English.
- New dependencies must be justified in the PR. The approved crate list is in
  PRD §15.

By contributing you agree that your work is dual-licensed under MIT OR
Apache-2.0, like the rest of the project.
