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

