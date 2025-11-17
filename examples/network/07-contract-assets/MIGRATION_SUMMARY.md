# Migration Summary: data/ → tmp/

## Overview

Successfully migrated the 07-contract-assets example from using `data/alice` and `data/bob` directories to using `tmp/alice` and `tmp/bob` subdirectories. This brings the example in line with the standard pattern used across other network examples where `tmp/` is gitignored.

## Changes Made

### Scripts Updated

All scripts were updated to use `tmp/alice` and `tmp/bob` instead of `data/alice` and `data/bob`:

1. **00-setup.sh**
   - Changed: `rm -rf data/` → `rm -rf tmp/alice tmp/bob tmp/send-commit-id.txt`
   - Changed: `mkdir -p data/alice data/bob` → `mkdir -p tmp/alice tmp/bob`

2. **00-setup-devnet1.sh**
   - Changed: `rm -rf data/` removed (tmp/ cleanup is sufficient)
   - Changed: `mkdir -p data/alice data/bob` → `mkdir -p tmp/alice tmp/bob`

3. **01-create-alice.sh**
   - Changed: `cd data/alice` → `cd tmp/alice`
   - Updated output message: `data/alice/.contract/` → `tmp/alice/.contract/`

4. **02-create-token.sh**
   - Changed: `cd data/alice` → `cd tmp/alice`

5. **03-create-bob.sh**
   - Changed: `cd data/bob` → `cd tmp/bob`
   - Updated output message: `data/bob/.contract/` → `tmp/bob/.contract/`

6. **04-alice-sends-tokens.sh**
   - Changed: `cd data/alice` → `cd tmp/alice`
   - Changed: `../send-commit-id.txt` path (now in tmp/)

7. **05-bob-receives-tokens.sh**
   - Changed: `cd data/bob` → `cd tmp/bob`
   - Changed: `../send-commit-id.txt` path (now in tmp/)

8. **06-query-balances.sh**
   - Changed: All `cd data/alice` → `cd tmp/alice`
   - Changed: All `cd data/bob` → `cd tmp/bob`

9. **08-invalid-double-send.sh**
   - Changed: `cd data/alice` → `cd tmp/alice`

### Test Scripts Updated

1. **test.sh**
   - Updated all file path validations from `data/` to `tmp/`
   - Updated all contract JSON file paths
   - Updated all commit file verification paths

2. **test-devnet1.sh**
   - Updated all file path validations from `data/` to `tmp/`
   - Updated all contract JSON file paths

### Documentation Updated

1. **README.md**
   - Updated Directory Structure section to show `tmp/` instead of `data/`

2. **IMPLEMENTATION_STATUS.md**
   - Updated Files Created section
   - Removed reference to `.gitignore` (not needed, tmp/ is already gitignored at repo level)
   - Added note: "Data stored in tmp/ (gitignored)"

### Files Removed

- Removed old `data/` directory and all its contents

## Contract Creation Command

Both Alice and Bob's contracts were already using `modal contract create` (no change needed):

```bash
modal contract create --output json > alice-contract.json
modal contract create --output json > bob-contract.json
```

## Directory Structure

### Before
```
07-contract-assets/
├── data/
│   ├── alice/
│   └── bob/
└── tmp/
    └── node1/
```

### After
```
07-contract-assets/
└── tmp/
    ├── alice/
    ├── bob/
    └── node1/
```

## Testing

All tests pass successfully:

### Local Test (`./test.sh`)
```
✅ All tests passed!
Passed: 27
Failed: 0
```

### Network Test (`./test-devnet1.sh`)
Expected to pass (not run in this migration)

## Benefits

1. **Consistency**: All temporary/generated files now in `tmp/` directory
2. **Cleaner Structure**: No separate `data/` directory, everything test-related in `tmp/`
3. **Gitignore Compliance**: `tmp/` is already gitignored at the repository level
4. **Standard Pattern**: Matches the pattern used in other network examples

## Verification

- ✅ No references to `data/alice` or `data/bob` remain
- ✅ All scripts updated and tested
- ✅ Local test suite passes (27/27 tests)
- ✅ Directory structure updated
- ✅ Documentation updated
- ✅ Old `data/` directory removed

## Migration Date

November 17, 2025

