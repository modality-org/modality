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
LLM_SYNTHESIS_GUIDE="$ROOT_DIR/experiments/llm-synthesizer/SYNTHESIS_GUIDE.md"
LLM_RULE_GENERATION_DOC="$ROOT_DIR/experiments/llm-synthesizer/rule-generation.md"
LLM_MODEL_SYNTHESIS_DOC="$ROOT_DIR/experiments/llm-synthesizer/model-synthesis.md"
LLM_PREDICATE_DESIGN_DOC="$ROOT_DIR/experiments/llm-synthesizer/predicate-design.md"
LLM_PIPELINE_DOC="$ROOT_DIR/experiments/llm-synthesizer/pipeline.md"
LLM_EXPERIMENT_LOG="$ROOT_DIR/experiments/llm-synthesizer/experiment-log.md"
LLM_ESCROW_PIPELINE_EXAMPLE="$ROOT_DIR/experiments/llm-synthesizer/examples/escrow_pipeline.md"
SYNTHESIS_V1_EXPERIMENT="$ROOT_DIR/experiments/synthesis-v1.md"
IETF_SCIM_STUB="$ROOT_DIR/experiments/ietf-autoformalization/rfc7644-scim/rules.modality.stub"
IETF_TOKEN_EXCHANGE_STUB="$ROOT_DIR/experiments/ietf-autoformalization/rfc8693-token-exchange/rules.modality.stub"
IETF_DEVICE_AUTHORIZATION_STUB="$ROOT_DIR/experiments/ietf-autoformalization/rfc8628-device-authorization/rules.modality.stub"
IETF_RATS_STUB="$ROOT_DIR/experiments/ietf-autoformalization/rfc9334-rats/rules.modality.stub"
IETF_HTTP_MESSAGE_SIGNATURES_STUB="$ROOT_DIR/experiments/ietf-autoformalization/rfc9421-http-message-signatures/rules.modality.stub"
AGENT_COOPERATION_ROADMAP="$ROOT_DIR/ROADMAP-AGENT-COOPERATION.md"

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

llm_synthesis_guide_required_patterns=(
  "Status: archived experiment notes."
  "formula implication sugar such as \`A -> B\`"
  "use explicit Boolean conditionals"
  "always(!<+RELEASE> true | eventually(<+DELIVER> true))"
  "\`!<+A> true | Q\` — if action A happens, Q must hold"
)

for pattern in "${llm_synthesis_guide_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_SYNTHESIS_GUIDE"; then
    echo "LLM synthesis guide is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_SYNTHESIS_GUIDE"; then
  echo "LLM synthesis guide should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

llm_rule_generation_required_patterns=(
  "Archived experiment notes for translating natural-language contract requirements"
  "Current onboarding examples avoid formula implication sugar."
  "conditionals: \`!A | B\`."
  "avoid \`[+ACTION] true\` as an antecedent"
  "always(!<+RELEASE> true | eventually(<+DELIVER> true))"
  "always(!<+RELEASE> true | <+RELEASE +signed_by(/users/alice.id)> true)"
  "always(!<+DELEGATE> true | <+DELEGATE +signed_by(/users/principal.id)> true)"
  "Do not use \`->\`, \`implies\`, or \`[+A] true\`"
)

for pattern in "${llm_rule_generation_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_RULE_GENERATION_DOC"; then
    echo "LLM rule-generation notes are missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_RULE_GENERATION_DOC"; then
  echo "LLM rule-generation notes should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

llm_model_synthesis_required_patterns=(
  "Status: archived experiment notes."
  "Current teaching examples avoid formula"
  "use explicit Boolean conditionals"
  "avoid \`[+ACTION] true\` as a"
  "always(!<+RELEASE> true | eventually(<+DELIVER> true))"
  "!<+RELEASE> true | <+RELEASE +signed_by(/users/alice.id)> true"
  "!<+EXECUTE> true | <+EXECUTE +threshold(2, /signers)> true"
  "!<+RELEASE> true | <+RELEASE +oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true"
)

for pattern in "${llm_model_synthesis_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_MODEL_SYNTHESIS_DOC"; then
    echo "LLM model-synthesis notes are missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_MODEL_SYNTHESIS_DOC"; then
  echo "LLM model-synthesis notes should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

