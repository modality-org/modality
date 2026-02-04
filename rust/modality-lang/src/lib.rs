#![allow(unexpected_cfgs)]
// Allow unused imports in generated grammar module
#![allow(unused_imports)]
// Allow empty line after outer attribute in generated grammar
#![allow(clippy::empty_line_after_outer_attr)]

pub mod ast;
pub mod lexer;
pub mod lalrpop_parser;
pub mod mermaid;
pub mod wasm;
pub mod model_checker;
pub mod synthesis;
pub mod printer;
pub mod evolution;
pub mod runtime;
pub mod agent;
pub mod patterns;
pub mod paths;
pub mod crypto;
pub mod contract_log;
pub mod nl_mapper;
pub mod formula_synthesis;
pub mod llm_synthesis;
pub mod validation;

// Include the generated parser
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);

pub use lalrpop_parser::{parse_file_lalrpop, parse_content_lalrpop, parse_all_models_lalrpop, parse_all_models_content_lalrpop, parse_all_formulas_content_lalrpop, parse_all_actions_lalrpop, parse_all_actions_content_lalrpop, parse_action_call_lalrpop, parse_all_tests_lalrpop, parse_all_tests_content_lalrpop};
pub use ast::{Model, Part, Transition, Property, PropertySign, PropertySource, Formula, FormulaExpr, PartState, Action, ActionCall, Test, TestStatement};
pub use mermaid::{generate_mermaid_diagram, generate_mermaid_diagrams, generate_mermaid_diagram_with_styling, generate_mermaid_diagram_with_state};
pub use model_checker::{ModelChecker, State, ModelCheckResult};
pub use synthesis::{synthesize, synthesize_from_pattern, identify_pattern, SynthesisResult, RulePattern};
pub use printer::print_model;
pub use evolution::{EvolvableContract, Amendment, Proposal, ProposalStatus, Approval, EvolutionRecord};
pub use runtime::{ContractInstance, SignedAction, CommitRecord, ContractState, ActionBuilder, RuntimeError, RuntimeResult, AvailableTransition};
pub use runtime::negotiation::{Proposal as NegotiationProposal, CounterProposal, ProposalStatus as NegotiationStatus};
pub use crypto::{verify_ed25519, sign_ed25519, generate_keypair, sha256, VerifyResult};
pub use contract_log::{ContractLog, Commit, Action as CommitAction, DerivedState};

// Re-export the generated parser
pub use grammar::ModelParser;
pub use grammar::FormulaParser;
pub use grammar::ActionParser;
pub use grammar::ActionCallParser;
pub use grammar::TestParser;
pub use grammar::TopLevelParser; 