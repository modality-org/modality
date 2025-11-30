# Finalized Rounds Section - Status Page

## Overview

A new section has been added to the Validators tab of the status page that displays the status of recently finalized rounds. This provides visibility into the consensus finalization process and helps monitor the health of the validator network.

## Feature Description

### Location

The "Recently Finalized Rounds" section appears on the **Validators tab** of the status page, positioned above the existing Epoch Nominees sections.

### Information Displayed

For each of the last 10 rounds (excluding the current round), the table shows:

| Column | Description |
|--------|-------------|
| **Round** | The round number |
| **Certified Blocks** | Number of blocks with certificates (finalized) |
| **Total Blocks** | Total number of blocks submitted in that round |
| **Completion %** | Percentage of blocks that are certified |
| **Status** | Visual indicator of round finalization state |

### Status Indicators

The status column uses color coding to quickly convey the finalization state:

- 🟢 **Finalized** (Green) - ≥67% of blocks are certified
  - This is the 2/3 consensus threshold (Byzantine fault tolerance)
  - Indicates the round has reached finality
  
- 🟡 **Partial** (Yellow) - >0% but <67% of blocks are certified
  - Some blocks are certified but consensus not yet reached
  - May indicate ongoing finalization or network issues
  
- ⚪ **In Progress** (Gray) - 0% of blocks are certified
  - No blocks have been certified yet
  - Expected for very recent rounds

## Implementation Details

### Code Changes

1. **Template Updates** (`rust/modal-node/src/templates/status.html`)
   - Added `{finalized_rounds_section}` placeholder to the Validators tab

2. **Template Functions** (`rust/modal-node/src/templates/mod.rs`)
   - `render_finalized_rounds_section()` - Renders the entire section with table structure
   - `render_finalized_round_row()` - Renders individual round rows with data
   - `render_empty_finalized_rounds()` - Renders message when no rounds exist
   - Updated `StatusPageVars` struct to include `finalized_rounds_section` field

3. **Status Server Logic** (`rust/modal-node/src/status_server.rs`)
   - `calculate_finalized_rounds()` - Queries validator blocks from the datastore
   - `build_finalized_rounds_html()` - Constructs HTML from round data
   - Integrated into `generate_status_html()` function

### Data Flow

```
generate_status_html()
    ↓
calculate_finalized_rounds()
    ↓ (queries ValidatorBlock::find_all_in_round_multi)
DatastoreManager (ValidatorActive + ValidatorFinal stores)
    ↓
RoundFinalizationData (round_id, certified_count, total_count)
    ↓
build_finalized_rounds_html()
    ↓ (calculates completion %, determines status)
render_finalized_rounds_section()
    ↓
HTML string injected into template
```

### Database Queries

The feature uses the multi-store query system to retrieve validator blocks:

- **ValidatorBlock::find_all_in_round_multi()** - Retrieves all blocks for a specific round
  - Merges data from ValidatorActive (recent) and ValidatorFinal (older) stores
  - Returns blocks regardless of certification status
  
- Filters blocks where `cert.is_some()` to count certified blocks

## Usage

### Viewing the Section

1. Start a modal-node instance (miner or validator)
2. Access the status page at `http://localhost:<status_port>`
3. Click on the **Validators** tab
4. The "Recently Finalized Rounds" section will be displayed at the top

### Interpreting the Data

**Healthy Network:**
- Recent rounds should show "Finalized" status (green)
- Completion percentages consistently ≥67%
- Only the most recent 1-2 rounds might show "Partial" or "In Progress"

**Potential Issues:**
- Multiple consecutive rounds with "Partial" status
  - May indicate validator connectivity issues
  - Could signal insufficient validator participation
  
- Old rounds still showing "In Progress"
  - Suggests finalization stalled
  - Check validator logs for errors

**Normal Patterns:**
- The current round (not shown) is still in progress
- Round N-1 might be "Partial" as finalization completes
- Round N-2 and older should typically be "Finalized"

## Technical Considerations

### Performance

- The section queries the last 10 rounds worth of data
- Uses efficient multi-store iteration
- Minimal overhead compared to existing status page queries
- Refreshes every 10 seconds (configurable via `STATUS_PAGE_REFRESH_SECS`)

### Byzantine Fault Tolerance

The 67% threshold (2/3+1) is chosen based on Byzantine fault tolerance requirements:
- With n validators, the network can tolerate up to (n-1)/3 Byzantine failures
- 2/3+1 honest validators ensure safety and liveness
- This matches the consensus threshold used throughout the validator network

### Store Architecture

The feature leverages the dual-store architecture:
- **ValidatorActive** - Contains recent, in-progress rounds
- **ValidatorFinal** - Contains older, finalized rounds
- Multi-store queries seamlessly merge data from both

## Future Enhancements

Potential improvements to consider:

1. **Detailed Round View** - Click to see individual block details
2. **Historical Trends** - Graph showing finalization rate over time  
3. **Alert Thresholds** - Visual warnings for prolonged non-finalization
4. **Validator Participation** - Show which validators contributed to each round
5. **Finalization Time** - Display how long each round took to finalize

## Testing

The implementation includes test coverage:

- Template placeholder verification
- Status calculation logic
- Completion percentage boundaries
- Edge cases (empty rounds, zero certified blocks)

## Related Components

- **ValidatorBlock Model** (`modal-datastore/src/models/validator/block.rs`)
  - Stores individual validator blocks with certificates
  
- **Multi-Store Queries** (`modal-datastore/src/models/validator/multi_store.rs`)
  - Provides transparent querying across active and final stores
  
- **Status Page Tabs** (`templates/status.html`)
  - Organizes different views (Overview, Miners, Validators)

## Conclusion

The finalized rounds section provides essential visibility into the consensus finalization process, helping node operators and network administrators monitor the health and progress of the validator network. The status indicators make it easy to quickly assess whether rounds are finalizing as expected or if there are potential issues requiring attention.

