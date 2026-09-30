# WASM Examples

Examples of the Modality language parser and model checker compiled to
WebAssembly.

## Files

- **`example.html`** - Browser demo: parse a model and draw it as Mermaid
- **`node-example.cjs`** - Node.js demo: parse, draw, and check a formula

The WASM bindings are build artifacts and are not checked in. Build them
from `rust/modality-lang`.

## Browser

```bash
cd rust/modality-lang
wasm-pack build --target web --out-dir dist
python3 -m http.server 8000
```

Then open <http://localhost:8000/examples/wasm/example.html>. The page loads
`../../dist/modality_lang.js`, so it must be served over HTTP, not opened as
a file.

## Node.js

```bash
cd rust/modality-lang
wasm-pack build --target nodejs --out-dir dist-node
node examples/wasm/node-example.cjs
```

## Notes

- `parse_model`, `parse_all_models` and `parse_formulas` return objects.
  `generate_mermaid*` and `check_formula*` take models and formulas as JSON
  text, so pass `JSON.stringify(model)`.
- A model file does not say where a run is. To draw current nodes, set
  `state` on the parsed model (`[{ part_name, current_nodes }]`) before
  `generate_mermaid_with_state`.
