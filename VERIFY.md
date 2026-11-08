# Verification

## Scope and deployment status

Program ID: `GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g`.

[Mainnet program on Solscan](https://solscan.io/account/GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g). The mainnet account is executable and uses Solana's upgradeable program loader.

## Mainnet check

Checked on 2026-11-08 against mainnet-beta at finalized slot `466816414`. The on-chain executable hash matches the release artifact available to the deployment operator:

| Check | Result |
|---|---|
| Mainnet executable hash | `a109a97f96d949ce4cc80b6c9dd0c00ba776e77d192a119ce16d58eeda7fca03` |
| Release artifact executable hash | `a109a97f96d949ce4cc80b6c9dd0c00ba776e77d192a119ce16d58eeda7fca03` |
| Release file SHA-256 | `7a461deb0e7a075894a90450aae0af067cd05279e642145b2db69daff8dd4d47` |
| Release file size | 1,113,024 bytes |
| Hash tool | `solana-verify` 0.5.2 |

This confirms the deployed executable matches that release artifact. A fresh rebuild from the public Git repository was not run as part of this check, so this record does not certify a source-to-binary match or an external security audit.

Recheck the deployed program and a local release file:

```bash
solana-verify get-program-hash --url https://api.mainnet-beta.solana.com GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g
solana-verify get-executable-hash target/deploy/sprent_vault.so
sha256sum target/deploy/sprent_vault.so
```

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

Reproducing the release requires the exact Git source revision and pinned Docker build image digest. Preserve the `.so` SHA-256, tool versions and deployment transaction outside the repository. Do not edit or reformat `programs/` after producing that artifact.

With Docker running:

```bash
BUILD_IMAGE=solanafoundation/solana-verifiable-build@sha256:2fc14309065785484919e85cdc40136c8e8f6f074e9d3d087f7a08015d9733a9
solana-verify build --base-image "$BUILD_IMAGE" --library-name sprent_vault \
  --cargo-build-sbf-args="--tools-version v1.52"
sha256sum target/deploy/sprent_vault.so
```

The ordinary local build above is a development check, not a substitute for this release build.

## Verify from the public repository

The source revision below has the same program files, `Cargo.toml` and `Cargo.lock` as the mainnet documentation update. Rebuild that exact revision with the pinned release image and compare it with mainnet. Docker is required; the flags shown here are supported by solana-verify 0.5.2.

```bash
COMMIT=4e4f209b6953cbc1c14947d872ac4c4a62ee934a
solana-verify verify-from-repo \
  --url https://api.mainnet-beta.solana.com \
  --program-id GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g \
  --library-name sprent_vault \
  --commit-hash "$COMMIT" \
  --base-image solanafoundation/solana-verifiable-build@sha256:2fc14309065785484919e85cdc40136c8e8f6f074e9d3d087f7a08015d9733a9 \
  --cargo-build-sbf-args="--tools-version v1.52" \
  https://github.com/sprentpro/sprent-vault
```

Do not use `--skip-build`: it skips building and verification. Publishing verification metadata on-chain is a separate signed action. Record the actual rebuild result and exact commit before claiming verification against public source.

Mainnet verification status: **release artifact hash match confirmed; public-source rebuild not performed in this check**.
