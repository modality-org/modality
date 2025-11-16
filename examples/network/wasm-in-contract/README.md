# WASM Validation Example

This example demonstrates uploading and using WASM modules for contract validation.

## Overview

This example shows how to:
1. Create a contract
2. Upload a WASM validation module
3. Push the contract to the network
4. Use the WASM module for validation

## Files

- `00-setup.sh` - Set up the example environment
- `01-create-contract.sh` - Create a new contract
- `02-upload-wasm.sh` - Upload WASM validation module
- `03-push-contract.sh` - Push to network
- `04-test-validation.sh` - Test the validation
- `validator.wasm` - Example WASM validation module (if available)
- `README.md` - This file

## Prerequisites

- Modal CLI installed
- Network running (see `examples/network/03-run-devnet3/`)
- Node with network access

## Running the Example

```bash
# Run all steps in order
./00-setup.sh
./01-create-contract.sh
./02-upload-wasm.sh
./03-push-contract.sh
./04-test-validation.sh
```

Or run the test script:
```bash
cd ../../
./test-numbered-examples.sh 10-wasm-validation
```

## What This Example Demonstrates

1. **WASM Upload via POST**: Shows how WASM modules are uploaded as POST actions with `.wasm` extension
2. **Gas Limits**: Demonstrates setting custom gas limits for execution
3. **Built-in Validation**: Uses built-in validators from modal-wasm-validation
4. **Cross-Platform**: The same WASM runs identically on all nodes

## Expected Output

After running the example:
- Contract created with ID
- WASM module uploaded and stored
- Network consensus validates the WASM
- Validation tests pass with deterministic results

## Troubleshooting

**WASM validation fails**: Ensure the WASM module is valid WebAssembly format

**Gas limit exceeded**: Increase the gas limit in step 2

**Module not found**: Check that consensus has processed the upload commit

## Next Steps

- Try uploading your own custom WASM validator
- Experiment with different gas limits
- Use the JavaScript SDK to interact with WASM validators

