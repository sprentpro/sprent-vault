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

