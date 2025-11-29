# modal contract commit-id Command Implementation

## Overview

Implemented a new `modal contract commit-id` command that outputs the commit ID from HEAD or with negative offsets to access parent commits. This eliminates the need to directly read `.contract/HEAD` files in shell scripts.

## Implementation

### Rust Module: `rust/modal/src/cmds/contract/commit_id.rs`

```rust
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::contract_store::ContractStore;

#[derive(Debug, Parser)]
#[command(about = "Get the commit ID (HEAD or with offset)")]
pub struct Opts {
    /// Directory path (defaults to current directory)
    #[clap(long)]
    dir: Option<PathBuf>,
    
    /// Offset from HEAD (e.g., -1 for parent, -2 for grandparent)
    #[clap(default_value = "0")]
    offset: i32,
}

pub async fn run(opts: &Opts) -> Result<()> {
    // Determine the contract directory
    let dir = if let Some(path) = &opts.dir {
        path.clone()
    } else {
        std::env::current_dir()?
    };

    // Open contract store
    let store = ContractStore::open(&dir)?;
    
    // Get HEAD commit
    let mut commit_id = store.get_head()?
        .ok_or_else(|| anyhow::anyhow!("No commits found in contract"))?;
    
    // Walk back through parents if offset is negative
    if opts.offset < 0 {
        for _ in 0..opts.offset.abs() {
            let commit = store.load_commit(&commit_id)?;
            commit_id = commit.head.parent
                .ok_or_else(|| anyhow::anyhow!("No parent commit found"))?;
        }
    } else if opts.offset > 0 {
        anyhow::bail!("Positive offsets are not supported. Use negative offsets (e.g., -1 for parent commit)");
    }
    
    // Output just the commit ID
    println!("{}", commit_id);

    Ok(())
}
```

## Usage

### Get HEAD commit
```bash
cd tmp/alice
modal contract commit-id
```

Output:
```
3ab6568beb7c68c5dcc597c17d905f07d80bcae4526801f34ec5b11f96ec7e15
```

### Get parent commit
```bash
modal contract commit-id -- -1
```

### Get grandparent commit
```bash
modal contract commit-id -- -2
```

### With directory flag
```bash
modal contract commit-id --dir tmp/alice
```

## Script Migrations

### Before
```bash
CREATE_COMMIT_ID=$(cat .contract/HEAD)
```

### After
```bash
CREATE_COMMIT_ID=$(modal contract commit-id)
```

## Benefits

1. **Cleaner**: No direct file system access
2. **Consistent API**: Uses modal CLI interface
3. **More Powerful**: Can access parent commits with offsets
4. **Error Handling**: Better error messages when commits don't exist
5. **Cross-Platform**: Works consistently across operating systems

## Updated Scripts

All scripts in `07-contract-assets` now use the new command:

1. **02-create-token.sh**: `CREATE_COMMIT_ID=$(modal contract commit-id)`
2. **04-alice-sends-tokens.sh**: `SEND_COMMIT_ID=$(modal contract commit-id)`
3. **05-bob-receives-tokens.sh**: `RECV_COMMIT_ID=$(modal contract commit-id)`

## Advanced Usage Examples

### Get the commit chain
```bash
# Current commit
echo "HEAD: $(modal contract commit-id)"

# Parent
echo "Parent: $(modal contract commit-id -- -1)"

# Grandparent
echo "Grandparent: $(modal contract commit-id -- -2)"
```

Output:
```
HEAD: 3ab6568beb7c68c5dcc597c17d905f07d80bcae4526801f34ec5b11f96ec7e15
Parent: d8d459ec620fcfe30661725843ee448c1f419444abd508cddc39d3adce2f079c
Grandparent: 0424e4ebf595fd727bb03b20cb8b3f2c3f0610d925c8c351fa756c0a8b004dc5
```

### Walk the commit history
```bash
#!/bin/bash
OFFSET=0
while true; do
    COMMIT=$(modal contract commit-id -- $OFFSET 2>/dev/null)
    if [ $? -ne 0 ]; then
        break
    fi
    echo "Commit $OFFSET: $COMMIT"
    OFFSET=$((OFFSET - 1))
done
```

## Testing

All tests pass successfully:

```bash
cd examples/network/07-contract-assets
./test.sh
```

Result:
```
✅ All tests passed!
Passed: 18
Failed: 0
```

## Implementation Details

### Commit Chain Walking

The command walks the commit chain by:
1. Loading the HEAD commit ID from `.contract/HEAD`
2. For each negative offset, loading the commit file
3. Reading the `commit.head.parent` field
4. Repeating until the desired offset is reached

### Error Handling

- Returns error if no commits exist
- Returns error if trying to walk past genesis (no parent)
- Returns error if positive offsets are used (not supported)
- Returns error if commit file is missing or corrupted

## Comparison with Direct File Access

### Lines of Code

**Before:**
```bash
COMMIT_ID=$(cat .contract/HEAD)
```

**After:**
```bash
COMMIT_ID=$(modal contract commit-id)
```

Same length, but more semantic and with better error handling.

### Accessing Parent Commits

**Before:** Not possible without manual parsing
```bash
# Would need to manually parse JSON
PARENT=$(cat .contract/commits/$(cat .contract/HEAD).json | jq -r '.head.parent')
```

**After:** Built-in support
```bash
PARENT=$(modal contract commit-id -- -1)
```

## Date

November 17, 2025

