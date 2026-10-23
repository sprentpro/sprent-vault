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

pub fn pps_of(nav: u128, total_shares: u128, last_pps: u128) -> Result<u128, AcctError> {
    if total_shares == 0 {
        return Ok(last_pps);
    }
    Ok(nav.checked_mul(PPS_SCALE).ok_or(AcctError::Overflow)? / total_shares)
}

#[cfg(test)]
mod tests {

    use super::*;

    const U: u128 = 1_000_000;
    fn usd(x: u128) -> u128 {
        x * U
    }
    fn pps(x_milli: u128) -> u128 {
        x_milli * PPS_SCALE / 1000
    }

    fn near(a: u128, b: u128, tol: u128) -> bool {
        a.abs_diff(b) <= tol
    }
    fn ep(nav: u128, shares: u128) -> EpochPrice {
        EpochPrice {
            nav,
            shares,
            pps: pps_of(nav, shares, PPS_SCALE).unwrap(),
        }
    }

    fn ep_at(x_milli: u128) -> EpochPrice {
        ep(usd(x_milli) * 1_000, usd(1_000_000))
    }

    struct Pos {
        shares: u128,
        hwm: u128,
    }

    #[test]
    fn v1_to_v8_one_vault() {
        let fee_bps = 1000u16;
        let mut total_shares: u128 = 0;
        let mut assets: u128 = 0;

        let first = EpochPrice {
            nav: 0,
            shares: 0,
            pps: PPS_SCALE,
        };
        let m = mint_shares(usd(120_000) as u64, &first, total_shares).unwrap();
        let mut a = Pos {
            shares: m.user_shares,
            hwm: PPS_SCALE,
        };
        total_shares += m.user_shares + m.dead_shares;
        assets += usd(120_000);
        assert_eq!(a.shares, usd(120_000) - BOOTSTRAP_SHARES);
        assert_eq!(pps_of(assets, total_shares, PPS_SCALE).unwrap(), PPS_SCALE);

        assets += usd(12_000);
        let p = pps_of(assets, total_shares, 0).unwrap();
        assert_eq!(p, pps(1100));
        let c = crystallize(a.shares, a.hwm, p, fee_bps).unwrap();
        assert!(near(c.fee_quote as u128, usd(1_200), U));
        assert_eq!(c.fee_quote, 1_199_999_990);
        assert_eq!(c.fee_shares, 1_090_909_082);
        a.shares = c.shares_after;
        a.hwm = c.hwm_after;
        total_shares -= c.fee_shares;
        assets -= c.fee_quote as u128;
        assert_eq!(a.shares / U, 118_909);
        assert_eq!(assets / U, 130_800);
        assert_eq!(a.hwm, pps(1100));
        assert!(near(pps_of(assets, total_shares, 0).unwrap(), pps(1100), 1));

        let e3 = ep(assets, total_shares);
        let m = mint_shares(usd(55_000) as u64, &e3, total_shares).unwrap();
        let mut b = Pos {
            shares: m.user_shares,
            hwm: e3.pps,
        };
        total_shares += m.user_shares;
        assets += usd(55_000);
        assert!(near(b.shares, usd(50_000), 10));
        assert!(b.shares <= usd(55_000) * PPS_SCALE / e3.pps);
        assert_eq!(total_shares / U, 168_909);
        assert_eq!(assets / U, 185_800);

        assets -= usd(18_580);
        let p4 = pps_of(assets, total_shares, 0).unwrap();
        assert!(near(p4, pps(990), 1_000));
        for pos in [&mut a, &mut b] {
            let c = crystallize(pos.shares, pos.hwm, p4, fee_bps).unwrap();
            assert_eq!(c.fee_quote, 0);
            assert!(c.hwm_after >= pps(1100));
        }

        assets += usd(18_580);
        let p5 = pps_of(assets, total_shares, 0).unwrap();
        assert!(near(p5, pps(1100), 1_000), "p5={p5}");
        for pos in [&a, &b] {
            assert_eq!(
                crystallize(pos.shares, pos.hwm, p5, fee_bps)
                    .unwrap()
                    .fee_quote,
                0
            );
        }

        assets += 16_890_900_000;
        let e6 = ep(assets, total_shares);
        let p6 = e6.pps;
        assert!(near(p6, pps(1200), 100_000));
        let ca = crystallize(a.shares, a.hwm, p6, fee_bps).unwrap();
        assert!(near(ca.fee_quote as u128, 1_189_090_000, 20_000));

        let cb = crystallize(b.shares, b.hwm, p6, fee_bps).unwrap();
        assert!(near(cb.fee_quote as u128, usd(500), 20_000));

        assert_eq!(
            cb.fee_quote,
            crystallize(b.shares, b.hwm, p6, fee_bps).unwrap().fee_quote
        );
        a.shares = ca.shares_after;
        a.hwm = ca.hwm_after;
        b.shares = cb.shares_after;
        total_shares -= ca.fee_shares + cb.fee_shares;
        assets -= (ca.fee_quote + cb.fee_quote) as u128;
        let redeem_b = redeem_quote(b.shares, &e6).unwrap();
        total_shares -= b.shares;
        assets -= redeem_b as u128;
        assert!(near(redeem_b as u128, (b.shares) * p6 / PPS_SCALE, 2));

        assert_eq!(
            crystallize(a.shares, a.hwm, p6, fee_bps).unwrap().fee_quote,
            0
        );
        let redeem_a = redeem_quote(a.shares, &e6).unwrap();
        total_shares -= a.shares;
        assets -= redeem_a as u128;

        assert_eq!(total_shares, BOOTSTRAP_SHARES);
        assert!(assets >= BOOTSTRAP_SHARES * p6 / PPS_SCALE);
    }

