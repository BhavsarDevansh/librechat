#!/usr/bin/env bash
# LibreChat Build Script
# Builds the Leptos WASM frontend and the Axum backend server.
# Usage: ./build.sh [-r]
#   -r  Build in release mode (optimized)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

RELEASE=""
PROFILE_DIR="debug"

if [[ "${1:-}" == "-r" ]]; then
    RELEASE="--release"
    PROFILE_DIR="release"
fi

echo "Building LibreChat (${PROFILE_DIR} mode)..."

# 1. Build frontend WASM
echo "Building frontend..."
cargo build --target wasm32-unknown-unknown ${RELEASE} -p frontend

# 2. Generate JS/WASM bindings
echo "Generating WASM bindings..."
FRONTEND_DIST="$SCRIPT_DIR/frontend/dist"
mkdir -p "$FRONTEND_DIST"

wasm-bindgen \
    --target web \
    --no-typescript \
    --out-dir "$FRONTEND_DIST" \
    "$SCRIPT_DIR/target/wasm32-unknown-unknown/${PROFILE_DIR}/frontend.wasm"

# 3. Copy CSS to dist
echo "Copying styles..."
cp "$SCRIPT_DIR/frontend/style/main.css" "$FRONTEND_DIST/main.css"

# 4. Generate index.html
echo "Generating index.html..."
cat > "$FRONTEND_DIST/index.html" << 'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <title>LibreChat</title>
    <link rel="stylesheet" href="/main.css"/>
<script type="module">
import init, * as bindings from '/frontend.js';
const wasm = await init({ module_or_path: '/frontend_bg.wasm' });


window.wasmBindings = bindings;


dispatchEvent(new CustomEvent("TrunkApplicationStarted", {detail: {wasm}}));

</script>
</head>
<body></body>
</html>
HTMLEOF

# 5. Build server
echo "Building server..."
cargo build ${RELEASE} -p server

echo ""
echo "Build complete!"
echo "Run with: cargo run -p server ${RELEASE}"
