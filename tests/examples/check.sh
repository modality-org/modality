#!/usr/bin/env bash
# Check that the repo's examples still work: every .modality example parses,
# lints clean and meets its own formulas; the hub scenarios run end to end;
# every Rust example builds. Needs `modal` built (rust/target/debug/modal).
#
#   tests/examples/check.sh            # everything
#   tests/examples/check.sh models     # .modality files only
#   tests/examples/check.sh hub        # hub scenario walkthroughs only
#   tests/examples/check.sh rust       # build every Rust example

set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MODAL="$ROOT/rust/target/debug/modal"
export PATH="$ROOT/rust/target/debug:$PATH"
what="${1:-all}"
failed=0
fail() { echo "✗ $*"; failed=1; }

# Files whose formulas must hold on their own model and lint clean. The
# syntax demonstrations (modality-lang models/ and tests/, the VS Code
# examples) only have to parse: they show every operator, props included.
model_files() {
  ls "$ROOT"/examples/*.modality
  ls "$ROOT"/experiments/llm-synthesizer/examples/*.modality
  ls "$ROOT"/rust/modality-lang/examples/*.modality
  ls "$ROOT"/rust/modality-lang/examples/models/*.modality
  ls "$ROOT"/rust/modality-lang/examples/tests/*.modality
  ls "$ROOT"/common/modality-vscode/examples/simple.modality
  ls "$ROOT"/tutorials/*/contracts/*.modality
}

check_models() {
  for f in $(model_files); do
    rel="${f#$ROOT/}"
    if ! out=$("$MODAL" model mermaid "$f" </dev/null 2>&1 >/dev/null); then
      fail "$rel: does not parse: ${out:0:200}"
      continue
    fi
    # Syntax demonstrations: parsing is the check; some formulas there are
    # false on purpose.
    case "$rel" in
      rust/modality-lang/examples/models/*|rust/modality-lang/examples/tests/*|common/*) continue ;;
    esac
    for formula in $(grep -oE '^\s*formula [A-Za-z_][A-Za-z0-9_]*' "$f" | awk '{print $2}'); do
      result=$("$MODAL" model check "$f" -f "$formula" </dev/null 2>&1)
      if ! grep -q '✅ Formula is satisfied (any witness node)' <<<"$result"; then
        fail "$rel: formula $formula does not hold on the file's model"
      fi
    done
    "$MODAL" model lint "$f" --deny-warnings </dev/null >/dev/null 2>&1 || fail "$rel: lint findings"
  done
  showcase="$ROOT/common/modality-vscode/examples/formula-syntax.modality"
  result=$("$MODAL" model lint "$showcase" </dev/null 2>&1)
  grep -q 'formula(s)' <<<"$result" || fail "${showcase#$ROOT/}: formulas do not parse"
  echo "· .modality examples checked"
}

# Run a walkthrough's ```bash blocks in order, in one shell, in a scratch dir.
run_walkthrough() {
  local md="$1" work
  work=$(mktemp -d)
  python3 - "$md" > "$work/script.sh" <<'PY'
import re, sys
blocks = re.findall(r'```bash\n(.*?)```', open(sys.argv[1]).read(), flags=re.S)
print("set -e\ntrap 'kill $(jobs -p) 2>/dev/null || true' EXIT")
print("\n".join(blocks))
PY
  (cd "$work" && bash "$work/script.sh" </dev/null >"$work/log" 2>&1) \
    || { fail "${md#$ROOT/}: walkthrough failed (log: $work/log)"; return; }
  grep -q '^unexpected' "$work/log" && fail "${md#$ROOT/}: a refusal did not happen"
  rm -rf "$work"
}

check_hub() {
  for md in escrow-3party members-only treasury-multisig service-agreement agent-swarm; do
    run_walkthrough "$ROOT/examples/hub-scenarios/$md.md"
  done
  echo "· hub scenarios run"
}

check_rust() {
  (cd "$ROOT/rust" && cargo build --all --examples --quiet \
    && cargo build -p modality-miner --features persistence --examples --quiet) \
    || fail "a Rust example does not build"
  echo "· Rust examples built"
}

case "$what" in
  models) check_models ;;
  hub) check_hub ;;
  rust) check_rust ;;
  all) check_models; check_hub; check_rust ;;
  *) echo "usage: $0 [models|hub|rust|all]"; exit 2 ;;
esac

[ "$failed" -eq 0 ] && echo "✓ examples check passed"
exit "$failed"
