# Changelog

This changelog describes the source snapshot. Dates do not indicate GitHub releases.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-04

### Added

- `sprent_vault` Anchor program: desk vault configuration, USDC deposits on an internal share ledger, Manifest seat claim and venue funding through the vault PDA, post-only quote batches with oracle band, inventory, bid size and loss limits, emergency cancel and guardian halt.
- Epoch close from flattened vault balances and a checked Pyth price, reconciliation freeze, per-depositor settlement with high-water-mark performance fee, withdrawal requests with cooldown and claims to the depositor's token account.
- Platform layer: service treasury split into holder rewards, buyback budget and team budget; $SPRENT staking with warmup, time-weighted reward windows and unstake cooldown; staking exclusion list; buyback ledger with recorded buys and burns.
- `@sprent/vault-accounting` TypeScript package: share mint and redeem, NAV, performance-fee crystallisation, service and creator-fee allocation, staker reward split, PnL decomposition and fund-class flow rules.
- Anchor IDL in `idl/sprent_vault.json` and verification record in `VERIFY.md`.

