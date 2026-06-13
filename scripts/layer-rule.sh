#!/usr/bin/env bash
# Layer-rule lint: sim/ must not import from shell/.
# Run from repo root.
set -e
if grep -rnE "use +crate::shell|crate::shell::" src/sim/; then
  echo "ERROR: sim/ must not import from shell/"
  exit 1
fi
echo "layer-rule: ok"
