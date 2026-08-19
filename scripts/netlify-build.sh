#!/usr/bin/env bash
# Build de la web para Netlify: WASM (wasm-pack) + vite.
# Corre desde la raíz del repo. Reproducible localmente: bash scripts/netlify-build.sh
set -euo pipefail

# wasm-pack: binario precompilado (mucho más rápido que cargo install)
if ! command -v wasm-pack >/dev/null; then
  curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
  export PATH="$HOME/.cargo/bin:$PATH"
fi

wasm-pack build crates/obrero-wasm --target web --out-dir ../../web/src/pkg

cd web
npm ci
npm run build
