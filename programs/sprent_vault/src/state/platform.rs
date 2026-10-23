use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::SprentError;

pub mod buyback_source {
    pub const CREATOR_FEE: u8 = 0;
    pub const SERVICE_FEE: u8 = 1;
}

pub mod buyback_state {
    pub const INTENT: u8 = 0;

    pub const BOUGHT: u8 = 1;

    pub const BURNED: u8 = 2;
}

#[account]
#[derive(InitSpace)]
pub struct Platform {
    pub admin: Pubkey,

    pub buyback_executor: Pubkey,
    pub quote_mint: Pubkey,

    pub sprent_mint: Pubkey,
    pub sprent_token_program: Pubkey,
    pub s_quote_ata: Pubkey,
    pub t_quote_ata: Pubkey,
    pub bb_quote_ata: Pubkey,
    pub burn_vault_ata: Pubkey,

    pub holder_bps: u16,
    pub buyback_bps: u16,
    pub stake_warmup_s: i64,
    pub stake_cooldown_s: i64,

    pub excluded: [Pubkey; MAX_EXCLUDED],
    pub excluded_len: u8,
    pub bump: u8,
    pub s_bump: u8,
    pub t_bump: u8,
    pub bb_bump: u8,
    pub burn_bump: u8,
    pub s_dist_seq: u64,
    pub s_received_total: u128,
    pub s_costs_total: u128,
    pub holder_alloc_total: u128,
    pub buyback_alloc_total: u128,
    pub team_alloc_total: u128,
    pub bought_total_creator: u128,
    pub bought_total_service: u128,
    pub burned_total_creator: u128,
    pub burned_total_service: u128,

    pub excluded_add_window: [u64; MAX_EXCLUDED],

    pub excluded_remove_window: [u64; MAX_EXCLUDED],
}

#[account]
#[derive(InitSpace)]
pub struct StakePool {
    pub mint: Pubkey,
    pub token_program: Pubkey,
    pub stake_vault: Pubkey,
    pub reward_vault: Pubkey,
    pub bump: u8,

    pub total_effective: u128,
    pub total_warming: u128,
    pub total_cooling: u128,

    pub current_window: u64,
    pub window_start: i64,

    pub window_token_time: u128,
    pub last_update: i64,

    pub acc_index: u128,

    pub carry: u64,
    pub reward_in_total: u128,
    pub reward_claimed_total: u128,
    pub reserved: [u8; 64],
}

#[account]
#[derive(InitSpace)]
pub struct RewardWindow {
    pub window: u64,
    pub start: i64,
    pub end: i64,
    pub reward: u64,
    pub token_time: u128,

    pub rate: u128,
    pub index_before: u128,
    pub index_after: u128,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub bump: u8,
    pub effective: u64,
    pub warming: u64,
    pub warm_until: i64,
    pub cooling: u64,
    pub cool_until: i64,

    pub window_token_time: u128,
    pub sync_window: u64,
    pub sync_ts: i64,
    pub reward_owed: u64,
    pub reward_claimed_total: u128,
    pub claim_nonce: u64,
    pub reserved: [u8; 64],
}

#[account]
#[derive(InitSpace)]
pub struct BuybackLedger {
    pub intent_id: u64,
    pub source: u8,
    pub state: u8,
    pub bump: u8,

    pub budget: u64,
    pub spent: u64,
    pub refunded: u64,
    pub bought: u64,
    pub burned: u64,
    pub buy_sig: [u8; 64],
    pub opened_slot: u64,
    pub bought_slot: u64,
    pub last_burn_slot: u64,
}

impl BuybackLedger {
    pub fn pending_burn(&self) -> u64 {
        self.bought - self.burned
    }
}

impl StakePool {
    pub fn update(&mut self, now: i64) -> Result<()> {
        if now > self.last_update {
            let dt = (now - self.last_update) as u128;
            self.window_token_time = self
                .window_token_time
                .checked_add(
                    self.total_effective
                        .checked_mul(dt)
                        .ok_or(SprentError::MathOverflow)?,
                )
                .ok_or(SprentError::MathOverflow)?;
            self.last_update = now;
        }
        Ok(())
    }
}

pub fn sync_stake(
    s: &mut StakeAccount,
    pool: &StakePool,
    closed: Option<&RewardWindow>,
    now: i64,
) -> Result<()> {
    if s.sync_window < pool.current_window {
        let w = closed.ok_or(error!(SprentError::RewardWindowMissing))?;
        require!(w.window == s.sync_window, SprentError::RewardWindowMissing);
        let seg = (s.effective as u128)
            .checked_mul((w.end - s.sync_ts.max(w.start)).max(0) as u128)
            .ok_or(SprentError::MathOverflow)?;
        let user_tt = s
            .window_token_time
            .checked_add(seg)
            .ok_or(SprentError::MathOverflow)?;
        let partial = user_tt
            .checked_mul(w.rate)
            .ok_or(SprentError::MathOverflow)?
            / RATE_SCALE;
        let full = (s.effective as u128)
            .checked_mul(pool.acc_index - w.index_after)
            .ok_or(SprentError::MathOverflow)?
            / RATE_SCALE;
        let add = u64::try_from(partial + full).map_err(|_| SprentError::MathOverflow)?;
        s.reward_owed = s
            .reward_owed
            .checked_add(add)
            .ok_or(SprentError::MathOverflow)?;
        s.sync_window = pool.current_window;
        s.window_token_time = (s.effective as u128)
            .checked_mul((now - pool.window_start).max(0) as u128)
            .ok_or(SprentError::MathOverflow)?;
    } else {
        let seg = (s.effective as u128)
            .checked_mul((now - s.sync_ts).max(0) as u128)
            .ok_or(SprentError::MathOverflow)?;
        s.window_token_time = s
            .window_token_time
            .checked_add(seg)
            .ok_or(SprentError::MathOverflow)?;
    }
    s.sync_ts = now;
    Ok(())
}

