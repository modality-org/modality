#!/usr/bin/env bash
# Public hybrid testnet health checks. Run from a laptop with modal on PATH
# (or MODAL=/path/to/modal). Does not install packages or wipe chain state.
set -euo pipefail

MODAL="${MODAL:-modal}"
fail=0
ok() { printf '  ok  %s\n' "$*"; }
bad() { printf '  FAIL %s\n' "$*"; fail=1; }

echo "== HTTPS landing and redirects"
for url in \
  https://testnet.modality.network/ \
  https://node0.testnet.modality.network/ \
  https://node1.testnet.modality.network/ \
  https://node2.testnet.modality.network/ \
  https://node3.testnet.modality.network/
do
  code=$(curl -fsS -o /tmp/modality-health-body -w '%{http_code}' --max-time 20 "$url" || true)
  if [[ "$code" == "200" ]]; then
    ok "$url HTTP $code ($(wc -c < /tmp/modality-health-body | tr -d ' ') bytes)"
  else
    bad "$url HTTP ${code:-curl-failed}"
  fi
done

landing_src=$(curl -fsS --max-time 20 https://testnet.modality.network/ || true)
if echo "$landing_src" | grep -q "var observer = 'https://node0.testnet.modality.network'"; then
  ok "landing polls node0 /status.json"
else
  bad "landing does not poll node0 /status.json"
fi

redir=$(curl -sS -o /dev/null -w '%{http_code} %{redirect_url}' --max-time 20 https://testnet.modal.money/ || true)
if [[ "$redir" == 301*testnet.modality.network* ]]; then
  ok "testnet.modal.money -> $redir"
else
  bad "testnet.modal.money redirect was '$redir'"
fi

echo "== DNS"
for host in testnet node0 node1 node2 node3; do
  name="${host}.testnet.modality.network"
  if [[ "$host" == "testnet" ]]; then name="testnet.modality.network"; fi
  if dig +short "$name" A | grep -qE '^[0-9]'; then
    ok "$name has A $(dig +short "$name" A | tr '\n' ' ')"
  else
    bad "$name missing A"
  fi
done
txt=$(dig +short TXT _dnsaddr.testnet.modality.network | tr -d '"')
if echo "$txt" | grep -q 'dns4/node1.testnet.modality.network'; then
  ok "_dnsaddr TXT present"
else
  bad "_dnsaddr TXT missing or unexpected: $txt"
fi

echo "== Status JSON / HEAD (new binaries only)"
for host in node1 node2 node3 node0; do
  url="https://${host}.testnet.modality.network/status.json"
  code=$(curl -sS -o /tmp/modality-status.json -w '%{http_code}' --max-time 20 "$url" || true)
  if [[ "$code" == "200" ]]; then
    ok "$url HTTP 200 $(head -c 180 /tmp/modality-status.json)"
    if [[ "$host" != "node0" ]]; then
      nval=$(python3 -c "import json; print(len(json.load(open('/tmp/modality-status.json')).get('named_validators') or []))" 2>/dev/null || echo 0)
      dest=$(python3 -c "import json; print(json.load(open('/tmp/modality-status.json')).get('dest_apply_requires_cert'))" 2>/dev/null || echo "")
      if [[ "$nval" == "3" ]]; then
        ok "$host named_validators=$nval dest_apply_requires_cert=$dest"
      else
        bad "$host named_validators=$nval (want 3) dest_apply_requires_cert=$dest"
      fi
    fi
  else
    bad "$url HTTP ${code:-curl-failed}"
  fi
  head_code=$(curl -sS -o /dev/null -w '%{http_code}' --max-time 20 -I "https://${host}.testnet.modality.network/" || true)
  if [[ "$head_code" == "200" ]]; then
    ok "HEAD https://${host}.testnet.modality.network/ $head_code"
  else
    bad "HEAD https://${host}.testnet.modality.network/ $head_code"
  fi
done

echo "== Explorer API (node0)"
exp_code=$(curl -sS -o /tmp/modality-explorer.json -w '%{http_code}' --max-time 20 \
  https://node0.testnet.modality.network/api/contracts || true)
if [[ "$exp_code" == "200" ]]; then
  ok "node0 /api/contracts HTTP 200"
else
  bad "node0 /api/contracts HTTP ${exp_code:-curl-failed}"
fi

echo "== P2P ping"
if ! command -v "$MODAL" >/dev/null 2>&1 && [[ ! -x "$MODAL" ]]; then
  bad "modal binary not found (set MODAL=)"
else
  ping_dir="${PING_DIR:-/tmp/modality-testnet-health-ping}"
  mkdir -p "$ping_dir"
  if [[ ! -f "$ping_dir/config.json" ]]; then
    "$MODAL" node create --dir "$ping_dir" --testnet >/tmp/modality-health-create.txt
  fi
  declare -a targets=(
    '/dns4/node1.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWE4NPREQxLkevA5Rxd61Xiue4tTkUGN22qNABD7Mw5JhM'
    '/dns4/node2.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWJpFYTRHNuPfwoj1hTf87aqB7CDJHKtVFp3RhPNB1DrRw'
    '/dns4/node3.testnet.modality.network/tcp/4040/ws/p2p/12D3KooWLHTsoeBE1ZWBgzumeSi6hsm3o9AndFufrGx7xLTyq2dw'
  )
  for t in "${targets[@]}"; do
    if "$MODAL" node ping --dir "$ping_dir" --target "$t" >/tmp/modality-health-ping.txt 2>&1; then
      ok "ping $t"
    else
      bad "ping $t ($(tail -n 3 /tmp/modality-health-ping.txt | tr '\n' ' '))"
    fi
  done
fi

if [[ "$fail" -ne 0 ]]; then
  echo "== FAILED"
  exit 1
fi
echo "== OK"
