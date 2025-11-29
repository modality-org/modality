#!/usr/bin/env bash
# View the HTTP status page for the miner

echo "📊 Opening HTTP Status Page"
echo ""
echo "The status page shows real-time information about the running miner:"
echo "  - Connected Peers"
echo "  - Total Miner Blocks"
echo "  - Current Mining Difficulty"
echo "  - Current Epoch"
echo "  - Last 80 blocks with their hashes and nominees"
echo ""
echo "The page auto-refreshes every 10 seconds."
echo ""
echo "Status page URL: http://localhost:8080"
echo ""

# Try to open in browser
if command -v open &> /dev/null; then
    open http://localhost:8080
elif command -v xdg-open &> /dev/null; then
    xdg-open http://localhost:8080
else
    echo "To view the status page, open this URL in your browser:"
    echo "http://localhost:8080"
fi

