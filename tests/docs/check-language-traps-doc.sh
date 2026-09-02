#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DOC="$ROOT_DIR/docs/language/rule-syntax.md"
GOTCHAS_DOC="$ROOT_DIR/docs/reference/gotchas.md"
MEMBERS_ONLY_TUTORIAL="$ROOT_DIR/docs/tutorials/members-only-contract.md"
JS_SDK_HUB_TUTORIAL="$ROOT_DIR/docs/tutorials/js-sdk-hub.md"
MULTISIG_TREASURY_TUTORIAL="$ROOT_DIR/docs/tutorials/multisig-treasury.md"
ORACLE_ESCROW_TUTORIAL="$ROOT_DIR/docs/tutorials/oracle-escrow.md"
FAQ_DOC="$ROOT_DIR/docs/faq.md"
HUB_REST_API_DOC="$ROOT_DIR/docs/reference/hub-rest-api.md"
IETF_METHODOLOGY_DOC="$ROOT_DIR/experiments/ietf-autoformalization/methodology.md"
ACME_SYNTHESIS_NOTES="$ROOT_DIR/experiments/ietf-autoformalization/rfc8555-acme/synthesis-notes.md"

required_patterns=(
  "### Commitment Versus Enabledness"
  "<+PAY> true"
  "[<+PAY>] true"
  "[+PAY] +signed_by(/parties/alice.id)"
  "Avoid \`[+PAY] true\` as a guard."
  "it does not prove \`PAY\`"
  "\`PAY\` is committed"
  "Run \`modality model lint <file>\`"
  "\`modality/vacuous-box-guard\`"
  "\`modality/implication-sugar\`"
  "rewrite them to explicit Boolean form"
  "Prefer explicit boolean form for conditional rules:"
  "!+modifies(/members) | +all_signed(/members)"
  "onboarding examples avoid it"
)

for pattern in "${required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$DOC"; then
    echo "rule syntax reference is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- 'φ[[:space:]]*->[[:space:]]*ψ| implies ' "$DOC"; then
  echo "rule syntax reference should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

gotchas_required_patterns=(
  "always(!<+ADD_MEMBER> true | <+ADD_MEMBER +all_signed(/members)> true)"
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
  "\`modality/implication-sugar\`"
  "always(!<+modifies(/x)> true | <+modifies(/x) +signed_by(/admin.id)> true)"
  "always(!<+modifies(/path)> true | <+modifies(/path) +all_signed(/members)> true)"
)

for pattern in "${gotchas_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$GOTCHAS_DOC"; then
    echo "gotchas reference is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$GOTCHAS_DOC"; then
  echo "gotchas reference should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

members_only_required_patterns=(
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
  "always(!<+modifies(/config)> true | <+modifies(/config) +signed_by(/admin.id)> true)"
)

for pattern in "${members_only_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MEMBERS_ONLY_TUTORIAL"; then
    echo "members-only tutorial is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$MEMBERS_ONLY_TUTORIAL"; then
  echo "members-only tutorial should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

js_sdk_hub_required_patterns=(
  "always(!<+RELEASE> true | <+RELEASE +signed_by(/parties/bob.id)> true)"
)

for pattern in "${js_sdk_hub_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$JS_SDK_HUB_TUTORIAL"; then
    echo "JS SDK hub tutorial is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$JS_SDK_HUB_TUTORIAL"; then
  echo "JS SDK hub tutorial should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

multisig_treasury_required_patterns=(
  "always(!<+WITHDRAW> true | ("
  "<+WITHDRAW +signed_by(/treasury/alice.id) +signed_by(/treasury/bob.id)> true"
  "<+WITHDRAW +signed_by(/treasury/alice.id) +signed_by(/treasury/carol.id)> true"
  "<+WITHDRAW +signed_by(/treasury/bob.id) +signed_by(/treasury/carol.id)> true"
)

for pattern in "${multisig_treasury_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MULTISIG_TREASURY_TUTORIAL"; then
    echo "multisig treasury tutorial is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$MULTISIG_TREASURY_TUTORIAL"; then
  echo "multisig treasury tutorial should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

oracle_escrow_required_patterns=(
  "always(!<+RELEASE> true | <+RELEASE +oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true)"
)

for pattern in "${oracle_escrow_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$ORACLE_ESCROW_TUTORIAL"; then
    echo "oracle escrow tutorial is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$ORACLE_ESCROW_TUTORIAL"; then
  echo "oracle escrow tutorial should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

faq_required_patterns=(
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
  "always(!<+RELEASE> true | <+RELEASE +after(/deadlines/expiry.datetime) +signed_by(/users/buyer.id)> true)"
)

for pattern in "${faq_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$FAQ_DOC"; then
    echo "FAQ is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$FAQ_DOC"; then
  echo "FAQ should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

hub_rest_api_required_patterns=(
  "always(![<+signed_by(/users/alice.id)>] true | eventually(<+RELEASE> true))"
)

for pattern in "${hub_rest_api_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$HUB_REST_API_DOC"; then
    echo "hub REST API reference is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$HUB_REST_API_DOC"; then
  echo "hub REST API reference should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_methodology_required_patterns=(
  'always(!<+sets(/path, "later")> true | !<+sets(/path, "earlier")> true)'
  'always(!<+sets(/path, "value")> true | <+signed_by(/users/p.id)> true)'
  'always(!<+sets(/token/status.text, "issued")> true | <+signed_by(/users/authorization_server.id)> true)'
)

for pattern in "${ietf_methodology_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_METHODOLOGY_DOC"; then
    echo "IETF methodology is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_METHODOLOGY_DOC"; then
  echo "IETF methodology should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

acme_synthesis_required_patterns=(
  '!<+sets(/order/status.text, "processing")> true | !<+sets(/order/status.text, "pending")>'
  '!<+sets(..., "valid")> true | <+signed_by(CA)>'
  '!(<+sets(..., "invalid")> true \| <+sets(/certificate/revoked.text, "true")> true) \| always([-sets(/certificate/in_use.text, "true")])'
)

for pattern in "${acme_synthesis_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$ACME_SYNTHESIS_NOTES"; then
    echo "ACME synthesis notes are missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$ACME_SYNTHESIS_NOTES"; then
  echo "ACME synthesis notes should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

echo "language traps doc check passed"
