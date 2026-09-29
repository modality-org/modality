fn main() {
    let mut args = std::env::args().skip(1);
    let actions = args.next().expect("actions JSON array");
    let out = args.next().expect("output wasm path");
    let actions: serde_json::Value = serde_json::from_str(&actions).expect("actions JSON");
    let bytes =
        modality_wasm_runtime::program_that_emits(&actions).expect("compile fixture program");
    std::fs::write(&out, bytes).expect("write wasm");
}
