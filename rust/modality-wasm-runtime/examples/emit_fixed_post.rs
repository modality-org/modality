fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("post path");
    let value = args.next().expect("post value");
    let out = args.next().expect("output wasm path");
    let bytes =
        modality_wasm_runtime::program_that_posts(&path, &value).expect("compile fixture program");
    std::fs::write(&out, bytes).expect("write wasm");
}
