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
MODELS_VS_RULES_DOC="$ROOT_DIR/docs/concepts/models-vs-rules.md"
MODAL_LOGIC_DOC="$ROOT_DIR/docs/concepts/modal-logic.md"
MEMBERS_ONLY_EXAMPLE="$ROOT_DIR/examples/members_only.modality"
TREASURY_MULTISIG_EXAMPLE="$ROOT_DIR/examples/treasury_multisig.modality"
ORACLE_ESCROW_EXAMPLE="$ROOT_DIR/examples/oracle_escrow.modality"
HUB_MEMBERS_ONLY_SCENARIO="$ROOT_DIR/examples/hub-scenarios/members-only.md"
HUB_BANK_DEPOSITS_SCENARIO="$ROOT_DIR/examples/hub-scenarios/bank-deposits.md"
BANK_DEPOSITS_JS_EXAMPLE="$ROOT_DIR/examples/bank_deposits.js"
UNBREAKABLE_TREASURY_RFC="$ROOT_DIR/docs/rfcs/RFC-002-UNBREAKABLE-TREASURY.md"
IETF_AUTOFORMALIZATION_PLAN="$ROOT_DIR/docs/progress/IETF_AUTOFORMALIZATION_PLAN.md"
RFC_0001_RESOURCE="$ROOT_DIR/docs/resources/rfc-0001.md"
STANDARD_PREDICATES_DOC="$ROOT_DIR/docs/reference/standard-predicates.md"

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

models_vs_rules_required_patterns=(
  "always(!<+CHANGE_CONFIG> true | <+CHANGE_CONFIG +signed_by(/admin.id)> true)"
  "always(!<+CHANGE_DATA> true | <+CHANGE_DATA +any_signed(/members)> true)"
  "always(!<+CHANGE_MEMBERS> true | <+CHANGE_MEMBERS +all_signed(/members)> true)"
  "always(!<+CHANGE_PUBLIC> true | <+CHANGE_PUBLIC +any_signed(/members)> true)"
  "always(!<+CHANGE_PRIVATE> true | <+CHANGE_PRIVATE +all_signed(/members)> true)"
  "Prefer explicit Boolean form for conditional rules:"
)

for pattern in "${models_vs_rules_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MODELS_VS_RULES_DOC"; then
    echo "models-vs-rules concept doc is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$MODELS_VS_RULES_DOC"; then
  echo "models-vs-rules concept doc should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

modal_logic_required_patterns=(
  "!φ | ψ          // Conditional rule: if φ, then ψ"
  "Prefer explicit Boolean conditionals in examples."
  "!<+RELEASE> true | <+RELEASE +signed_by(/parties/buyer.id)> true"
)

for pattern in "${modal_logic_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MODAL_LOGIC_DOC"; then
    echo "modal logic concept doc is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- 'φ[[:space:]]*->[[:space:]]*ψ| true[[:space:]]*->| implies ' "$MODAL_LOGIC_DOC"; then
  echo "modal logic concept doc should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

members_only_example_required_patterns=(
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
)

for pattern in "${members_only_example_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$MEMBERS_ONLY_EXAMPLE"; then
    echo "members-only example is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$MEMBERS_ONLY_EXAMPLE"; then
  echo "members-only example should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

treasury_multisig_example_required_patterns=(
  "always(!<+EXECUTE> true | <+EXECUTE +threshold(\"2\", /treasury/signers)> true)"
  "!<+PROPOSE_SIGNER_CHANGE> true | <+PROPOSE_SIGNER_CHANGE +threshold(\"3\", /treasury/signers)> true"
  "!<+CONFIRM_SIGNER_CHANGE> true | <+CONFIRM_SIGNER_CHANGE +threshold(\"3\", /treasury/signers)> true"
  "always(!<+CANCEL> true | <+CANCEL +signed_by(/treasury/proposer.id)> true)"
)

for pattern in "${treasury_multisig_example_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$TREASURY_MULTISIG_EXAMPLE"; then
    echo "treasury multisig example is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$TREASURY_MULTISIG_EXAMPLE"; then
  echo "treasury multisig example should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

