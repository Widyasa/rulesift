# Release checklist: v0.1.0

Only Widya tags, publishes, and manages secrets. This checklist was prepared
on 29 Oct 2026. Run each step in order and tick it in the release PR.

Name check (29 Oct 2026):

- `rulesift` is free on npm, crates.io, and PyPI, and not in homebrew-core.
- `rulesift-core` is free on crates.io.
- The tap repo `Widyasa/homebrew-tap` exists.

## 0. One-time setup

- [ ] **Token scope.** Give the CLI token the workflow scope:
  `gh auth refresh -h github.com -s workflow`
- [ ] **Actions.** Re-enable GitHub Actions for the repo (Settings →
  Actions), after the billing lock is resolved.
- [ ] **Workflows.** Activate the staged workflows as described in
  [`ci/workflows/README.md`](../ci/workflows/README.md): `git mv` them into
  `.github/workflows/`, run `dist plan`, and open a PR.
- [ ] **CI checks.** Confirm `ci`, `self-test`, `rulesift`, and `wheels` are
  green on that PR. Then close #4, #37, and #48.
- [ ] **Security settings** (Settings → Code security): enable private
  vulnerability reporting (SECURITY.md links to it) and Code Scanning.
- [ ] **Repository secrets** (Settings → Secrets and variables → Actions):
  - `HOMEBREW_TAP_TOKEN`: a fine-grained PAT with *Contents: read and write*
    on `Widyasa/homebrew-tap` only.
  - `NPM_TOKEN`: an npm *automation* token for the account that will own
    `rulesift`.
- [ ] **Registry accounts.** Log in locally:
  - crates.io: `cargo login` with an API token that has the
    `publish-new` and `publish-update` scopes;
  - PyPI: a project-scoped API token after the first upload, or set up
    trusted publishing.

## 1. Pre-flight on `master`

```sh
git switch master && git pull --ff-only
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release -p rulesift && scripts/fp-check.sh target/release/rulesift fp-out   # must exit 0
target/release/rulesift --version        # rulesift 0.1.0
target/release/rulesift scan . && target/release/rulesift verify
dist plan                                # lists 5 archives, sh/ps1 installers, rulesift.rb, npm package
cargo publish --dry-run -p rulesift-core
maturin build --release --locked -o /tmp/wheels   # optional local wheel check
```

- [ ] All of the above pass.
- [ ] **Release PR.** Set the release date in `CHANGELOG.md`
  (`## [0.1.0] - YYYY-MM-DD`) and merge the PR.
- [ ] **Status note.** Remove the "v0.1.0 is being prepared" note from
  `README.md` in the same PR.

## 2. crates.io (before the tag)

`rulesift` depends on `rulesift-core = "0.1.0"`, so publish the library
first:

```sh
cargo publish -p rulesift-core
# wait until https://crates.io/crates/rulesift-core shows 0.1.0
cargo publish -p rulesift
```

## 3. Tag and GitHub Release (cargo-dist)

```sh
git switch master && git pull --ff-only
git tag -a v0.1.0 -m "RuleSift v0.1.0"
git push origin v0.1.0
```

Pushing the tag starts `release.yml`. It:

- plans the release and builds the 5 targets;
- creates the GitHub Release with archives, `rulesift-installer.sh`,
  `rulesift-installer.ps1`, `rulesift.rb`, and `sha256.sum`;
- pushes `Formula/rulesift.rb` to `Widyasa/homebrew-tap`
  (`HOMEBREW_TAP_TOKEN`);
- runs `npm publish --access public` for `rulesift` (`NPM_TOKEN`).

Check:

- [ ] The release workflow is green.
- [ ] The release notes match the CHANGELOG.

## 4. PyPI

`wheels.yml` also runs on the tag. When it finishes:

```sh
run=$(gh run list --repo Widyasa/rulesift --workflow wheels.yml --branch v0.1.0 --limit 1 --json databaseId --jq '.[0].databaseId')
gh run download "$run" --repo Widyasa/rulesift -p 'wheel-*' -p sdist -D dist/
python -m pip install --upgrade twine
twine check dist/*/*
twine upload dist/*/*
```

- [ ] Upload done.
- [ ] Scope the PyPI token to the `rulesift` project for future releases.

## 5. Verify from a clean machine or container

```sh
npx rulesift --version                     # rulesift 0.1.0
pip install rulesift && rulesift --version # rulesift 0.1.0
brew install widyasa/tap/rulesift && rulesift --version
cargo install rulesift && rulesift --version
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Widyasa/rulesift/releases/download/v0.1.0/rulesift-installer.sh | sh
```

- [ ] **All four package managers** report `0.1.0`: npm, pip, Homebrew,
  cargo.
- [ ] **Action.** In a scratch repo, a workflow using
  `Widyasa/rulesift@v0.1.0` installs the release binary, uploads SARIF, and
  writes the PR comment.
- [ ] **pre-commit.** `pre-commit try-repo https://github.com/Widyasa/rulesift v0.1.0 rulesift-system --all-files`
  works with `rulesift` on PATH.

## 6. After the release

- [ ] **Dogfooding.** In `.github/workflows/rulesift.yml`, switch to
  `uses: Widyasa/rulesift@v0.1.0` and drop `install: source` and the Rust
  setup steps.
- [ ] **Changelog.** Add a new `## [Unreleased]` section to `CHANGELOG.md`.
- [ ] **patchsense.** Optionally adopt RuleSift in `Widyasa/patchsense` with
  [adopt-in-patchsense.md](adopt-in-patchsense.md).
- [ ] **Marketplace.** Optionally publish the Action to the GitHub
  Marketplace from the release page.

## Notes

- `rulesift-core` is published because `cargo install rulesift` needs it
  from crates.io. Its rules and report schema are packaged from
  `crates/rulesift-core/data/`, a copy of `rules/builtin/` and `schemas/`
  that a test keeps in sync.
- The `language: rust` pre-commit hook (`id: rulesift`) builds with
  `cargo install --path .`. That fails in this virtual workspace, so
  `rulesift-system` (needs `rulesift` on PATH) is the reliable hook until
  that is resolved. This decision is pending with Widya.
