#!/bin/bash
# Example CLI workflow for agent contract negotiation

set -e

echo "=== Modality CLI Workflow Demo ==="
echo ""

# Step 1: Agent A creates a proposal
echo "Step 1: Agent A proposes an escrow contract to Agent B"
modality contract propose \
    --type escrow \
    --from agent_a \
    --to agent_b \
    --terms "100 tokens for data analysis" \
    --output proposal.json

echo "Proposal saved to proposal.json"
echo ""

# Step 2: Agent B reviews and accepts
echo "Step 2: Agent B accepts the proposal"
modality contract accept \
    --proposal proposal.json \
    --output contract.json

echo "Contract created: contract.json"
echo ""

# Step 3: Check initial status
echo "Step 3: Check contract status"
modality contract status --contract contract.json
echo ""

# Step 4: Agent A deposits
echo "Step 4: Agent A checks available actions"
modality contract actions --contract contract.json --agent agent_a
echo ""

echo "Step 4: Agent A deposits"
modality contract act \
    --contract contract.json \
    --agent agent_a \
    --action deposit
echo ""

# Step 5: Agent B delivers
echo "Step 5: Agent B checks available actions"
modality contract actions --contract contract.json --agent agent_b
echo ""

echo "Step 5: Agent B delivers"
modality contract act \
    --contract contract.json \
    --agent agent_b \
    --action deliver
echo ""

# Step 6: Agent A releases
echo "Step 6: Agent A releases payment"
modality contract act \
    --contract contract.json \
    --agent agent_a \
    --action release
echo ""

# Step 7: Final status
echo "Step 7: Final contract status"
modality contract status --contract contract.json
echo ""

# Step 8: View history
echo "Step 8: Contract history"
modality contract history --contract contract.json
echo ""

echo "=== Workflow Complete ==="

# Cleanup
rm -f proposal.json contract.json
