//! Run a program's `execute` on one input under a gas limit and print its
//! output: `run_program <program.wasm> <input.json> [gas_limit]`.

fn main() {
    let mut args = std::env::args().skip(1);
    let wasm = args.next().expect("program wasm path");
    let input = args.next().expect("input JSON path");
    let gas_limit = args
        .next()
        .map(|g| g.parse().expect("gas limit"))
        .unwrap_or(modality_wasm_runtime::DEFAULT_GAS_LIMIT);
    let wasm = std::fs::read(&wasm).expect("read wasm");
    let input = std::fs::read_to_string(&input).expect("read input");
    let mut executor = modality_wasm_runtime::WasmExecutor::new(gas_limit);
    match executor.execute(&wasm, "execute", &input) {
        Ok(out) => println!("{out}"),
        Err(err) => {
            eprintln!("{err:#}");
            std::process::exit(1);
        }
    }
}
