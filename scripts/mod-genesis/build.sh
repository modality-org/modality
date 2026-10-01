#!/usr/bin/env bash
# Build a network's MOD contract genesis with the modal CLI.
#
#   scripts/mod-genesis/build.sh --out DIR --foundation PASSFILE --params PARAMS.json [--wasm FILE] [--canonical]
#
# PARAMS.json:
#   {"quantity": 2100000000000000, "divisibility": 100000000,
#    "block_subsidy": 5000000000, "halving_interval": 210000, "slow_start": 0,
#    "cap": 1900000000000000,
#    "hash_func": "randomx", "genesis_block_hash": "<hash of miner block 0>",
#    "allocations": [{"to": "<contract id>", "amount": 100000000000000}]}
#
# `hash_func` is the miner chain's proof of work (default randomx), and
# `genesis_block_hash`, when given, ties the first minted block to block 0.
#
# All amounts are in the smallest unit (1 MOD = divisibility units).
# `quantity` is the whole supply, `cap` what emission may pay out of it, and
# the allocations plus the cap may not exceed the quantity.
#
# The foundation key signs the bootstrap, the CREATE, each allocation and the
# rules; after the rules only the emission program's output is accepted, so
# the contract has no owner. Writes DIR/contract (the contract directory) and
# DIR/genesis.json, the value of a network config's `mod_contract`. Prints
# the contract id and the program's sha256.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT=""
FOUNDATION=""
PARAMS=""
WASM=""
CANONICAL=""
while [ $# -gt 0 ]; do
    case "$1" in
        --out) OUT="$2"; shift 2 ;;
        --foundation) FOUNDATION="$2"; shift 2 ;;
        --params) PARAMS="$2"; shift 2 ;;
        --wasm) WASM="$2"; shift 2 ;;
        --canonical) CANONICAL="--canonical"; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done
if [ -z "$OUT" ] || [ -z "$FOUNDATION" ] || [ -z "$PARAMS" ]; then
    echo "usage: $0 --out DIR --foundation PASSFILE --params PARAMS.json [--wasm FILE] [--canonical]" >&2
    exit 2
fi

if [ -z "$WASM" ]; then
    read -r WASM _ < <("$ROOT/programs/mod-emission/build.sh" $CANONICAL)
fi
if command -v sha256sum >/dev/null; then
    SHA=$(sha256sum "$WASM" | cut -d' ' -f1)
else
    SHA=$(shasum -a 256 "$WASM" | cut -d' ' -f1)
fi

param() {
    python3 - "$PARAMS" "$1" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
v = p.get(sys.argv[2], 0)
if not isinstance(v, int) or v < 0:
    sys.exit(f"{sys.argv[2]} must be a whole number of units, not {v!r}")
print(v)
PY
}
QUANTITY=$(param quantity)
DIVISIBILITY=$(param divisibility)
python3 - "$PARAMS" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
allocated = sum(a["amount"] for a in p.get("allocations", []))
if p["quantity"] <= 0 or p["divisibility"] <= 0:
    sys.exit("quantity and divisibility must be positive")
if allocated + p.get("cap", 0) > p["quantity"]:
    sys.exit(f"allocations ({allocated}) plus the emission cap ({p.get('cap', 0)}) exceed the quantity ({p['quantity']})")
for a in p.get("allocations", []):
    if not a.get("to") or not isinstance(a.get("amount"), int) or a["amount"] <= 0:
        sys.exit(f"each allocation needs a contract id `to` and a positive whole amount: {a}")
PY

DIR="$OUT/contract"
rm -rf "$DIR"
mkdir -p "$OUT"
modal contract create --dir "$DIR" >/dev/null
modal checkout --dir "$DIR" >/dev/null
modal set-named-id /foundation.id "$FOUNDATION" --dir "$DIR" >/dev/null
for name in block_subsidy halving_interval slow_start cap; do
    modal contract set --dir "$DIR" "/network/emission/$name.num" "$(param $name)" >/dev/null
