#!/usr/bin/env bash
# Create a faucet: a contract that drips a fixed amount of an asset it holds,
# once, to each key that registers with it. Nobody owns it. Its first commit
# posts the drip and the asset, the model and the rules, and is signed by no
# one; after that the rules hold for good:
#   - a key is registered only by its holder (/claimants/<label>.id);
#   - a registered key sends exactly the drip, once, marking its slot;
#   - the faucet mints nothing, and its config, model and rules never change;
#   - anyone may fund it, with a RECV of a SEND to it and nothing else.
#
#   scripts/testnet/faucet/create.sh <dir> [asset] [drip]
#
# asset: <creator contract>:<asset id>; default: the testnet's MOD.
# drip: in the asset's smallest units; default 1000000000 (10 MOD).
# Push <dir> to a node to deploy it (`modal contract push`); keep <dir> as
# made, so the same faucet can be pushed to a network after a new genesis.
set -euo pipefail

DIR="${1:?usage: $0 <dir> [asset] [drip]}"
MODAL="${MODAL:-modal}"
HERE="$(cd "$(dirname "$0")" && pwd)"
INFO="$HERE/../../../rust/modality-networks/networks/testnet/info.json"
ASSET="${2:-$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['mod_contract']['contract_id'] + ':MOD')" "$INFO")}"
DRIP="${3:-1000000000}"

[ -e "$DIR" ] && { echo "$DIR already exists" >&2; exit 1; }
"$MODAL" contract create --dir "$DIR" > /dev/null
"$MODAL" contract set --dir "$DIR" /config/drip.num "$DRIP" > /dev/null
"$MODAL" contract set --dir "$DIR" /config/asset.text "$ASSET" > /dev/null

# Every step after the first leaves the config, model and rules alone.
FIXED="-modifies(/config) -modifies(/model) -modifies(/rules)"
cat > "$DIR/model/default.modality" <<MODEL
model Faucet {
  initial q0
  q0 --> q1: +POST
  q1 --> q1: +RECV -POST -SEND -CREATE $FIXED
  q1 --> q1: +POST -SEND -RECV -CREATE $FIXED -state_exists(/claimants/\$k.id) +post_to_path(/claimants/\$k.id) +posts_own_key(/claimants/\$k.id) -modifies(/claimants/\$k) -modifies(/claimants/!\$k)
  q1 --> q1: +SEND +POST -RECV -CREATE $FIXED +signed_by(/claimants/\$k.id) -signed_by(/claimants/!\$k.id) -bool_true(/claimants/\$k/claimed.bool) +post_to(/claimants/\$k/claimed.bool, "true") -modifies(/claimants/\$k.id) -modifies(/claimants/!\$k) +sent_eq("$ASSET", /config/drip.num)
}
MODEL

rule() {
    "$MODAL" add-rule --dir "$DIR" --name "$1" "$2" > /dev/null
}
rule config_fixed 'always([+modifies(/config)] false)'
rule model_fixed 'always([+modifies(/model)] false)'
rule rules_fixed 'always([+modifies(/rules)] false)'
rule no_create 'always([+CREATE] false)'
rule drip_size "always([+SEND -sent_eq(\"$ASSET\", /config/drip.num)] false)"
rule drips_once 'always([+SEND +signed_by(/claimants/$k.id) +bool_true(/claimants/$k/claimed.bool)] false)'
rule drip_marks 'always([+SEND +signed_by(/claimants/$k.id) -post_to(/claimants/$k/claimed.bool, "true")] false)'
rule drips_signed 'always([+SEND -any_signed(/claimants)] false)'
rule own_keys 'always([+post_to_path(/claimants/$k.id) -posts_own_key(/claimants/$k.id)] false)'

"$MODAL" commit --all --dir "$DIR" --message "Faucet" --output json > /dev/null
echo "Faucet $("$MODAL" contract id --dir "$DIR"): drips $DRIP of $ASSET once per registered key"
