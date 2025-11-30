# Finalized Rounds Section - Visual Example

This document provides a visual representation of the "Recently Finalized Rounds" section on the Validators tab.

## Section Layout

```
╔══════════════════════════════════════════════════════════════════════════════╗
║                                Validators Tab                                 ║
╠══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐            ║
║  │ Current Epoch   │  │ Block Height    │  │ Completed Epochs│            ║
║  │       42        │  │      1680       │  │       42        │            ║
║  └─────────────────┘  └─────────────────┘  └─────────────────┘            ║
║                                                                               ║
║  ┌─────────────────────────────────────────────────────────────────────┐   ║
║  │                     Recently Finalized Rounds                        │   ║
║  ├──────────────────────────────────────────────────────────────────────┤   ║
║  │ Round │ Certified Blocks │ Total Blocks │ Completion % │   Status    │   ║
║  ├──────────────────────────────────────────────────────────────────────┤   ║
║  │  41   │       25         │      25      │   100.0%     │ ✅ Finalized│   ║
║  │  40   │       24         │      25      │    96.0%     │ ✅ Finalized│   ║
║  │  39   │       25         │      25      │   100.0%     │ ✅ Finalized│   ║
║  │  38   │       23         │      25      │    92.0%     │ ✅ Finalized│   ║
║  │  37   │       25         │      25      │   100.0%     │ ✅ Finalized│   ║
║  │  36   │       16         │      25      │    64.0%     │ ⚠️ Partial  │   ║
║  │  35   │       25         │      25      │   100.0%     │ ✅ Finalized│   ║
║  │  34   │       24         │      25      │    96.0%     │ ✅ Finalized│   ║
║  │  33   │       25         │      25      │   100.0%     │ ✅ Finalized│   ║
║  │  32   │        0         │      25      │     0.0%     │ ⚪ In Progress│  ║
║  └──────────────────────────────────────────────────────────────────────┘   ║
║                                                                               ║
║  ┌─────────────────────────────────────────────────────────────────────┐   ║
║  │               Epoch 41 Nominees (Shuffled Order)                     │   ║
║  │                          ...                                         │   ║
║  └─────────────────────────────────────────────────────────────────────┘   ║
║                                                                               ║
╚══════════════════════════════════════════════════════════════════════════════╝
```

## Color Coding

The Status column uses color-coded indicators for quick visual assessment:

### 🟢 Finalized (Green)
- **Color**: `#4ade80` (bright green)
- **Threshold**: ≥ 67% completion
- **Meaning**: Round has achieved Byzantine fault tolerant consensus
- **Example**: Round 41 with 25/25 blocks certified (100%)

### 🟡 Partial (Yellow/Amber)
- **Color**: `#fbbf24` (amber)
- **Threshold**: > 0% and < 67% completion
- **Meaning**: Some blocks certified but consensus threshold not met
- **Example**: Round 36 with 16/25 blocks certified (64%)

### ⚪ In Progress (Gray)
- **Color**: `#888` (gray)
- **Threshold**: 0% completion
- **Meaning**: No blocks have been certified yet
- **Example**: Round 32 with 0/25 blocks certified (0%)

## Example Scenarios

### Healthy Network
```
Round 45:  25/25 (100%) ✅ Finalized
Round 44:  24/25 ( 96%) ✅ Finalized  
Round 43:  25/25 (100%) ✅ Finalized
Round 42:  25/25 (100%) ✅ Finalized
Round 41:  23/25 ( 92%) ✅ Finalized
```
**Interpretation**: All rounds consistently exceed the 67% threshold. Network is healthy.

### Network with Issues
```
Round 45:  12/25 ( 48%) ⚠️ Partial
Round 44:  14/25 ( 56%) ⚠️ Partial
Round 43:  10/25 ( 40%) ⚠️ Partial
Round 42:  25/25 (100%) ✅ Finalized
Round 41:  24/25 ( 96%) ✅ Finalized
```
**Interpretation**: Recent rounds not reaching finalization. Possible validator connectivity issues or insufficient participation.

### Recovering Network
```
Round 45:  25/25 (100%) ✅ Finalized
Round 44:  24/25 ( 96%) ✅ Finalized
Round 43:  18/25 ( 72%) ✅ Finalized
Round 42:  15/25 ( 60%) ⚠️ Partial
Round 41:  10/25 ( 40%) ⚠️ Partial
```
**Interpretation**: Network had issues in rounds 41-42 but has recovered. Finalization resumed normally.

### Typical In-Progress Round
```
Round 45:   0/25 (  0%) ⚪ In Progress
Round 44:  25/25 (100%) ✅ Finalized
Round 43:  24/25 ( 96%) ✅ Finalized
```
**Interpretation**: Round 45 just started. No certificates yet is normal. Will update as validators submit certificates.

## Technical Details

### Update Frequency
- The section refreshes every 10 seconds (configurable via `STATUS_PAGE_REFRESH_SECS`)
- Shows the last 10 rounds (configurable via `STATUS_FINALIZED_ROUNDS_TO_SHOW`)
- Current round is excluded (always in progress)

### Data Source
- Queries `ValidatorBlock` entries from both:
  - **ValidatorActive**: Recent, in-progress blocks
  - **ValidatorFinal**: Older, finalized blocks
- Counts blocks where `cert` field is `Some(...)` as certified

### Byzantine Fault Tolerance
- **Threshold**: 66.67% (2/3 + 1)
- **Rationale**: With n validators, network tolerates up to ⌊(n-1)/3⌋ Byzantine failures
- **Safety**: 2/3+1 honest validators guarantee correctness
- **Liveness**: Network can make progress even with f < n/3 failures

### Empty State
When no validator blocks exist:
```
┌─────────────────────────────────────────────────────────┐
│           Recently Finalized Rounds                     │
├─────────────────────────────────────────────────────────┤
│ Round │ Certified │ Total │ Completion % │   Status     │
├─────────────────────────────────────────────────────────┤
│                No finalized rounds yet                   │
└─────────────────────────────────────────────────────────┘
```
This is expected when:
- Node just started
- Running as miner-only (no validator rounds)
- Network hasn't progressed beyond round 0

## Browser Compatibility

The section uses standard HTML table styling and should work in all modern browsers:
- Chrome/Edge (Chromium)
- Firefox
- Safari
- Opera

The dark theme uses:
- Background: `#1a1a1a` (dark gray)
- Text: `#e0e0e0` (light gray)
- Headers: `#4a9eff` (blue)
- Borders: `#333` (medium gray)

## Accessibility

- Clear visual indicators (color + text)
- High contrast ratios for readability
- Hover effects on table rows
- Semantic HTML table structure
- No reliance on color alone (status text included)

## Mobile Responsiveness

The table is scrollable on smaller screens:
- Horizontal scroll enabled via `.blocks-container`
- Max height of 600px with vertical scroll
- Custom styled scrollbars (webkit)
- Maintains readability on all screen sizes

## Performance Considerations

- Lightweight: ~5-10 KB per page load
- Minimal JavaScript (only for tab switching)
- Efficient queries (indexed by round)
- Capped at 10 rounds maximum
- No client-side computation required

