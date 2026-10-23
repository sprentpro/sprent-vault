pub const PPS_SCALE: u128 = 1_000_000_000_000;

pub const BOOTSTRAP_SHARES: u128 = 1_000;

pub const PRICE_FP_ONE: u128 = 1_000_000_000_000_000_000_000_000;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AcctError {
    ZeroShares,
    Overflow,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Minted {
    pub user_shares: u128,
    pub dead_shares: u128,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct EpochPrice {
    pub nav: u128,
    pub shares: u128,
    pub pps: u128,
}

pub fn mint_shares(
    amount: u64,
    ep: &EpochPrice,
    current_total_shares: u128,
) -> Result<Minted, AcctError> {
    let a = amount as u128;
    let minted = if ep.shares == 0 {
        if ep.pps == 0 {
            return Err(AcctError::Overflow);
        }
        a.checked_mul(PPS_SCALE).ok_or(AcctError::Overflow)? / ep.pps
    } else {
        if ep.nav == 0 {
            return Err(AcctError::ZeroShares);
        }
        a.checked_mul(ep.shares).ok_or(AcctError::Overflow)? / ep.nav
    };
    if current_total_shares == 0 {
        if minted <= BOOTSTRAP_SHARES {
            return Err(AcctError::ZeroShares);
        }
        return Ok(Minted {
            user_shares: minted - BOOTSTRAP_SHARES,
            dead_shares: BOOTSTRAP_SHARES,
        });
    }
    if minted == 0 {
        return Err(AcctError::ZeroShares);
    }
    Ok(Minted {
        user_shares: minted,
        dead_shares: 0,
    })
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Crystallized {
    pub fee_quote: u64,
    pub fee_shares: u128,
    pub shares_after: u128,
    pub hwm_after: u128,
}

pub fn crystallize(
    shares: u128,
    hwm: u128,
    pps: u128,
    fee_bps: u16,
) -> Result<Crystallized, AcctError> {
    if shares == 0 || pps <= hwm {
        return Ok(Crystallized {
            fee_quote: 0,
            fee_shares: 0,
            shares_after: shares,
            hwm_after: hwm,
        });
    }
    let gain = pps - hwm;
    let num = (fee_bps as u128)
        .checked_mul(gain)
        .and_then(|x| x.checked_mul(shares))
        .ok_or(AcctError::Overflow)?;

    let den = 10_000u128 * PPS_SCALE;
    if num < den {
        return Ok(Crystallized {
            fee_quote: 0,
            fee_shares: 0,
            shares_after: shares,
            hwm_after: pps,
        });
    }
    let fee = num.div_ceil(den);
    let fee_shares = fee
        .checked_mul(PPS_SCALE)
        .ok_or(AcctError::Overflow)?
        .div_ceil(pps)
        .min(shares);
    let fee_quote = u64::try_from(fee).map_err(|_| AcctError::Overflow)?;
    Ok(Crystallized {
        fee_quote,
        fee_shares,
        shares_after: shares - fee_shares,
        hwm_after: pps,
    })
}

pub fn redeem_quote(shares: u128, ep: &EpochPrice) -> Result<u64, AcctError> {
    if ep.shares == 0 {
        return Ok(0);
    }
    let v = shares.checked_mul(ep.nav).ok_or(AcctError::Overflow)? / ep.shares;
    u64::try_from(v).map_err(|_| AcctError::Overflow)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NavInputs {
    pub tracked_idle_quote: u64,
    pub tracked_idle_base: u64,

    pub seat_quote: u64,
    pub seat_base: u64,
    pub pending_deposits_quote: u64,
    pub payable_withdrawals_quote: u64,
    pub fee_payable_quote: u64,
    pub price_fp: u128,
}

pub fn nav_quote(i: &NavInputs) -> Result<i128, AcctError> {
    let base = (i.tracked_idle_base as u128)
        .checked_add(i.seat_base as u128)
        .ok_or(AcctError::Overflow)?;
    let base_val = base.checked_mul(i.price_fp).ok_or(AcctError::Overflow)? / PRICE_FP_ONE;
    let assets = (i.tracked_idle_quote as i128)
        + (i.seat_quote as i128)
        + i128::try_from(base_val).map_err(|_| AcctError::Overflow)?;
    Ok(assets
        - i.pending_deposits_quote as i128
        - i.payable_withdrawals_quote as i128
        - i.fee_payable_quote as i128)
}

