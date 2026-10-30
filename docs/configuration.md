# Configuration

RuleSift works without any config. To customize it, create a commented
`rulesift.toml` with:

```sh
rulesift init
```

RuleSift uses the nearest `rulesift.toml`, searching upward from the first
scanned path. Two ways to override that:

- `--config FILE` uses a specific file.
- `RULESIFT_CONFIG=FILE` does the same from the environment. An empty value
  disables config discovery.

Which config applies depends on who controls it (#160):

- **`rulesift diff`** (the GitHub Action's pull request mode) and MCP
  `diff_refs` read `rulesift.toml` (and `rulesift.lock`) from the **base**
  revision, searching upward from the scanned directory in the base tree;
  without one there, the defaults apply. A pull request cannot weaken the
  policy that checks it. Changes to `rulesift.toml`, `rulesift.lock`, or
  custom rule files are reported:
  [RS-LOCK-004](rules/RS-LOCK-004.md) (exit 1) when they weaken the policy,
  [RS-LOCK-005](rules/RS-LOCK-005.md) (review only) otherwise. Custom rules
  in `extra_dirs` are read from the working tree, so changing them is
  RS-LOCK-004. `--config` and `RULESIFT_CONFIG` still win (an empty
  `RULESIFT_CONFIG` means the defaults); point them outside the checkout,
  never at a file the pull request can change.
- **`rulesift scan`** uses the config of the checkout it scans: that is your
  own policy on your own machine. When you vet third-party content (a
  downloaded skill), a `rulesift.toml` inside it would apply to its own
  scan; pass `--config` or `RULESIFT_CONFIG=` (empty: defaults) instead.
  The Action runs `scan` only outside pull requests.
- **MCP `scan_path`** uses the config of the server's working directory,
  never one inside the scanned path, so a copied skill cannot bring its own
  policy.

Every agent file skipped by configuration (`exclude`, `include`, `kinds`,
`tools`) gets a warning (`skipped by config: …`), except files under the
default excludes `node_modules/**` and `target/**`.

Unknown keys are an error.

## `[scan]`

| Key | Default | Meaning |
|---|---|---|
| `paths` | `["."]` | What to scan when no paths are given on the command line |
| `exclude` | `["node_modules/**", "target/**"]` | Globs to skip, matched relative to the directory of `rulesift.toml` (and to the scanned directory). They apply to walked files and to files passed explicitly on the command line. `.gitignore`, `.ignore`, and `.git/info/exclude` never hide an agent file, tracked in git or not, inside a git repository or not: agents load it from disk anyway, so it is scanned with a warning (`ignored by an ignore file; scanned anyway`). Use `exclude` to skip one. Directories matched by `exclude` are not walked at all. Ignored dependency directories named `node_modules` at any depth (e.g. `packages/ui/node_modules` in a monorepo after `npm ci`) are not walked either, unless they sit inside an agent location (below a hidden directory such as `.claude`, or in a skill directory): agents do not load agent files from installed packages. Files tracked in git there are still scanned. The default stays `node_modules/**` rather than `**/node_modules/**`, so a tracked `node_modules` inside a skill directory is still checked. A symlink at an agent location into a path RuleSift does not scan (an `exclude` match or such a dependency directory, installed or not) is reported as RS-FMT-008. If the git index cannot be read, a warning says that git submodules are not detected |
| `kinds` | all | `skill`, `instructions`, `command`, `agent`, `mcp-config`, `hooks`, `settings`, `plugin` |
| `tools` | `[]` (all) | Tool ids from [supported-tools.md](supported-tools.md), e.g. `["cursor", "gemini-cli"]` |
| `max_file_size_kib` | `2048` | Larger agent files are not scanned and are reported as [RS-FMT-008](rules/RS-FMT-008.md) (critical); larger text files inside a skill directory (scripts, references) get RS-FMT-008 too; larger binary assets there are still read in full (up to 64 MiB) and skipped with a warning only when they are a real binary format (structure check, as for smaller files); larger than 64 MiB gives RS-FMT-008 |

The command-line flags `--kinds`, `--tools`, `--include`, and `--exclude`
override these for one run.

When several paths are passed, the config is looked up from the deepest
directory that contains all of them (for example the repository root for
`rulesift scan docs/AGENTS.md AGENTS.md`).

## `[policy]`

| Key | Default | Meaning |
|---|---|---|
| `fail_on` | `"block"` | Exit 1 when any file reaches this tier: `block`, `review`, `info`, `never` |
| `min_severity` | `"low"` | Hide findings below this severity. Scores still use all findings |

## `[score]`

Each finding adds its severity weight (`critical`, `high`, `medium`, `low`) to
the file's score, capped at 100. Two adjustments:

- Files that agents always load (root `AGENTS.md`, `CLAUDE.md`,
  `alwaysApply: true` rules, hooks, settings, …) are multiplied by
  `always_loaded_multiplier`. Skills and subagents whose frontmatter declares
  `hooks:` count as always loaded too, like hooks in `settings.json`.
- A file that both reads secrets and sends data out gets `toxic_combo_bonus`.

A hook with a network or execution risk ([RS-CFG-002](rules/RS-CFG-002.md),
high) puts its file at least in `review`: the score is raised to `review_at`
wherever the hook is declared. Lowering the rule below `high` with
`severity_override` removes this floor.

The tier comes from the final score:

| Tier | Score |
|---|---|
| `block` | at least `block_at` (70) |
| `review` | at least `review_at` (40) |
| `info` | above 0 |
| `ok` | 0 |

## `[rules]`

| Key | Meaning |
|---|---|
| `extra_dirs` | Directories with your own YAML rules (same as `--rules DIR`) |
| `disable` | Rule ids to turn off (same as `--disable ID`) |
| `[rules.severity_override]` | e.g. `"RS-SEC-003" = "low"` |

User rules use the same YAML schema as [`rules/builtin/`](../rules/builtin/).
Each rule has:

- `id`, `title`, `severity`, `message`;
- an optional `applies_to`;
- a `match` tree (`regex`, `phrases`, `json_path`, `frontmatter`, `any`/`all`/`not`, `scope`);
- inline `examples` with `should_match` / `should_not_match`.

Validate rules and run their examples with:

```sh
rulesift rules test ./my-rules
```

## Suppressing findings

**Scoped suppression with a reason** (required):

```toml
[[suppress]]
rule = "RS-EXEC-001"
path = "skills/installer/SKILL.md"   # glob, optional (all files if absent)
reason = "Documented installer; script is vendored and pinned."
```

Suppressed findings stay in the JSON report with `"suppressed": true` and do
not count toward scores.

**Baseline**, to adopt RuleSift in an existing repo without fixing everything
first:

```sh
rulesift scan . --write-baseline rulesift-baseline.json   # record today's findings
rulesift scan . --baseline rulesift-baseline.json         # only new findings count
```

Fingerprints ignore line numbers, so moving text around does not resurface a
baselined finding.

Fingerprints are computed on a stable path, so a baseline written one way
matches every other way of scanning the same repo (`scan .`, `scan
/abs/path/to/repo`, `scan repo` from the parent directory, a no-argument scan
from a subdirectory, explicit file arguments) and every checkout location,
including CI runners. The path is relative to the first of:

1. the directory of the `rulesift.toml` in use, when every scanned path is
   inside it;
2. the enclosing git work tree;
3. the scanned directory (for a single file outside git: the directory its
   registry path starts in, e.g. the repo root of `.cursor/rules/x.mdc`).

Reported paths are relative to the working directory when you pass relative
paths. Absolute arguments, and no-argument scans that fall back to
`[scan].paths`, are reported relative to the same stable root instead of as
machine-specific absolute paths.

## `[explain]`

Optional AI second opinion with the agent CLI you already have; see
[explain.md](explain.md).

| Key | Default | Meaning |
|---|---|---|
| `agent` | `"auto"` | `auto`, `codex`, `claude`, `cursor` (env `RULESIFT_AGENT` wins over this) |
| `timeout_secs` | `120` | The agent process is killed after this |
| `max_findings` | `10` | Findings included in the prompt |

## `rulesift.lock`

`rulesift lock` records a hash for every agent file, skill directory, and MCP
server entry. Commit the file. `rulesift verify` (or `rulesift lock --check`)
then exits 1 with these findings:

- [RS-LOCK-001](rules/RS-LOCK-001.md): something changed;
- [RS-LOCK-002](rules/RS-LOCK-002.md): something is new;
- [RS-LOCK-003](rules/RS-LOCK-003.md): something is missing.

Line endings are normalized, but any other byte change, including invisible
characters, changes the hash. MCP `env` values are never stored; only their
names are.

The `lock` object in JSON reports shows the lock state of the checked
directory: `present` says whether it has a `rulesift.lock`, and `violations`
lists the RS-LOCK findings. `rulesift scan <dir>` and `rulesift diff` against
the working tree check the lock; `scan` reports violations (and prints a
warning) without changing its exit code, while `verify` and `diff` exit 1.
`lock` is `null` when the lock was not checked: a scan of single files or of
several paths, or `diff --head` between two revisions without policy changes.

In `diff` against the working tree, the **base** revision's `rulesift.lock`
is the approval (#160): a pull request that changes a locked file and runs
`rulesift lock` passes, with an [RS-LOCK-005](rules/RS-LOCK-005.md) review
note listing what its lock approves; a pull request that deletes the lock
or drops entries for files that still exist gets
[RS-LOCK-004](rules/RS-LOCK-004.md) (exit 1). `rulesift verify` checks the
working tree against its own `rulesift.lock` and has no base revision.