    #[test]
    fn v9_loss_then_entry_pays_on_recovery() {
        let d = mint_shares(usd(8_000) as u64, &ep_at(800), 1)
            .unwrap()
            .user_shares;
        assert_eq!(d, usd(10_000));
        let fee_d = crystallize(d, pps(800), pps(1000), 1000).unwrap().fee_quote;
        assert_eq!(fee_d as u128, usd(10_000) * 2 / 10 / 10);
        let fee_e = crystallize(usd(10_000), pps(1000), pps(1000), 1000)
            .unwrap()
            .fee_quote;
        assert_eq!(fee_e, 0);

        let single = crystallize(d, pps(1000), pps(1000), 1000)
            .unwrap()
            .fee_quote;
        assert_eq!(single, 0);
        assert_ne!(single, fee_d);
    }

    #[test]
    fn v10_peak_entry_flat_not_charged() {
        let f = mint_shares(usd(1_200) as u64, &ep_at(1200), 1)
            .unwrap()
            .user_shares;
        assert_eq!(f, usd(1_000));
        assert_eq!(
            crystallize(f, pps(1200), pps(1200), 1000)
                .unwrap()
                .fee_quote,
            0
        );

        assert!(
            crystallize(f, pps(1000), pps(1200), 1000)
                .unwrap()
                .fee_quote
                > 0
        );
    }

    #[test]
    fn v11_withdraw_redeposit_cannot_avoid_fee() {
        let g = usd(10_000);
        let stay = crystallize(g, pps(1000), pps(1300), 1000).unwrap();
        let leave = crystallize(g, pps(1000), pps(1300), 1000).unwrap();
        assert_eq!(stay.fee_quote, leave.fee_quote);
        assert_eq!(leave.fee_quote as u128, usd(300));
        let e = ep_at(1300);
        let redeem = redeem_quote(leave.shares_after, &e).unwrap();
        let back = mint_shares(redeem, &e, 1).unwrap().user_shares;
        assert!(back <= leave.shares_after);
        assert_eq!(
            crystallize(back, pps(1300), pps(1300), 1000)
                .unwrap()
                .fee_quote,
            0
        );
    }

