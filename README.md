<p align="center"><img src="assets/banner.png" alt="SPRENT" width="100%"></p>

# SPRENT

<p align="center">
<a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-101113?style=flat-square" alt="License: MIT"></a>
<a href="https://solana.com"><img src="https://img.shields.io/badge/chain-Solana-101113?style=flat-square" alt="Chain: Solana"></a>
<a href="https://www.anchor-lang.com"><img src="https://img.shields.io/badge/Anchor-0.31.1-101113?style=flat-square" alt="Anchor 0.31.1"></a>
<a href="programs/sprent_vault"><img src="https://img.shields.io/badge/rust-1.95.0-A34B38?style=flat-square" alt="Rust 1.95.0"></a>
<a href="sdk"><img src="https://img.shields.io/badge/typescript-5.9.3-A34B38?style=flat-square" alt="TypeScript 5.9.3"></a>
<a href="https://github.com/sprentpro/sprent-vault/actions/workflows/ci.yml"><img src="https://github.com/sprentpro/sprent-vault/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
<a href="VERIFY.md"><img src="https://img.shields.io/badge/tests-64%20passing-C2F143?style=flat-square&labelColor=101113" alt="Tests: 64 passing"></a>
<a href="https://sprent.pro"><img src="https://img.shields.io/badge/website-sprent.pro-F5F4EE?style=flat-square&labelColor=101113" alt="Website"></a>
<a href="https://x.com/sprent_pro"><img src="https://img.shields.io/badge/X-%40sprent__pro-101113?style=flat-square" alt="X"></a>
</p>

<p align="center">
<a href="https://sprent.pro">Website</a> &nbsp;/&nbsp;
<a href="https://x.com/sprent_pro">X</a> &nbsp;/&nbsp;
<a href="https://github.com/sprentpro/sprent-vault">Source</a> &nbsp;/&nbsp;
<a href="https://github.com/sprentpro">Organization</a> &nbsp;/&nbsp;
<a href="VERIFY.md">Verification</a>
</p>

**On-chain market-making desks, with custody and accounting in the program.**

## Overview

SPRENT is a custody vault on Solana that market-makes a real order book. Depositors put USDC into a desk vault and receive shares on an internal ledger; an operator key quotes post-only orders on the [Manifest](https://github.com/CKS-Systems/manifest) SOL/USDC market through the vault, and every epoch the vault flattens its venue position, prices its holdings against the Pyth oracle and settles each depositor's share of the result, with a high-water-mark performance fee.

The operator key can place and cancel quotes inside on-chain risk limits. It cannot send funds to an address of its choosing: the vault PDA is the only Manifest trader, Manifest withdrawals land in vault-owned token accounts, and customer withdrawals pay out only to the depositor's own token account. Trading can still lose money, and the admin key and the program upgrade authority carry more power than the operator; see [Trust assumptions](#trust-assumptions).

This repository contains the on-chain program `sprent_vault` (Anchor) and `@sprent/vault-accounting`, a dependency-free TypeScript reference of the vault ledger used by the off-chain indexer.

- Program ID: `GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g` ([devnet explorer](https://explorer.solana.com/address/GnfjrTU3LuytJm1BSLVWvBM721Jvs1dTQBa4aWAMUw3g?cluster=devnet)). Mainnet deployment is pending.
- Venue: Manifest `MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms` ([Solscan](https://solscan.io/account/MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms?cluster=mainnet-beta))
- Tests: 64 passing (16 Rust unit tests, 48 TypeScript tests), recorded in [VERIFY.md](VERIFY.md)

## How it works

1. **Deposit.** `deposit` moves USDC from the depositor into the desk's vault token account and queues it. Shares are minted at the next epoch's settled price after the entry delay, so a deposit never buys into a price it has already seen.
2. **Quote.** The operator calls `fund_venue` to move working capital into the vault's Manifest seat and `quote_batch` to cancel and place orders. The program signs the Manifest CPI with the vault PDA, rejects any order type other than PostOnly, rejects prices outside the oracle band, and enforces inventory, bid size and orders-per-batch limits. A loss beyond the epoch loss limit halts the desk; a halted desk can only reduce inventory.
3. **Close the epoch.** `cancel_all` and `defund_venue` bring every order and balance back to the vault. `close_epoch` then requires zero open orders, reads the vault's actual token balances and the Pyth SOL/USD price (staleness, confidence and book-deviation checks) and writes an `EpochNav` record. If a check fails the epoch is frozen for reconciliation instead of being priced.
4. **Settle.** `settle_position` brings one depositor up to the closed epoch: mints queued deposits, crystallises the performance fee above that depositor's own high-water mark and turns matured withdrawal requests into claimable USDC. `sweep_fees` moves crystallised fees to the service treasury.
5. **Withdraw.** `request_withdraw` queues shares behind the cooldown; after settlement `claim_withdraw` pays USDC to a token account owned by the depositor.
6. **Platform.** Net service income is split by `distribute_service` into a holder reward pool for staked $SPRENT, a buyback budget and a team budget. Staking uses a warmup, time-weighted reward windows and an unstake cooldown. Buybacks are tracked as intents on a `BuybackLedger` (opened, recorded with the buy signature, burned), with customer vault accounts rejected as a funding source.

