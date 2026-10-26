use anchor_lang::prelude::*;

use crate::errors::SprentError;

pub const PYTH_RECEIVER_ID: Pubkey = pubkey!("rec5EKMGg6MxZYaMdyBfgwp4d5rB9T1VQH5pJv5LtFJ");

pub const PRICE_UPDATE_V2_DISCRIMINATOR: [u8; 8] = [34, 241, 35, 99, 157, 126, 244, 205];
pub const FP_DECIMALS: i32 = 24;

const VERIFICATION_PARTIAL: u8 = 0;
const VERIFICATION_FULL: u8 = 1;

#[derive(Clone, Copy, Debug)]
pub struct OraclePrice {
    pub price: i64,
    pub conf: u64,
    pub expo: i32,
    pub publish_time: i64,
}

fn rd<const N: usize>(data: &[u8], off: usize) -> Result<[u8; N]> {
    let s = data
        .get(off..off + N)
        .ok_or_else(|| error!(SprentError::OracleLayoutMismatch))?;
    let mut b = [0u8; N];
    b.copy_from_slice(s);
    Ok(b)
}

pub fn read_price_update(ai: &AccountInfo, expected_feed: &[u8; 32]) -> Result<OraclePrice> {
    require_keys_eq!(
        *ai.owner,
        PYTH_RECEIVER_ID,
        SprentError::OracleOwnerMismatch
    );
    let data = ai.try_borrow_data()?;
    require!(
        rd::<8>(&data, 0)? == PRICE_UPDATE_V2_DISCRIMINATOR,
        SprentError::OracleLayoutMismatch
    );
    let level = rd::<1>(&data, 40)?[0];
    let off = match level {
        VERIFICATION_FULL => 41,
        VERIFICATION_PARTIAL => return err!(SprentError::OracleNotFullyVerified),
        _ => return err!(SprentError::OracleLayoutMismatch),
    };
    let feed = rd::<32>(&data, off)?;
    require!(&feed == expected_feed, SprentError::OracleFeedMismatch);
    Ok(OraclePrice {
        price: i64::from_le_bytes(rd::<8>(&data, off + 32)?),
        conf: u64::from_le_bytes(rd::<8>(&data, off + 40)?),
        expo: i32::from_le_bytes(rd::<4>(&data, off + 48)?),
        publish_time: i64::from_le_bytes(rd::<8>(&data, off + 52)?),
    })
}

pub fn checked_oracle_fp(
    p: &OraclePrice,
    now_ts: i64,
    max_age_s: u32,
    max_conf_bps: u16,
    base_decimals: u8,
    quote_decimals: u8,
) -> Result<u128> {
    require!(p.price > 0, SprentError::OraclePriceInvalid);
    let age = now_ts.saturating_sub(p.publish_time);
    require!(age <= max_age_s as i64, SprentError::OracleStale);
    let price = p.price as u128;
    require!(
        (p.conf as u128)
            .checked_mul(10_000)
            .ok_or_else(|| error!(SprentError::MathOverflow))?
            <= price
                .checked_mul(max_conf_bps as u128)
                .ok_or_else(|| error!(SprentError::MathOverflow))?,
        SprentError::OracleConfidenceTooWide
    );

    let exp = p.expo + quote_decimals as i32 - base_decimals as i32 + FP_DECIMALS;
    scale_pow10(price, exp).ok_or_else(|| error!(SprentError::PriceOutOfRange))
}

pub fn order_price_fp(mantissa: u32, exponent: i8) -> Result<u128> {
    require!(mantissa > 0, SprentError::PriceOutOfRange);
    scale_pow10(mantissa as u128, exponent as i32 + FP_DECIMALS)
        .ok_or_else(|| error!(SprentError::PriceOutOfRange))
}

pub fn scale_pow10(v: u128, exp: i32) -> Option<u128> {
    if exp >= 0 {
        let p = 10u128.checked_pow(exp as u32)?;
        v.checked_mul(p)
    } else {
        let p = 10u128.checked_pow((-exp) as u32)?;
        Some(v / p)
    }
}

pub fn within_band(order_fp: u128, oracle_fp: u128, band_bps: u16) -> Result<bool> {
    let diff = order_fp.abs_diff(oracle_fp);
    let lhs = diff
        .checked_mul(10_000)
        .ok_or_else(|| error!(SprentError::MathOverflow))?;
    let rhs = oracle_fp
        .checked_mul(band_bps as u128)
        .ok_or_else(|| error!(SprentError::MathOverflow))?;
    Ok(lhs <= rhs)
}

pub fn bid_notional_quote(base_atoms: u64, price_fp: u128) -> Result<u128> {
    let one = 10u128.pow(FP_DECIMALS as u32);
    let num = (base_atoms as u128)
        .checked_mul(price_fp)
        .ok_or_else(|| error!(SprentError::MathOverflow))?;
    Ok(num.div_ceil(one))
}

pub const ORACLE_PYTH: u8 = 0;

pub const ORACLE_PEG: u8 = 1;

#[derive(Clone, Copy, Debug, Default)]
pub struct OracleRead {
    pub price_fp: u128,

    pub freeze: u8,
    pub price: i64,
    pub conf: u64,
    pub expo: i32,
    pub publish_time: i64,
}

#[allow(clippy::too_many_arguments)]
pub fn read_oracle(
    mode: u8,
    ai: &AccountInfo,
    feed: &[u8; 32],
    now_ts: i64,
    max_age_s: u32,
    max_conf_bps: u16,
    base_decimals: u8,
    quote_decimals: u8,
) -> Result<OracleRead> {
    use crate::state::freeze_reason as fr;
    let exp_shift = quote_decimals as i32 - base_decimals as i32 + FP_DECIMALS;
    match mode {
        ORACLE_PEG => {
            require_keys_eq!(
                *ai.key,
                anchor_lang::system_program::ID,
                SprentError::OracleOwnerMismatch
            );
            let price_fp =
                scale_pow10(1, exp_shift).ok_or_else(|| error!(SprentError::PriceOutOfRange))?;
            Ok(OracleRead {
                price_fp,
                freeze: fr::NONE,
                price: 1,
                conf: 0,
                expo: 0,
                publish_time: now_ts,
            })
        }
        ORACLE_PYTH => {
            let px = read_price_update(ai, feed)?;
            let mut r = OracleRead {
                price_fp: 0,
                freeze: fr::NONE,
                price: px.price,
                conf: px.conf,
                expo: px.expo,
                publish_time: px.publish_time,
            };
            if px.price <= 0 {
                r.freeze = fr::ORACLE_PRICE;
                return Ok(r);
            }
            r.price_fp = scale_pow10(px.price as u128, px.expo + exp_shift)
                .ok_or_else(|| error!(SprentError::PriceOutOfRange))?;
            if now_ts.saturating_sub(px.publish_time) > max_age_s as i64 {
                r.freeze = fr::ORACLE_STALE;
            } else if (px.conf as u128) * 10_000 > (px.price as u128) * max_conf_bps as u128 {
                r.freeze = fr::ORACLE_CONFIDENCE;
            }
            Ok(r)
        }
        _ => err!(SprentError::InvalidParams),
    }
}

