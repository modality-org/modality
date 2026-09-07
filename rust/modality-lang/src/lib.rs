#![allow(unexpected_cfgs)]
// Allow unused imports in generated grammar module
#![allow(unused_imports)]
// Allow empty line after outer attribute in generated grammar
#![allow(clippy::empty_line_after_outer_attr)]

pub mod agent;
pub mod ast;
pub mod contract_log;
pub mod crypto;
pub mod evolution;
pub mod formula_lint;
pub mod formula_synthesis;
pub mod lalrpop_parser;
pub mod lexer;
pub mod llm_synthesis;
pub mod mermaid;
pub mod model_checker;
pub mod nl_mapper;
pub mod paths;
pub mod patterns;
pub mod printer;
pub mod runtime;
pub mod synthesis;
pub mod validation;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Include the generated parser
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);

pub use ast::{
    Action, ActionCall, Formula, FormulaExpr, Model, Part, PartState, Property, PropertySign,
    PropertySource, Test, TestStatement, Transition,
};
pub use contract_log::{Action as CommitAction, Commit, ContractLog, DerivedState};
pub use crypto::{generate_keypair, sha256, sign_ed25519, verify_ed25519, VerifyResult};
pub use evolution::{
    Amendment, Approval, EvolutionRecord, EvolvableContract, Proposal, ProposalStatus,
};
pub use formula_lint::{
    find_span_in_source, lint_formula, lint_formula_with_source, lint_formulas_in_content,
    witness_node_names, FormulaLintDiagnostic, FormulaLintOptions, LintCode, LintSeverity,
    LintSpan,
};
pub use lalrpop_parser::{
    parse_action_call_lalrpop, parse_all_actions_content_lalrpop, parse_all_actions_lalrpop,
    parse_all_formulas_content_lalrpop, parse_all_models_content_lalrpop, parse_all_models_lalrpop,
    parse_all_tests_content_lalrpop, parse_all_tests_lalrpop, parse_content_lalrpop,
    parse_file_lalrpop,
};
pub use mermaid::{
    generate_mermaid_diagram, generate_mermaid_diagram_with_state,
    generate_mermaid_diagram_with_styling, generate_mermaid_diagrams,
};
pub use model_checker::{ModelCheckResult, ModelChecker, State};
pub use printer::print_model;
pub use runtime::negotiation::{
    CounterProposal, Proposal as NegotiationProposal, ProposalStatus as NegotiationStatus,
};
pub use runtime::{
    ActionBuilder, AvailableTransition, CommitRecord, ContractInstance, ContractState,
    RuntimeError, RuntimeResult, SignedAction,
};
pub use synthesis::{
    identify_pattern, synthesize, synthesize_from_pattern, RulePattern, SynthesisResult,
};
pub use validation::{
    plus_sets_path_value, sets_path_value, suggest_predicate, validate_no_raw_propositions,
    validate_sets_one_of, ValidationError, KNOWN_PREDICATES,
};

// Re-export the generated parser
pub use grammar::ActionCallParser;
pub use grammar::ActionParser;
pub use grammar::FormulaParser;
pub use grammar::ModelParser;
pub use grammar::TestParser;
pub use grammar::TopLevelParser;