llm_predicate_design_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula"
  "use explicit Boolean"
  "avoid \`[+ACTION] true\` as a guard"
  "always(!<+DELIVER> true | <+DELIVER +before(/state/deadline.datetime)> true)"
  "always(!<+PAY> true | <+PAY +amount_gte(/state/minimum.json)> true)"
  "always(!<+EXECUTE> true | <+EXECUTE +threshold(\"2\", /users)> true)"
  "always(!<+RELEASE> true | <+RELEASE +oracle_attests(/oracles/delivery.id, \"delivered\", \"true\")> true)"
)

for pattern in "${llm_predicate_design_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_PREDICATE_DESIGN_DOC"; then
    echo "LLM predicate-design notes are missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_PREDICATE_DESIGN_DOC"; then
  echo "LLM predicate-design notes should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

llm_pipeline_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "conditionals such as \`!<+ACTION> true | REQUIRED_FORMULA\`"
  "F1: always(!<+RELEASE> true | eventually(<+DELIVER> true))"
  "F2: always(!<+RELEASE> true |"
  "<+RELEASE +signed_by(/users/alice.id)> true)"
)

for pattern in "${llm_pipeline_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_PIPELINE_DOC"; then
    echo "LLM pipeline notes are missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_PIPELINE_DOC"; then
  echo "LLM pipeline notes should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

llm_experiment_log_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula"
  "use explicit Boolean conditionals"
  "avoid \`[+ACTION] true\` as a guard"
  "always(!<+RELEASE> true | <+DELIVER> true)"
  "always(!<+REVEAL_A> true | ("
  "always(!<+DELEGATE> true | <+DELEGATE +signed_by(/users/agent_a.id)> true)"
  "<+EXECUTE +signed_by(/users/member1.id) +signed_by(/users/member2.id)> true"
  "always(!<+PAY_DESIGN> true | <+DELIVER_DESIGN> true)"
)

for pattern in "${llm_experiment_log_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_EXPERIMENT_LOG"; then
    echo "LLM experiment log is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_EXPERIMENT_LOG"; then
  echo "LLM experiment log should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

llm_escrow_pipeline_example_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "F1: always(!<+RELEASE> true | eventually(<+DELIVER> true))"
  "F3: always(!<+DEPOSIT> true | <+DEPOSIT +signed_by(/users/alice.id)> true)"
  "F4: always(!<+DELIVER> true | <+DELIVER +signed_by(/users/bob.id)> true)"
  "F5: always(!<+RELEASE> true | <+RELEASE +signed_by(/users/alice.id)> true)"
  "Historic ordering approximation"
  "same-transition signature evidence"
)

for pattern in "${llm_escrow_pipeline_example_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$LLM_ESCROW_PIPELINE_EXAMPLE"; then
    echo "LLM escrow pipeline example is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$LLM_ESCROW_PIPELINE_EXAMPLE"; then
  echo "LLM escrow pipeline example should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

synthesis_v1_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "use explicit Boolean conditionals"
  "avoid \`[+ACTION] true\` as a"
  "always(!<+DO_X> true | <+DO_X +signed_by(/users/alice.id)> true)"
  "always(!<+RECEIVED_PAYMENT> true | always([<+DELIVER>] true))"
)

