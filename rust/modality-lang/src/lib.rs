pub mod parser;
pub mod ast;
pub mod lexer;
pub mod lalrpop_parser;
pub mod mermaid;

// Include the generated parser
use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub grammar);

pub use parser::parse_file;
pub use lalrpop_parser::{parse_file_lalrpop, parse_content_lalrpop, parse_all_models_lalrpop, parse_all_models_content_lalrpop};
pub use ast::{Model, Graph, Transition, Property, PropertySign};
pub use mermaid::{generate_mermaid_diagram, generate_mermaid_diagrams, generate_mermaid_diagram_with_styling};

// Re-export the generated parser
pub use grammar::ModelParser; 