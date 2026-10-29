#!/usr/bin/env bash
# False-positive check (PRD §14): scan pinned public repos and fail if any
# file reaches `block`. Read-only; needs git and a built rulesift binary.
#
#   scripts/fp-check.sh [RULESIFT_BIN] [OUT_DIR]
#
# A pattern of the form `=SRC:DST` is not a sparse pattern: after checkout the
# directory SRC is moved to DST (for collections that keep e.g. Cursor rules
# in `rules/` instead of `.cursor/rules/`).
#
# Writes OUT_DIR/<owner>__<repo>.json and OUT_DIR/summary.md.
set -euo pipefail

bin=${1:-target/release/rulesift}
out=${2:-fp-out}
list=${FP_REPOS:-$(dirname "$0")/fp-repos.txt}
bin=$(cd "$(dirname "$bin")" && pwd)/$(basename "$bin")
mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
# Built-in defaults plus reviewed true positives, independent of any
# rulesift.toml in the cloned repos.
config=$(cd "$(dirname "$0")" && pwd)/fp-accepted.toml

printf '| Repo | Commit | Files | With findings | Findings | Accepted | Block | Review | Info |\n|---|---|---|---|---|---|---|---|---|\n' > "$out/summary.md"
status=0
while read -r repo sha patterns; do
  case "$repo" in ''|'#'*) continue ;; esac
  dir="$work/${repo//\//__}"
  git init -q "$dir"
  git -C "$dir" remote add origin "https://github.com/$repo.git"
  sparse=() moves=()
  for p in $patterns; do
    case "$p" in =*) moves+=("${p#=}") ;; *) sparse+=("$p") ;; esac
  done
  if [ ${#sparse[@]} -gt 0 ]; then
    git -C "$dir" sparse-checkout set --no-cone "${sparse[@]}"
  fi
  git -C "$dir" fetch -q --depth 1 --filter=blob:none origin "$sha"
  git -C "$dir" -c advice.detachedHead=false checkout -q FETCH_HEAD
  for m in "${moves[@]+"${moves[@]}"}"; do
    mkdir -p "$dir/$(dirname "${m#*:}")"
    mv "$dir/${m%%:*}" "$dir/${m#*:}"
  done
  json="$out/${repo//\//__}.json"
  code=0
  (cd "$dir" && "$bin" --config "$config" scan . --format json --fail-on block) > "$json" || code=$?
  if [ "$code" -gt 1 ]; then
    echo "error: rulesift exited $code on $repo" >&2
    exit "$code"
  fi
  [ "$code" -eq 1 ] && status=1
  python3 - "$json" "$repo" "$sha" >> "$out/summary.md" <<'PY'
import json, sys
r = json.load(open(sys.argv[1])); s = r["summary"]
fs = r["files"]
live = sum(1 for f in fs for x in f["findings"] if not x["suppressed"])
acc = sum(1 for f in fs for x in f["findings"] if x["suppressed"])
withf = sum(1 for f in fs if any(not x["suppressed"] for x in f["findings"]))
print(f"| {sys.argv[2]} | `{sys.argv[3][:12]}` | {s['files_scanned']} | {withf} | {live} | {acc} | {s['block']} | {s['review']} | {s['info']} |")
for f in r["files"]:
    if f["tier"] == "block":
        ids = sorted({x["rule_id"] for x in f["findings"] if not x["suppressed"]})
        print(f"<!-- BLOCK {sys.argv[2]}:{f['path']} {' '.join(ids)} -->", file=sys.stderr)
PY
done < "$list"
cat "$out/summary.md"
exit "$status"