    #[test]
    fn v12_donation_does_not_move_pps() {
        let base = NavInputs {
            tracked_idle_quote: usd(100_000) as u64,
            ..Default::default()
        };
        let nav = nav_quote(&base).unwrap() as u128;

        let ata_after_donation = base.tracked_idle_quote + usd(1_000_000) as u64;
        assert!(ata_after_donation > base.tracked_idle_quote);
        assert_eq!(pps_of(nav, usd(100_000), 0).unwrap(), PPS_SCALE);
    }

    #[test]
    fn v13_zero_share_deposit_rejected() {
        assert_eq!(mint_shares(1, &ep(2, 1), 1), Err(AcctError::ZeroShares));
        let first = EpochPrice {
            nav: 0,
            shares: 0,
            pps: PPS_SCALE,
        };
        assert_eq!(mint_shares(1_000, &first, 0), Err(AcctError::ZeroShares));
        assert_eq!(mint_shares(10, &ep(0, 5), 5), Err(AcctError::ZeroShares));
    }

    #[test]
    fn v14_round_trip_rounds_for_the_vault() {
        let e = ep(11, 10);
        let s = mint_shares(100, &e, 10).unwrap().user_shares;
        assert_eq!(s, 90);
        assert_eq!(redeem_quote(s, &e).unwrap(), 99);
    }

    #[test]
    fn v15_fee_rounds_up() {
        let c = crystallize(123_450, pps(1000), pps(1100), 1000).unwrap();
        assert_eq!(c.fee_quote, 1235);
        assert_eq!(c.fee_shares, (1235u128 * PPS_SCALE).div_ceil(pps(1100)));

        assert_eq!(
            crystallize(1, pps(1000), pps(1100), 1000)
                .unwrap()
                .fee_quote,
            0
        );
        assert_eq!(
            crystallize(100, pps(1000), pps(1100), 1000)
                .unwrap()
                .fee_quote,
            1
        );
        assert_eq!(
            crystallize(101, pps(1000), pps(1100), 1000)
                .unwrap()
                .fee_quote,
            2
        );
    }

    #[test]
    fn deposit_then_redeem_never_profits() {
        let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let mut minted_cases = 0u32;
        for i in 0..200_000u32 {
            let shares = 1_000 + (next() % 1_000_000_000_000_000) as u128;

            let ratio_milli = 1 + (next() % 1_000_000) as u128;
            let nav = (shares * ratio_milli / 1_000).max(1);
            let amount = match i % 4 {
                0 => 1 + next() % 10,
                1 => 1 + next() % 1_000_000,
                _ => 1 + next() % 20_000_000_000,
            };
            let e = ep(nav, shares);
            let Ok(m) = mint_shares(amount, &e, shares) else {
                continue;
            };
            minted_cases += 1;

            assert!(
                redeem_quote(m.user_shares, &e).unwrap() <= amount,
                "same-epoch i={i}"
            );

            let e2 = ep(nav + amount as u128, shares + m.user_shares);
            assert!(
                redeem_quote(m.user_shares, &e2).unwrap() <= amount,
                "next-epoch i={i}"
            );
        }
        assert!(minted_cases > 100_000);

        let first = EpochPrice {
            nav: 0,
            shares: 0,
            pps: PPS_SCALE,
        };
        let m = mint_shares(1_001, &first, 0).unwrap();
        assert_eq!(m.user_shares, 1);
        assert!(redeem_quote(m.user_shares, &ep(1_001, 1_001)).unwrap() <= 1_001);

        assert!(mint_shares(20_000_000_000, &ep(1, 1_000_000_000_000_000), 1).is_ok());
    }

    #[test]
    fn nav_excludes_liabilities_and_marks_base() {
        let i = NavInputs {
            tracked_idle_quote: 1_000,
            tracked_idle_base: 500_000_000,
            seat_quote: 200,
            seat_base: 500_000_000,
            pending_deposits_quote: 300,
            payable_withdrawals_quote: 100,
            fee_payable_quote: 50,
            price_fp: 150_000_000_000_000_000_000_000,
        };
        assert_eq!(
            nav_quote(&i).unwrap(),
            1_000 + 200 + 150_000_000 - 300 - 100 - 50
        );
    }
}
