#!/usr/bin/env bash
# Publish a release the Release workflow built, after checking it here.
#
#   scripts/packages/publish-ci-release.sh <run id>
#
# Downloads the run's `release-<version>` artifact and checks, with a `modal`
# you trust (MODAL, default: `modal` on PATH; build it from source at the
# release's commit), that the channel's release contract accepts the release
# and its files. Refuses unless the new release log extends the one already
# published, so a release cannot rewrite the contract's history. Then uploads
# the package to <channel>/<version>/, points <channel>/latest/ at it, and
# publishes the release log.
set -euo pipefail

RUN_ID="${1:?usage: $0 <release workflow run id>}"
MODAL="${MODAL:-modal}"
BUCKET="${BUCKET:-get.modality.org-content}"
BASE_URL="${BASE_URL:-https://get.modality.org}"
DISTRIBUTION_ID="${DISTRIBUTION_ID:-E1FBO6H39OPO86}"
REPO="${REPO:-modality-org/modality}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "Downloading run $RUN_ID..."
gh run download "$RUN_ID" --repo "$REPO" --pattern 'release-*' --dir "$WORK"
DIR=$(find "$WORK" -maxdepth 1 -type d -name 'release-*' | head -1)
[ -n "$DIR" ] || { echo "run $RUN_ID has no release artifact" >&2; exit 1; }

VERSION=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['version'])" "$DIR/manifest.json")
CHANNEL=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['git_branch'])" "$DIR/manifest.json")
echo "Release $CHANNEL $VERSION"

FILES=()
while read -r _ path; do
    FILES+=(--file "$path=$DIR/$path")
done < "$DIR/SHA256SUMS"
"$MODAL" release verify --log "$DIR/release-contract/log.json" --channel "$CHANNEL" \
    --version "$VERSION" "${FILES[@]}"

echo "Checking the new log extends the published one..."
if curl -fsSL "$BASE_URL/$CHANNEL/release-contract/log.json" -o "$WORK/published-log.json"; then
    python3 - "$WORK/published-log.json" "$DIR/release-contract/log.json" <<'PY'
import json, sys
old, new = (json.load(open(p)) for p in sys.argv[1:3])
if old["contract_id"] != new["contract_id"]:
    sys.exit("the new log is for another release contract")
old_ids = [c["commit_id"] for c in old["commits"]]
new_ids = [c["commit_id"] for c in new["commits"]]
if new_ids[: len(old_ids)] != old_ids:
    sys.exit("the new log does not extend the published one")
print(f"extends the published log by {len(new_ids) - len(old_ids)} commit(s)")
PY
else
    echo "No published log yet: this release starts it."
fi

echo "Uploading..."
aws s3 sync "$DIR" "s3://$BUCKET/$CHANNEL/$VERSION/" --exclude 'release-contract/*'
aws s3 sync "s3://$BUCKET/$CHANNEL/$VERSION/" "s3://$BUCKET/$CHANNEL/latest/" --delete
aws s3 cp "$DIR/release-contract/log.json" "s3://$BUCKET/$CHANNEL/release-contract/log.json" \
    --content-type application/json
aws cloudfront create-invalidation --distribution-id "$DISTRIBUTION_ID" \
    --paths "/$CHANNEL/latest/*" "/$CHANNEL/release-contract/*" > /dev/null
echo "Published $CHANNEL $VERSION: $BASE_URL/$CHANNEL/$VERSION/"
