use modality_lang::{parse_file_lalrpop, PropertySign};

fn main() {
    let path = std::env::args().nth(1).expect("Usage: compare_parsers <file>");
    let model = parse_file_lalrpop(&path).expect("Failed to parse file");
    println!("Parsed model: {:#?}", model);
} 