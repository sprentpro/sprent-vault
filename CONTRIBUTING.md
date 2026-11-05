# Contributing

Thank you for helping improve SPRENT. This document explains how to build, test and submit changes.

## Toolchain

| Tool | Version |
|---|---|
| Rust | 1.95.0 (pinned in `rust-toolchain.toml`) |
| Solana CLI | 3.0.0 |
| Anchor CLI | 0.31.1 |
| Node.js | 22 or newer |

## Build, test and lint

These are the same commands the CI workflow runs (`.github/workflows/ci.yml`):

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

## Changes to the program

- Keep the public interface stable unless the change is intentional. After `bash scripts/build.sh`, compare `target/idl/sprent_vault.json` with `idl/sprent_vault.json` using the normalised hash in [VERIFY.md](VERIFY.md), and commit the new IDL when the interface changes.
- Every new instruction needs its account checks (seeds, `has_one` or `address` constraints) and a test.
- Accounting changes must be made in both `programs/sprent_vault/src/accounting.rs` and `sdk/src/accounting.ts`, with tests on both sides.
- Update `VERIFY.md` with the new test counts.

## Pull requests

- One topic per pull request, with a short description of what changed and why.
- Commit messages are plain sentences describing the change (no type prefixes).
- All CI steps must pass.

## Reporting security issues

Do not open public issues for vulnerabilities. Follow [SECURITY.md](SECURITY.md).

## Code of conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
