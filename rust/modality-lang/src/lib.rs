pub mod ast;
pub mod lexer;
pub mod lalrpop_parser;
pub mod mermaid;
pub mod wasm;
pub mod model_checker;

// Include the generated parser
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);

pub use lalrpop_parser::{parse_file_lalrpop, parse_content_lalrpop, parse_all_models_lalrpop, parse_all_models_content_lalrpop, parse_all_formulas_content_lalrpop, parse_all_actions_lalrpop, parse_all_actions_content_lalrpop, parse_action_call_lalrpop, parse_all_tests_lalrpop, parse_all_tests_content_lalrpop};
pub use ast::{Model, Part, Transition, Property, PropertySign, Formula, FormulaExpr, PartState, Action, ActionCall, Test, TestStatement};
pub use mermaid::{generate_mermaid_diagram, generate_mermaid_diagrams, generate_mermaid_diagram_with_styling, generate_mermaid_diagram_with_state};
pub use model_checker::{ModelChecker, State, ModelCheckResult};

// Re-export the generated parser
pub use grammar::ModelParser;
pub use grammar::FormulaParser;
pub use grammar::ActionParser;
pub use grammar::ActionCallParser;
pub use grammar::TestParser;
pub use grammar::TopLevelParser; 