pub(crate) fn excluded_at(p: &Platform, k: &Pubkey, w: u64) -> bool {
    (0..MAX_EXCLUDED).any(|i| {
        p.excluded[i] == *k
            && *k != Pubkey::default()
            && (p.excluded_add_window[i] & !EXCLUSION_PENDING) <= w
            && (p.excluded_remove_window[i] == 0 || w < p.excluded_remove_window[i])
    })
}

pub(crate) fn excluded_listed(p: &Platform, k: &Pubkey) -> bool {
    (0..MAX_EXCLUDED)
        .any(|i| p.excluded[i] == *k && *k != Pubkey::default() && p.excluded_remove_window[i] == 0)
}

pub(crate) fn recount_excluded(p: &mut Platform) {
    p.excluded_len = (0..MAX_EXCLUDED)
        .filter(|&i| p.excluded[i] != Pubkey::default())
        .count() as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool() -> StakePool {
        StakePool {
            mint: Pubkey::default(),
            token_program: Pubkey::default(),
            stake_vault: Pubkey::default(),
            reward_vault: Pubkey::default(),
            bump: 0,
            total_effective: 0,
            total_warming: 0,
            total_cooling: 0,
            current_window: 0,
            window_start: 0,
            window_token_time: 0,
            last_update: 0,
            acc_index: 0,
            carry: 0,
            reward_in_total: 0,
            reward_claimed_total: 0,
            reserved: [0; 64],
        }
    }
    fn acct() -> StakeAccount {
        StakeAccount {
            owner: Pubkey::new_unique(),
            bump: 0,
            effective: 0,
            warming: 0,
            warm_until: 0,
            cooling: 0,
            cool_until: 0,
            window_token_time: 0,
            sync_window: 0,
            sync_ts: 0,
            reward_owed: 0,
            reward_claimed_total: 0,
            claim_nonce: 0,
            reserved: [0; 64],
        }
    }
    fn add(p: &mut StakePool, s: &mut StakeAccount, w: Option<&RewardWindow>, now: i64, amt: u64) {
        p.update(now).unwrap();
        sync_stake(s, p, w, now).unwrap();
        s.effective += amt;
        p.total_effective += amt as u128;
    }
    fn close(p: &mut StakePool, now: i64, reward: u64) -> RewardWindow {
        p.update(now).unwrap();
        let rate = if p.window_token_time > 0 {
            reward as u128 * RATE_SCALE / p.window_token_time
        } else {
            0
        };
        let before = p.acc_index;
        p.acc_index += rate * (now - p.window_start) as u128;
        let w = RewardWindow {
            window: p.current_window,
            start: p.window_start,
            end: now,
            reward,
            token_time: p.window_token_time,
            rate,
            index_before: before,
            index_after: p.acc_index,
            bump: 0,
        };
        p.current_window += 1;
        p.window_start = now;
        p.window_token_time = 0;
        w
    }

    #[test]
    fn time_weighted_split_and_conservation() {
        let mut p = pool();
        let (mut a, mut b) = (acct(), acct());
        add(&mut p, &mut a, None, 0, 1_000);
        add(&mut p, &mut b, None, 43_200, 1_000);
        let w0 = close(&mut p, 86_400, 3_000_000);

        sync_stake(&mut a, &p, Some(&w0), 86_400).unwrap();
        sync_stake(&mut b, &p, Some(&w0), 86_400).unwrap();
        assert!(a.reward_owed.abs_diff(2_000_000) <= 1 && b.reward_owed.abs_diff(1_000_000) <= 1);
        assert!(a.reward_owed + b.reward_owed <= 3_000_000);

        let w1 = close(&mut p, 172_800, 1_000_000);
        let _w2 = close(&mut p, 259_200, 1_000_000);
        sync_stake(&mut a, &p, Some(&w1), 259_200).unwrap();
        assert!(
            a.reward_owed <= 3_000_000 && 3_000_000 - a.reward_owed <= 5,
            "owed={}",
            a.reward_owed
        );
    }

    #[test]
    fn flash_stake_gets_only_its_seconds() {
        let mut p = pool();
        let (mut a, mut f) = (acct(), acct());
        add(&mut p, &mut a, None, 0, 1_000_000);
        add(&mut p, &mut f, None, 86_399, 1_000_000);
        let w0 = close(&mut p, 86_400, 1_000_000);
        sync_stake(&mut f, &p, Some(&w0), 86_400).unwrap();
        assert!(f.reward_owed <= 12);
    }

    #[test]
    fn reward_with_no_stakers_carries_rate_zero() {
        let mut p = pool();
        let w = close(&mut p, 100, 5_000);
        assert_eq!(w.rate, 0);
    }
}