oracle_escrow_example_required_patterns=(
  "always(!<+RELEASE> true | <+RELEASE +oracle_attests(/oracles/delivery, \"delivered\", \"true\")> true)"
  "!<+REFUND> true | <+REFUND +oracle_attests(/oracles/delivery, \"delivered\", \"false\")> true"
  "!<+TIMEOUT_REFUND> true | <+TIMEOUT_REFUND +signed_by(/users/buyer.id) +after(/escrow/deadline)> true"
  "always(!<+DEPOSIT> true | <+DEPOSIT +signed_by(/users/buyer.id)> true)"
  "always(!<+SHIP> true | <+SHIP +signed_by(/users/seller.id)> true)"
)

for pattern in "${oracle_escrow_example_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$ORACLE_ESCROW_EXAMPLE"; then
    echo "oracle escrow example is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$ORACLE_ESCROW_EXAMPLE"; then
  echo "oracle escrow example should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

hub_members_only_scenario_required_patterns=(
  "always(!<+ADD_MEMBER> true | <+ADD_MEMBER +all_signed(/members)> true)"
)

for pattern in "${hub_members_only_scenario_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$HUB_MEMBERS_ONLY_SCENARIO"; then
    echo "hub members-only scenario is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$HUB_MEMBERS_ONLY_SCENARIO"; then
  echo "hub members-only scenario should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

hub_bank_deposits_scenario_required_patterns=(
  "!<+WITHDRAW> true |"
  "+WITHDRAW"
  "+signed_by(/action/account.id)"
  "+balance_sufficient("
)

for pattern in "${hub_bank_deposits_scenario_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$HUB_BANK_DEPOSITS_SCENARIO"; then
    echo "hub bank-deposits scenario is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$HUB_BANK_DEPOSITS_SCENARIO"; then
  echo "hub bank-deposits scenario should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

for pattern in "${hub_bank_deposits_scenario_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$BANK_DEPOSITS_JS_EXAMPLE"; then
    echo "bank deposits JS example is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$BANK_DEPOSITS_JS_EXAMPLE"; then
  echo "bank deposits JS example should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

unbreakable_treasury_rfc_required_patterns=(
  "always (!<+modifies(/transfer)> true | <+modifies(/transfer) +signed_by(/owner.id)> true)"
)

for pattern in "${unbreakable_treasury_rfc_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$UNBREAKABLE_TREASURY_RFC"; then
    echo "unbreakable treasury RFC is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$UNBREAKABLE_TREASURY_RFC"; then
  echo "unbreakable treasury RFC should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_autoformalization_plan_required_patterns=(
  'always(!<+X> true | eventually(<+Y> true))'
  'always(!<+FINALIZE> true | eventually(<+COMPLETE_AUTHORIZATION> true))'
  'always(!<+FINALIZE> true | <+FINALIZE +signed_by(/users/account_holder.id)> true)'
  'always(!<+REVOKE> true | always([-USE_CERTIFICATE] true))'
)

for pattern in "${ietf_autoformalization_plan_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_AUTOFORMALIZATION_PLAN"; then
    echo "IETF autoformalization plan is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_AUTOFORMALIZATION_PLAN"; then
  echo "IETF autoformalization plan should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

rfc_0001_required_patterns=(
  "!<+RELEASE> true | <+DELIVER> true"
  "explicit Boolean form for the conditional"
)

for pattern in "${rfc_0001_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$RFC_0001_RESOURCE"; then
    echo "RFC-0001 resource is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$RFC_0001_RESOURCE"; then
  echo "RFC-0001 resource should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

standard_predicates_required_patterns=(
  "always(!<+modifies(/members)> true | <+modifies(/members) +all_signed(/members)> true)"
)

for pattern in "${standard_predicates_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$STANDARD_PREDICATES_DOC"; then
    echo "standard predicates reference is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$STANDARD_PREDICATES_DOC"; then
  echo "standard predicates reference should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

echo "language traps doc check passed"
