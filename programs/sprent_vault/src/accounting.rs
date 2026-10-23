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

