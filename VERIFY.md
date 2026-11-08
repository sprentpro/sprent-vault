# Verification

## Scope and deployment status

Program ID: `GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g`.

[Devnet program](https://explorer.solana.com/address/GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g?cluster=devnet).
The devnet account is executable. Mainnet deployment is pending. Neither an IDL comparison nor a passing test suite proves that an on-chain binary matches this repository.

## Build and test locally

Use Rust 1.95.0, Solana CLI 3.0.0, Anchor CLI 0.31.1 and Node.js 22. The build script selects SBF platform-tools v1.52.

```bash
bash scripts/build.sh
cargo test -p sprent_vault
cargo fmt --all -- --check
cd sdk
npm ci
npm run lint
npm run build
npm test
```

The suites contain 16 Rust tests and 48 TypeScript tests. These cover the checked-in unit and accounting cases; they are not an external security audit.

## Compare the interface

After building, compare the generated IDL with the checked-in IDL, excluding documentation fields:

```bash
python3 - target/idl/sprent_vault.json idl/sprent_vault.json <<'PY'
import hashlib, json, sys
def strip(value):
    if isinstance(value, dict):
        return {k: strip(v) for k, v in value.items() if k != "docs"}
    if isinstance(value, list):
        return [strip(v) for v in value]
    return value
hashes = [hashlib.sha256(json.dumps(strip(json.load(open(f))), sort_keys=True, separators=(",", ":")).encode()).hexdigest() for f in sys.argv[1:]]
print(*hashes, sep="\n")
assert hashes[0] == hashes[1], "IDL mismatch"
PY
```

## Reproducible release build

The deployment operator must pin the exact Git commit and Docker build image digest before building the mainnet artifact. Preserve the `.so` SHA-256, tool versions and deployment transaction outside the repository. Do not edit or reformat `programs/` after producing that artifact.

With Docker running and `BUILD_IMAGE` set to the agreed verification image **including its sha256 digest**:

```bash
: "${BUILD_IMAGE:?Set the pinned release build image digest}"
solana-verify build --base-image "$BUILD_IMAGE" --library-name sprent_vault
sha256sum target/deploy/sprent_vault.so
```

The ordinary local build above is a development check, not a substitute for this release build.

## Verify after mainnet deployment

Run this from the exact commit used to build and deploy. The repository must be public and the mainnet program must exist. Use the same pinned build image as the release artifact. These flags are supported by solana-verify 0.5.2.

```bash
: "${BUILD_IMAGE:?Use the exact release build image digest}"
COMMIT=$(git rev-parse HEAD)
solana-verify verify-from-repo \
  --url https://api.mainnet-beta.solana.com \
  --program-id GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g \
  --library-name sprent_vault \
  --commit-hash "$COMMIT" \
  --base-image "$BUILD_IMAGE" \
  https://github.com/sprentpro/sprent-vault
```

Do not use `--skip-build`: it skips the comparison. Publishing verification metadata on-chain is a separate signed action; do not accept that prompt unless authorized. Record the actual comparison result, commit, image digest and transaction before marking a mainnet release as verified.

Mainnet verification status: **not run — no mainnet deployment yet**.
