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

