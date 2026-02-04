use modality_lang::parse_file_lalrpop;

fn main() {
    let path = std::env::args().nth(1).expect("Usage: parse_example <file>");
    let model = parse_file_lalrpop(&path).expect("Failed to parse file");
    println!("Parsed model: {:#?}", model);
} 