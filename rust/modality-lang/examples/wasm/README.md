# WASM Examples

This directory contains examples demonstrating how to use the Modality Language Parser compiled to WebAssembly.

## Files

- **`example.html`** - Browser-based demo showing how to use the WASM parser in a web page
- **`node-example.cjs`** - Node.js example showing how to use the WASM parser in a Node.js environment
- **`README.md`** - Detailed documentation for the WASM package

## How to Use

### Browser Example

1. Build the WASM module:
   ```bash
   wasm-pack build --target web --out-dir ../../dist
   ```

2. Copy the built files to the examples directory:
   ```bash
   cp ../../dist/modality_lang.js ../../dist/modality_lang_bg.wasm .
   ```

3. Open `example.html` in a web browser

### Node.js Example

1. Build the WASM module for Node.js:
   ```bash
   wasm-pack build --target nodejs --out-dir ../../dist-node
   ```

2. Copy the built files to the examples directory:
   ```bash
   cp ../../dist-node/modality_lang.js ../../dist-node/modality_lang_bg.wasm .
   ```

3. Run the example:
   ```bash
   node node-example.cjs
   ```

## Notes

- The example files reference the WASM modules from the `dist` and `dist-node` directories
- You may need to adjust the import paths in the examples based on your setup
- The WASM modules are not included in this directory as they are build artifacts
