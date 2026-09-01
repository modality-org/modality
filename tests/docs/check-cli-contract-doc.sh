#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/cli/contract-commands.md"
MODAL_MAIN="$ROOT_DIR/rust/modal/src/main.rs"
COMMIT_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/commit.rs"
DIFF_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/diff.rs"
PUSH_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/push.rs"
PULL_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/pull.rs"
PACK_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/pack.rs"
UNPACK_SOURCE="$ROOT_DIR/rust/modal-cli-contract/src/unpack.rs"

required_patterns=(
  "# Contract Commands (\`modal contract\` / \`modal c\`)"
  "modal c create [OPTIONS]"
  "\`--dir <DIR>\` | Directory path where the contract will be created"
  "\`--output <FORMAT>\` | Output format: \`text\` or \`json\`"
  "modal c checkout [OPTIONS]"
  "\`--dir <DIR>\` | Contract directory (defaults to current directory)"
  "modal c set [OPTIONS] <PATH> <VALUE>"
  "modal c set-named-id [OPTIONS] <PATH> <NAME>"
  "passfile path or a passfile name"
  "modal c set-named-id /parties/alice.id alice.passfile"
  "Create a new commit from the contract working directories, a single state path,"
  "\`--path <PATH>\` | State path to write for a single \`POST\`-style commit"
  "\`--value <VALUE>\` | Value for the single-path commit"
  "\`--method <METHOD>\` | Commit method for the single-path commit"
  "\`--sign <PASSFILE>\` | Sign commit with a passfile; repeat to attach multiple signatures"
  "\`--all\`, \`-a\` | Commit all changed \`state/\`, \`rules/\`, and \`model/default.modality\` files"
  "\`--asset-id <ASSET_ID>\` | Asset ID for \`CREATE\` commits"
  "\`--to-contract <TO_CONTRACT>\` | Destination contract ID for \`SEND\` commits"
  "\`--send-commit-id <SEND_COMMIT_ID>\` | Source \`SEND\` commit ID for \`RECV\` commits"
  "modal c commit --path /notes.text --value \"signed update\" --sign alice.passfile"
  "modal c commit --all --sign alice.passfile --sign bob.passfile"
  "\`--remote <URL>\` | Target node multiaddress or hub URL; also saves it under the remote name"
  "\`--remote-name <NAME>\` | Remote name (default: \`origin\`)"
  "\`--hub-creds <FILE>\` | Hub credentials file for HTTP hub remotes"
  "modal c pull [URL] [OPTIONS]"
  "\`[URL]\` | Full contract URL to clone"
  "\`pack\` | \`--output <FILE>\`, \`-o <FILE>\` | Output \`.contract\` file path"
  "\`unpack\` | \`--force\` | Overwrite an existing output directory"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq -- "$pattern" "$DOC"; then
    echo "contract command reference is missing current help-surface text: $pattern" >&2
    exit 1
  fi
done

for forbidden_pattern in \
  "modal c set-named-id <PATH> --named <NAME>" \
  "modal c set-named-id /parties/alice.id --named alice" \
  "\`--name <NAME>\` | Contract name" \
  "\`--template <TEMPLATE>\` | Initialize from template" \
  "\`--file <FILE>\` | Read value from file" \
  "\`--type <TYPE>\` | Explicit path type" \
  "\`--state\` | Commit only state changes" \
  "\`--rules\` | Commit only rule changes" \
  "\`--model\` | Commit only model changes" \
  "\`--commit <HASH>\` | Checkout specific commit" \
  "\`--commit <HASH>\` | Compare against specific commit" \
  "\`--force\` | Force push" \
  "\`--checkout\` | Checkout after pulling" \
  "\`--stat\` | Show only file statistics"; do
  if grep -Fq -- "$forbidden_pattern" "$DOC"; then
    echo "contract command reference still contains stale help-surface text: $forbidden_pattern" >&2
    exit 1
  fi
done

for source_guard in \
  'path: Option<String>' \
  'value: Option<String>' \
  'method: String' \
  'dir: Option<PathBuf>' \
  'output: String' \
  'asset_id: Option<String>' \
  'quantity: Option<u64>' \
  'divisibility: Option<u64>' \
  'to_contract: Option<String>' \
  'amount: Option<u64>' \
  'send_commit_id: Option<String>' \
  'sign: Vec<PathBuf>' \
  'all: bool' \
  'message: Option<String>' \
  'action: Option<String>'; do
  if ! grep -Fq -- "$source_guard" "$COMMIT_SOURCE"; then
    echo "contract commit source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

for stale_source_flag in \
  'state: bool' \
  'rules: bool' \
  'model: bool'; do
  if grep -Fq -- "$stale_source_flag" "$COMMIT_SOURCE"; then
    echo "contract commit source grew a flag that should be documented deliberately: $stale_source_flag" >&2
    exit 1
  fi
done

for source_guard in \
  'dir: Option<PathBuf>' \
  'output: String'; do
  if ! grep -Fq -- "$source_guard" "$DIFF_SOURCE"; then
    echo "contract diff source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

if grep -Fq -- 'commit: Option' "$DIFF_SOURCE"; then
  echo "contract diff source grew a commit comparison option that should be documented deliberately" >&2
  exit 1
fi

for source_guard in \
  'remote: Option<String>' \
  'remote_name: String' \
  'dir: Option<PathBuf>' \
  'node_dir: Option<PathBuf>' \
  'hub_creds: Option<PathBuf>' \
  'output: String'; do
  if ! grep -Fq -- "$source_guard" "$PUSH_SOURCE"; then
    echo "contract push source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
  if ! grep -Fq -- "$source_guard" "$PULL_SOURCE"; then
    echo "contract pull source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

if ! grep -Fq -- 'url: Option<String>' "$PULL_SOURCE"; then
  echo "contract pull source no longer exposes documented clone URL argument" >&2
  exit 1
fi

for source_guard in \
  'pub output: Option<PathBuf>' \
  'pub dir: Option<PathBuf>'; do
  if ! grep -Fq -- "$source_guard" "$PACK_SOURCE"; then
    echo "contract pack source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

for source_guard in \
  'input: PathBuf' \
  'output: Option<PathBuf>' \
  'force: bool'; do
  if ! grep -Fq -- "$source_guard" "$UNPACK_SOURCE"; then
    echo "contract unpack source no longer exposes documented option: $source_guard" >&2
    exit 1
  fi
done

for command_variant in \
  "Commit(modal_cli_contract::commit::Opts)" \
  "Diff(modal_cli_contract::diff::Opts)" \
  "Push(modal_cli_contract::push::Opts)" \
  "Pull(modal_cli_contract::pull::Opts)" \
  "Pack(modal_cli_contract::pack::Opts)" \
  "Unpack(modal_cli_contract::unpack::Opts)"; do
  if ! grep -Fq -- "$command_variant" "$MODAL_MAIN"; then
    echo "modal wrapper no longer wires documented contract command: $command_variant" >&2
    exit 1
  fi
done

echo "contract command doc check passed"
