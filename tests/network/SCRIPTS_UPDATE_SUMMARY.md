# Examples Network Scripts Updated to Use `modal`

All network example scripts have been updated to use the `modal` CLI command instead of `modality`.

## Updated Scripts

### 01-ping-node/
- ✅ `01-run-node1.sh` - Changed `modality node run` to `modal node run`
- ✅ `02-ping-node1-from-node2.sh` - Changed `modality node ping` to `modal node ping`

### 02-run-devnet2/
- ✅ `01-run-node1.sh` - **Note: Uses `modality-js` (not changed, different CLI)**
- ✅ `02-run-node2.sh` - Changed `modality node run` to `modal node run`

### 03-run-devnet3/
- ✅ `01-run-node1.sh` - Changed `modality node run` to `modal node run`
- ✅ `02-run-node2.sh` - Changed `modality node run` to `modal node run`
- ✅ `03-run-node3.sh` - Changed `modality node run` to `modal node run`

### 04-sync-miner-blocks/
- ✅ `01-run-node1.sh` - Changed `modality node run` to `modal node run`
- ✅ `03-sync-all-blocks.sh` - Changed `modality net mining sync` to `modal net mining sync`
- ✅ `04-sync-epoch.sh` - Changed `modality net mining sync` to `modal net mining sync`
- ✅ `05-sync-range.sh` - Changed `modality net mining sync` to `modal net mining sync`
- ✅ `06-view-blocks-json.sh` - Changed `modality net mining sync` to `modal net mining sync`
- ✅ `07-inspect-storage.sh` - Changed `modality net storage` to `modal net storage`

### 05-mining/
- ✅ `01-mine-blocks.sh` - Changed `modality node run-miner` to `modal node run-miner`
- ✅ `02-inspect-blocks.sh` - Changed `modality net storage` to `modal net storage`
- ✅ `03-view-difficulty-progression.sh` - Build check updated to `modal`
- ✅ `05-test-divergent-chains.sh` - All instances updated to `modal`

## Commands Updated

| Old Command | New Command |
|-------------|-------------|
| `modality node run` | `modal node run` |
| `modality node ping` | `modal node ping` |
| `modality node run-miner` | `modal node run-miner` |
| `modality net mining sync` | `modal net mining sync` |
| `modality net storage` | `modal net storage` |

## Note on `modality-js`

The command `modality-js` was NOT changed as it refers to a separate JavaScript-based CLI tool:
- `examples/network/02-run-devnet2/01-run-node1.sh` uses `modality-js net run-node` (unchanged)

## Summary

✅ **14 files updated** across 5 example directories  
✅ All Rust-based `modality` commands changed to `modal`  
✅ JavaScript-based `modality-js` commands left unchanged  
✅ No remaining instances of standalone `modality` command in scripts