done
text_param() {
    python3 - "$PARAMS" "$1" "$2" <<'PY'
import json, sys
print(json.load(open(sys.argv[1])).get(sys.argv[2], sys.argv[3]))
PY
}
modal contract set --dir "$DIR" /network/emission/hash_func.text "$(text_param hash_func randomx)" >/dev/null
GENESIS_BLOCK=$(text_param genesis_block_hash "")
if [ -n "$GENESIS_BLOCK" ]; then
    modal contract set --dir "$DIR" /network/emission/genesis_block_hash.text "$GENESIS_BLOCK" >/dev/null
fi
modal contract set --dir "$DIR" /emission/next_index.num 1 >/dev/null
modal contract set --dir "$DIR" /emission/emitted.num 0 >/dev/null
mkdir -p "$DIR/state/__programs__"
python3 - "$WASM" "$DIR/state/__programs__/emission.wasm" <<'PY'
import base64, pathlib, sys
pathlib.Path(sys.argv[2]).write_text(base64.b64encode(pathlib.Path(sys.argv[1]).read_bytes()).decode("ascii"))
PY

PROGRAM="/__programs__/emission.wasm"
EMITTED="+emitted_by($PROGRAM, \"$SHA\")"
FIXED="-modifies(/network) -modifies(/__programs__)"
mkdir -p "$DIR/model"
cat > "$DIR/model/default.modality" <<EOF
model Mod {
  initial q0
  q0 --> q1: +POST
  q1 --> q2: +CREATE -SEND -RECV $FIXED +signed_by(/foundation.id)
  q2 --> q2: +SEND -CREATE -RECV $FIXED -modifies(/emission) +signed_by(/foundation.id)
  q2 --> q3: +modifies(/rules) -SEND -RECV -CREATE $FIXED +signed_by(/foundation.id)
  q3 --> q3: $EMITTED +tracks(/emission/emitted.num, "MOD", "issued") +mined_headers(/emission/blocks) $FIXED
}
EOF

commit() {
    modal contract commit --theory v3 --dir "$DIR" --sign "$FOUNDATION" --output json "$@" >/dev/null
}
modal commit --theory v3 --all --dir "$DIR" --sign "$FOUNDATION" --output json --message Bootstrap >/dev/null
commit --method create --asset-id MOD --quantity "$QUANTITY" --divisibility "$DIVISIBILITY"
python3 - "$PARAMS" <<'PY' | while read -r to amount; do commit --method send --asset-id MOD --to-contract "$to" --amount "$amount"; done
import json, sys
for a in json.load(open(sys.argv[1])).get("allocations", []):
    print(a["to"], a["amount"])
PY

add_rule() {
    modal add-rule --name "$1" --dir "$DIR" "$2" >/dev/null
}
add_rule only_the_program "always([-emitted_by($PROGRAM, \"$SHA\")] false)"
add_rule emitted_counts_what_went_out "always([-tracks(/emission/emitted.num, \"MOD\", \"issued\")] false)"
add_rule parameters_fixed "always([+modifies(/network)] false)"
add_rule program_fixed "always([+modifies(/__programs__)] false)"
add_rule headers_are_mined "always([-mined_headers(/emission/blocks)] false)"
modal commit --theory v3 --all --dir "$DIR" --sign "$FOUNDATION" --output json --message Rules >/dev/null

CONTRACT_ID=$(modal contract id --dir "$DIR")
python3 - "$DIR" "$CONTRACT_ID" "$OUT/genesis.json" <<'PY'
import json, pathlib, sys
contract, contract_id, out = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
commits, current = [], (contract / ".contract/HEAD").read_text().strip()
while current:
    file = json.loads((contract / ".contract/commits" / f"{current}.json").read_text())
    commits.append({"commit_id": current, "body": file["body"], "head": file["head"]})
    current = file["head"].get("parent")
commits.reverse()
out.write_text(json.dumps({"contract_id": contract_id, "commits": commits}, indent=2) + "\n")
PY
echo "$CONTRACT_ID $SHA"
