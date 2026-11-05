#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
anchor build --no-idl -- --tools-version v1.52
mkdir -p target/idl target/types
anchor idl build -o target/idl/sprent_vault.json -t target/types/sprent_vault.ts
