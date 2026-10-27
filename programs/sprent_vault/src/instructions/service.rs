use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct DistributeService<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [PLATFORM_SEED], bump = platform.bump, has_one = admin @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    /// CHECK: S authority PDA
    #[account(seeds = [TREASURY_S_SEED], bump = platform.s_bump)]
    pub treasury_s: UncheckedAccount<'info>,
    #[account(mut, address = platform.s_quote_ata @ SprentError::AccountSubstitution)]
    pub s_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = platform.t_quote_ata @ SprentError::AccountSubstitution)]
    pub t_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut, address = platform.bb_quote_ata @ SprentError::AccountSubstitution)]
    pub bb_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(mut, address = stake_pool.reward_vault @ SprentError::AccountSubstitution)]
    pub reward_vault: Box<Account<'info, TokenAccount>>,
    #[account(
        init,
        payer = admin,
        space = 8 + RewardWindow::INIT_SPACE,
        seeds = [REWARD_WINDOW_SEED, stake_pool.current_window.to_le_bytes().as_ref()],
        bump
    )]
    pub reward_window: Box<Account<'info, RewardWindow>>,
    #[account(address = platform.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTeam<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [PLATFORM_SEED], bump = platform.bump, has_one = admin @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    /// CHECK: T authority PDA
    #[account(seeds = [TREASURY_T_SEED], bump = platform.t_bump)]
    pub treasury_t: UncheckedAccount<'info>,
    #[account(mut, address = platform.t_quote_ata @ SprentError::AccountSubstitution)]
    pub t_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint)]
    pub destination: Box<Account<'info, TokenAccount>>,
    #[account(address = platform.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
}

pub(crate) fn distribute_service(
    ctx: Context<DistributeService>,
    declared_costs: u64,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    {
        let p = &ctx.accounts.platform;
        let cw = ctx.accounts.stake_pool.current_window;
        for i in 0..MAX_EXCLUDED {
            let a = p.excluded_add_window[i];
            require!(
                p.excluded[i] == Pubkey::default()
                    || a & EXCLUSION_PENDING == 0
                    || (a & !EXCLUSION_PENDING) > cw,
                SprentError::ExclusionNotApplied
            );
        }
    }
    let bal = ctx.accounts.s_quote_ata.amount;
    require!(declared_costs <= bal, SprentError::InvalidParams);
    let net = bal - declared_costs;
    let p = &ctx.accounts.platform;
    let holder = ((net as u128) * p.holder_bps as u128 / BPS_DENOMINATOR as u128) as u64;
    let bb = ((net as u128) * p.buyback_bps as u128 / BPS_DENOMINATOR as u128) as u64;
    let team = net - holder - bb;
    let seeds: &[&[u8]] = &[TREASURY_S_SEED, &[p.s_bump]];
    let a = &ctx.accounts;
    for (to, amt) in [
        (a.t_quote_ata.to_account_info(), declared_costs + team),
        (a.reward_vault.to_account_info(), holder),
        (a.bb_quote_ata.to_account_info(), bb),
    ] {
        if amt > 0 {
            token::transfer_checked(
                CpiContext::new_with_signer(
                    a.token_program.to_account_info(),
                    TransferChecked {
                        from: a.s_quote_ata.to_account_info(),
                        mint: a.quote_mint.to_account_info(),
                        to,
                        authority: a.treasury_s.to_account_info(),
                    },
                    &[seeds],
                ),
                amt,
                a.quote_mint.decimals,
            )?;
        }
    }

    let sp = &mut ctx.accounts.stake_pool;
    sp.update(now)?;
    let reward = holder
        .checked_add(sp.carry)
        .ok_or(SprentError::MathOverflow)?;
    let w = &mut ctx.accounts.reward_window;
    w.window = sp.current_window;
    w.start = sp.window_start;
    w.end = now;
    w.token_time = sp.window_token_time;
    w.bump = ctx.bumps.reward_window;
    w.index_before = sp.acc_index;
    if sp.window_token_time > 0 && reward > 0 {
        w.reward = reward;
        w.rate = (reward as u128)
            .checked_mul(RATE_SCALE)
            .ok_or(SprentError::MathOverflow)?
            / sp.window_token_time;
        let len = (now - sp.window_start).max(0) as u128;
        sp.acc_index = sp
            .acc_index
            .checked_add(w.rate.checked_mul(len).ok_or(SprentError::MathOverflow)?)
            .ok_or(SprentError::MathOverflow)?;
        sp.carry = 0;
    } else {
        w.reward = 0;
        w.rate = 0;
        sp.carry = reward;
    }
    w.index_after = sp.acc_index;
    sp.reward_in_total = sp
        .reward_in_total
        .checked_add(holder as u128)
        .ok_or(SprentError::MathOverflow)?;
    sp.current_window = sp
        .current_window
        .checked_add(1)
        .ok_or(SprentError::MathOverflow)?;
    sp.window_start = now;
    sp.window_token_time = 0;

    let p = &mut ctx.accounts.platform;
    p.s_dist_seq = p
        .s_dist_seq
        .checked_add(1)
        .ok_or(SprentError::MathOverflow)?;
    p.s_received_total = p
        .s_received_total
        .checked_add(bal as u128)
        .ok_or(SprentError::MathOverflow)?;
    p.s_costs_total = p
        .s_costs_total
        .checked_add(declared_costs as u128)
        .ok_or(SprentError::MathOverflow)?;
    p.holder_alloc_total = p
        .holder_alloc_total
        .checked_add(holder as u128)
        .ok_or(SprentError::MathOverflow)?;
    p.buyback_alloc_total = p
        .buyback_alloc_total
        .checked_add(bb as u128)
        .ok_or(SprentError::MathOverflow)?;
    p.team_alloc_total = p
        .team_alloc_total
        .checked_add(team as u128)
        .ok_or(SprentError::MathOverflow)?;
    emit!(ServiceDistributed {
        seq: p.s_dist_seq,
        gross: bal,
        costs: declared_costs,
        net,
        holder,
        buyback: bb,
        team,
        window: w.window,
        window_reward: w.reward,
        carry: sp.carry,
    });
    Ok(())
}

pub(crate) fn withdraw_team(ctx: Context<WithdrawTeam>, amount: u64) -> Result<()> {
    require!(amount > 0, SprentError::ZeroAmount);
    let p = &ctx.accounts.platform;
    let seeds: &[&[u8]] = &[TREASURY_T_SEED, &[p.t_bump]];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.t_quote_ata.to_account_info(),
                mint: ctx.accounts.quote_mint.to_account_info(),
                to: ctx.accounts.destination.to_account_info(),
                authority: ctx.accounts.treasury_t.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        ctx.accounts.quote_mint.decimals,
    )?;
    emit!(TeamWithdrawn {
        amount,
        destination: ctx.accounts.destination.key()
    });
    Ok(())
}
