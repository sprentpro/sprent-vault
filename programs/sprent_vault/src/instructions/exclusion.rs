use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::instructions::*;
use crate::state::*;

#[derive(Accounts)]
pub struct SetExcluded<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [PLATFORM_SEED], bump = platform.bump, has_one = admin @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,

    #[account(seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Option<Box<Account<'info, StakePool>>>,
}

pub(crate) fn set_excluded<'info>(
    ctx: Context<'_, '_, '_, 'info, SetExcluded<'info>>,
    add: Vec<Pubkey>,
    remove: Vec<Pubkey>,
) -> Result<()> {
    require!(
        !add.is_empty() || !remove.is_empty(),
        SprentError::InvalidParams
    );
    require!(
        ctx.remaining_accounts.len() == add.len(),
        SprentError::InvalidParams
    );
    let (now_w, eff) = match &ctx.accounts.stake_pool {
        Some(sp) => (sp.current_window, sp.current_window + 1),
        None => (0, 0),
    };
    let p = &mut ctx.accounts.platform;
    for (i, k) in add.iter().enumerate() {
        require!(
            *k != Pubkey::default() && !excluded_listed(p, k),
            SprentError::InvalidParams
        );
        let ra = &ctx.remaining_accounts[i];
        let (pda, _) = Pubkey::find_program_address(&[STAKE_SEED, k.as_ref()], ctx.program_id);
        require_keys_eq!(ra.key(), pda, SprentError::AccountSubstitution);
        let mut flag = 0u64;
        if ra.owner == ctx.program_id && !ra.data_is_empty() {
            let sa = StakeAccount::try_deserialize(&mut &ra.try_borrow_data()?[..])?;
            if sa.effective > 0 || sa.warming > 0 {
                flag = EXCLUSION_PENDING;
            }
        }
        let slot = (0..MAX_EXCLUDED)
            .find(|&j| {
                p.excluded[j] == Pubkey::default()
                    || (p.excluded_remove_window[j] != 0 && p.excluded_remove_window[j] <= now_w)
            })
            .ok_or(SprentError::ExcludedListFull)?;
        p.excluded[slot] = *k;
        p.excluded_add_window[slot] = eff | flag;
        p.excluded_remove_window[slot] = 0;
        emit!(ExclusionChanged {
            wallet: *k,
            excluded: true,
            effective_window: eff,
            stake_to_remove: flag != 0
        });
    }
    for k in remove.iter() {
        let slot = (0..MAX_EXCLUDED)
            .find(|&j| p.excluded[j] == *k && p.excluded_remove_window[j] == 0)
            .ok_or(SprentError::InvalidParams)?;
        let added = p.excluded_add_window[slot] & !EXCLUSION_PENDING;
        if eff == 0 || added >= eff {
            p.excluded[slot] = Pubkey::default();
            p.excluded_add_window[slot] = 0;
            p.excluded_remove_window[slot] = 0;
        } else {
            p.excluded_add_window[slot] = added;
            p.excluded_remove_window[slot] = eff;
        }
        emit!(ExclusionChanged {
            wallet: *k,
            excluded: false,
            effective_window: eff,
            stake_to_remove: false
        });
    }
    recount_excluded(p);
    Ok(())
}

pub(crate) fn apply_exclusion(ctx: Context<StakeCrank>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let window = ctx.accounts.closed_window.as_ref().map(|w| &***w);
    let sp = &mut ctx.accounts.stake_pool;
    let s = &mut ctx.accounts.stake_account;
    let p = &mut ctx.accounts.platform;
    require!(
        excluded_at(p, &s.owner, sp.current_window),
        SprentError::InvalidParams
    );
    if s.effective == 0 && s.warming == 0 {
        let mut cleared = false;
        for i in 0..MAX_EXCLUDED {
            if p.excluded[i] == s.owner && p.excluded_add_window[i] & EXCLUSION_PENDING != 0 {
                p.excluded_add_window[i] &= !EXCLUSION_PENDING;
                cleared = true;
            }
        }
        require!(cleared, SprentError::NothingToSettle);
        emit!(ExclusionApplied {
            owner: s.owner,
            window: sp.current_window,
            removed_effective: 0,
            removed_warming: 0
        });
        return Ok(());
    }
    sp.update(now)?;
    sync_stake(s, sp, window, now)?;
    sp.window_token_time = sp.window_token_time.saturating_sub(s.window_token_time);
    s.window_token_time = 0;
    let (e, w) = (s.effective, s.warming);
    s.effective = 0;
    s.warming = 0;
    s.cooling = s
        .cooling
        .checked_add(e + w)
        .ok_or(SprentError::MathOverflow)?;
    s.cool_until = now
        .checked_add(p.stake_cooldown_s)
        .ok_or(SprentError::MathOverflow)?;
    sp.total_effective -= e as u128;
    sp.total_warming -= w as u128;
    sp.total_cooling = sp
        .total_cooling
        .checked_add((e + w) as u128)
        .ok_or(SprentError::MathOverflow)?;
    for i in 0..MAX_EXCLUDED {
        if p.excluded[i] == s.owner {
            p.excluded_add_window[i] &= !EXCLUSION_PENDING;
        }
    }
    emit!(ExclusionApplied {
        owner: s.owner,
        window: sp.current_window,
        removed_effective: e,
        removed_warming: w
    });
    Ok(())
}
