#!/usr/bin/env bash
# Run TLC on the hybrid-consensus composition spec.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
jar="${TLA2TOOLS_JAR:-$root/tla/tla2tools.jar}"
if [[ ! -f "$jar" ]]; then
  echo "Downloading tla2tools.jar to $jar" >&2
  curl -fsSL -o "$jar" \
    "https://github.com/tlaplus/tlaplus/releases/download/v1.8.0/tla2tools.jar"
fi
cfg="${1:-$root/tla/MCHybrid.cfg}"
java -XX:+UseParallelGC -cp "$jar" tlc2.TLC \
  -config "$cfg" \
  -workers auto \
  "$root/tla/Hybrid.tla"