for pattern in "${synthesis_v1_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$SYNTHESIS_V1_EXPERIMENT"; then
    echo "synthesis v1 experiment is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$SYNTHESIS_V1_EXPERIMENT"; then
  echo "synthesis v1 experiment should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_scim_stub_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "explicit Boolean conditionals such as \`!A | B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "always(!<+UPDATE_USER> true | eventually(<+CREATE_USER> true))"
  "always(!<+CREATE_USER> true | <+CREATE_USER +signed_by(/users/identity_provider.id)> true)"
  "always(!<+DELETE_USER> true | eventually(<+DEPROVISION_USER> true))"
  "always(!<+DELETE_USER> true | <+DELETE_USER +signed_by(/users/provisioning_admin.id)> true)"
  "always(!<+SUSPEND_USER> true | always([-REACTIVATE_USER] true))"
)

for pattern in "${ietf_scim_stub_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_SCIM_STUB"; then
    echo "IETF SCIM stub is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_SCIM_STUB"; then
  echo "IETF SCIM stub should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_token_exchange_stub_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "explicit Boolean conditionals such as \`!A | B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "always(!<+ISSUE_EXCHANGED_TOKEN> true | eventually(<+VALIDATE_EXCHANGE> true))"
  "always(!<+ISSUE_EXCHANGED_TOKEN> true | <+ISSUE_EXCHANGED_TOKEN +signed_by(/users/authorization_server.id)> true)"
  "always(!<+ISSUE_EXCHANGED_TOKEN> true | eventually(<+DELEGATE> true))"
  "always(!<+DENY_EXCHANGE> true | always([-ISSUE_EXCHANGED_TOKEN] true))"
  "always(!<+DELEGATE> true | <+DELEGATE +signed_by(/users/subject.id)> true)"
)

for pattern in "${ietf_token_exchange_stub_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_TOKEN_EXCHANGE_STUB"; then
    echo "IETF token exchange stub is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_TOKEN_EXCHANGE_STUB"; then
  echo "IETF token exchange stub should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_device_authorization_stub_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "explicit Boolean conditionals such as \`!A | B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "always(!<+ISSUE_TOKEN> true | eventually(<+AUTHORIZE> true))"
  "always(!<+ISSUE_TOKEN> true | <+ISSUE_TOKEN +signed_by(/users/authorization_server.id)> true)"
  "always(!<+DENY> true | always([-ISSUE_TOKEN] true))"
  "always(!<+EXPIRE> true | always([-ISSUE_TOKEN] true))"
  "always(!<+POLL_TOKEN> true | eventually(<+REQUEST_DEVICE_CODE> true))"
)

for pattern in "${ietf_device_authorization_stub_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_DEVICE_AUTHORIZATION_STUB"; then
    echo "IETF device authorization stub is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_DEVICE_AUTHORIZATION_STUB"; then
  echo "IETF device authorization stub should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_rats_stub_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "explicit Boolean conditionals such as \`!A | B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "always(!<+APPRAISE> true | eventually(<+SUBMIT_EVIDENCE> true))"
  "always(!<+GRANT_ACCESS> true | eventually(<+ATTEST_PASS> true))"
  "always(!<+GRANT_ACCESS> true | <+GRANT_ACCESS +oracle_attests(/oracles/verifier.id, \"appraisal\", \"pass\")> true)"
  "always(!<+ATTEST_FAIL> true | always([-GRANT_ACCESS] true))"
  "always(!<+SUBMIT_EVIDENCE> true | <+SUBMIT_EVIDENCE +signed_by(/users/attester.id)> true)"
)

for pattern in "${ietf_rats_stub_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_RATS_STUB"; then
    echo "IETF RATS stub is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_RATS_STUB"; then
  echo "IETF RATS stub should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

ietf_http_message_signatures_stub_required_patterns=(
  "Status: archived experiment notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "explicit Boolean conditionals such as \`!A | B\`"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "always(!<+ACCEPT_REQUEST> true | eventually(<+VERIFY_SIGNATURE> true))"
  "always(!<+ACCEPT_REQUEST> true | eventually(<+SIGN_REQUEST> true))"
  "always(!<+SIGN_REQUEST> true | <+SIGN_REQUEST +signed_by(/users/signer.id)> true)"
  "always(!<+ROTATE_SIGNING_KEY> true | <+ROTATE_SIGNING_KEY +signed_by(/users/policy_authority.id)> true)"
  "always(!<+REJECT_REQUEST> true | always([-ACCEPT_REQUEST] true))"
)

for pattern in "${ietf_http_message_signatures_stub_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$IETF_HTTP_MESSAGE_SIGNATURES_STUB"; then
    echo "IETF HTTP message signatures stub is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$IETF_HTTP_MESSAGE_SIGNATURES_STUB"; then
  echo "IETF HTTP message signatures stub should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

agent_cooperation_roadmap_required_patterns=(
  "Status: archived planning notes."
  "avoid formula implication sugar such as \`A -> B\`"
  "explicit Boolean conditionals such as"
  "avoid \`[+ACTION] true\` as a conditional antecedent"
  "always(!<+APPROVE> true | <+APPROVE +signed_by(/users/alice.id)> true)"
  "always(!<+A> true | <+A +signed_by(/users/alice.id)> true)"
  "always(!<+COOPERATE> true | <+COOPERATE> true)"
)

for pattern in "${agent_cooperation_roadmap_required_patterns[@]}"; do
  if ! grep -Fq "$pattern" "$AGENT_COOPERATION_ROADMAP"; then
    echo "agent cooperation roadmap is missing language-trap text: $pattern" >&2
    exit 1
  fi
done

if grep -Eq -- ' true[[:space:]]*->| implies ' "$AGENT_COOPERATION_ROADMAP"; then
  echo "agent cooperation roadmap should not present formula implication sugar as the teaching path" >&2
  exit 1
fi

echo "language traps doc check passed"
