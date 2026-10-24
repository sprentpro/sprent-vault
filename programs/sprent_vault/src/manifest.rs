use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;

use crate::errors::SprentError;

pub const MANIFEST_PROGRAM_ID: Pubkey = pubkey!("MNFSTqtC93rEfYHB6hF82sKdZpUDFWkViLByLd1k1Ms");

pub const TAG_CLAIM_SEAT: u8 = 1;
pub const TAG_DEPOSIT: u8 = 2;
pub const TAG_WITHDRAW: u8 = 3;
pub const TAG_BATCH_UPDATE: u8 = 6;

pub const ORDER_TYPE_POST_ONLY: u8 = 2;

pub const MARKET_DISCRIMINANT: u64 = 4_859_840_929_024_028_656;
pub const MARKET_HEADER_LEN: usize = 256;
const OFF_BASE_DECIMALS: usize = 9;
const OFF_QUOTE_DECIMALS: usize = 10;
const OFF_BASE_MINT: usize = 16;
const OFF_QUOTE_MINT: usize = 48;
const OFF_BASE_VAULT: usize = 80;
const OFF_QUOTE_VAULT: usize = 112;

pub const MARKET_VAULT_SEED: &[u8] = b"vault";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WireCancel {
    pub order_sequence_number: u64,
    pub order_index_hint: Option<u32>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WirePlace {
    pub base_atoms: u64,
    pub price_mantissa: u32,
    pub price_exponent: i8,
    pub is_bid: bool,
    pub last_valid_slot: u32,
    pub order_type: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WireBatch {
    pub trader_index_hint: Option<u32>,
    pub cancels: Vec<WireCancel>,
    pub orders: Vec<WirePlace>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct WireAmount {
    pub amount_atoms: u64,
    pub trader_index_hint: Option<u32>,
}

pub struct MarketHeader {
    pub base_decimals: u8,
    pub quote_decimals: u8,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
}

fn read_pubkey(data: &[u8], off: usize) -> Pubkey {
    let mut b = [0u8; 32];
    b.copy_from_slice(&data[off..off + 32]);
    Pubkey::new_from_array(b)
}

pub fn read_market_header(market: &AccountInfo) -> Result<MarketHeader> {
    require_keys_eq!(
        *market.owner,
        MANIFEST_PROGRAM_ID,
        SprentError::MarketOwnerMismatch
    );
    let data = market.try_borrow_data()?;
    require!(
        data.len() >= MARKET_HEADER_LEN,
        SprentError::MarketLayoutMismatch
    );
    let mut d = [0u8; 8];
    d.copy_from_slice(&data[0..8]);
    require!(
        u64::from_le_bytes(d) == MARKET_DISCRIMINANT,
        SprentError::MarketLayoutMismatch
    );
    Ok(MarketHeader {
        base_decimals: data[OFF_BASE_DECIMALS],
        quote_decimals: data[OFF_QUOTE_DECIMALS],
        base_mint: read_pubkey(&data, OFF_BASE_MINT),
        quote_mint: read_pubkey(&data, OFF_QUOTE_MINT),
        base_vault: read_pubkey(&data, OFF_BASE_VAULT),
        quote_vault: read_pubkey(&data, OFF_QUOTE_VAULT),
    })
}

pub fn market_vault_address(market: &Pubkey, mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[MARKET_VAULT_SEED, market.as_ref(), mint.as_ref()],
        &MANIFEST_PROGRAM_ID,
    )
    .0
}

fn data_with_tag<T: AnchorSerialize>(tag: u8, params: &T) -> Result<Vec<u8>> {
    let mut data = vec![tag];
    params
        .serialize(&mut data)
        .map_err(|_| error!(SprentError::SerializationFailed))?;
    Ok(data)
}

pub fn claim_seat_ix(trader: Pubkey, market: Pubkey, system_program: Pubkey) -> Instruction {
    Instruction {
        program_id: MANIFEST_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(trader, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: vec![TAG_CLAIM_SEAT],
    }
}

#[allow(clippy::too_many_arguments)]
pub fn token_move_ix(
    tag: u8,
    trader: Pubkey,
    market: Pubkey,
    trader_token: Pubkey,
    market_vault: Pubkey,
    token_program: Pubkey,
    mint: Pubkey,
    amount_atoms: u64,
) -> Result<Instruction> {
    Ok(Instruction {
        program_id: MANIFEST_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(trader, true),
            AccountMeta::new(market, false),
            AccountMeta::new(trader_token, false),
            AccountMeta::new(market_vault, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(mint, false),
        ],
        data: data_with_tag(
            tag,
            &WireAmount {
                amount_atoms,
                trader_index_hint: None,
            },
        )?,
    })
}

pub fn batch_update_ix(
    trader: Pubkey,
    market: Pubkey,
    system_program: Pubkey,
    batch: &WireBatch,
) -> Result<Instruction> {
    Ok(Instruction {
        program_id: MANIFEST_PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(trader, true),
            AccountMeta::new(market, false),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: data_with_tag(TAG_BATCH_UPDATE, batch)?,
    })
}

pub fn invoke_as_vault(
    ix: &Instruction,
    infos: &[AccountInfo],
    config_key: &Pubkey,
    vault_bump: u8,
) -> Result<()> {
    let bump = [vault_bump];
    let seeds: &[&[u8]] = &[
        crate::constants::VAULT_AUTH_SEED,
        config_key.as_ref(),
        &bump,
    ];
    invoke_signed(ix, infos, &[seeds]).map_err(Into::into)
}

const OFF_BIDS_ROOT: usize = 156;
const OFF_BIDS_BEST: usize = 160;
const OFF_ASKS_ROOT: usize = 164;
const OFF_ASKS_BEST: usize = 168;
const OFF_SEATS_ROOT: usize = 172;
const BLOCK: usize = 80;
const NODE_LEFT: usize = 0;
const NODE_RIGHT: usize = 4;
const NODE_VALUE: usize = 16;

const SEAT_TRADER: usize = NODE_VALUE;
const SEAT_BASE: usize = NODE_VALUE + 32;
const SEAT_QUOTE: usize = NODE_VALUE + 40;

const ORDER_PRICE: usize = NODE_VALUE;
const ORDER_TRADER_INDEX: usize = NODE_VALUE + 32;
pub const NIL_INDEX: u32 = u32::MAX;
const MAX_TREE_DEPTH: usize = 64;
const MAX_ORDER_NODES: u32 = 4_096;

pub const MANIFEST_PRICE_TO_FP: u128 = 1_000_000;

pub const MAX_VAULT_ORDERS: usize = 20;

#[derive(Debug, Clone, Copy, Default)]
pub struct VaultOrder {
    pub seq: u64,
    pub index: u32,
    pub is_bid: bool,
    pub expired: bool,
    pub base_atoms: u64,

    pub price_raw: u128,
}

impl VaultOrder {
    pub fn quote_atoms(&self) -> Option<u128> {
        (self.base_atoms as u128)
            .checked_mul(self.price_raw)
            .map(|x| x.div_ceil(MANIFEST_PRICE_ONE))
    }
}

pub const MANIFEST_PRICE_ONE: u128 = 1_000_000_000_000_000_000;

#[derive(Debug, Clone, Copy, Default)]
pub struct VenueView {
    pub seat_found: bool,
    pub seat_base: u64,
    pub seat_quote: u64,

    pub vault_orders: u32,
    pub orders: [VaultOrder; MAX_VAULT_ORDERS],
    pub best_bid_fp: Option<u128>,
    pub best_ask_fp: Option<u128>,
}

impl VenueView {
    pub fn listed(&self) -> &[VaultOrder] {
        &self.orders[..(self.vault_orders as usize).min(MAX_VAULT_ORDERS)]
    }
}

fn rd_u32(d: &[u8], off: usize) -> Result<u32> {
    let s = d
        .get(off..off + 4)
        .ok_or_else(|| error!(SprentError::MarketLayoutMismatch))?;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}
