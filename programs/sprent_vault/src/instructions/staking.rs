use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, TransferChecked};
use anchor_spl::token_interface::{
    self, Mint as IMint, TokenAccount as ITokenAccount, TokenInterface,
};

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::state::*;

#[derive(Accounts)]
pub struct StakeOp<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [PLATFORM_SEED], bump = platform.bump)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + StakeAccount::INIT_SPACE,
        seeds = [STAKE_SEED, owner.key().as_ref()],
        bump
    )]
    pub stake_account: Box<Account<'info, StakeAccount>>,

    pub closed_window: Option<Box<Account<'info, RewardWindow>>>,
    #[account(address = stake_pool.mint @ SprentError::AccountSubstitution)]
    pub sprent_mint: Box<InterfaceAccount<'info, IMint>>,
    #[account(mut, token::mint = sprent_mint, token::authority = owner)]
    pub owner_sprent: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(mut, address = stake_pool.stake_vault @ SprentError::AccountSubstitution)]
    pub stake_vault: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(address = stake_pool.token_program @ SprentError::AccountSubstitution)]
    pub sprent_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StakeCrank<'info> {
    #[account(mut, seeds = [PLATFORM_SEED], bump = platform.bump)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(mut, seeds = [STAKE_SEED, stake_account.owner.as_ref()], bump = stake_account.bump)]
    pub stake_account: Box<Account<'info, StakeAccount>>,
    pub closed_window: Option<Box<Account<'info, RewardWindow>>>,
}

#[derive(Accounts)]
pub struct StakeOwnerOp<'info> {
    pub owner: Signer<'info>,
    #[account(seeds = [PLATFORM_SEED], bump = platform.bump)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(mut, seeds = [STAKE_SEED, owner.key().as_ref()], bump = stake_account.bump, has_one = owner @ SprentError::DepositorOwnerMismatch)]
    pub stake_account: Box<Account<'info, StakeAccount>>,
    pub closed_window: Option<Box<Account<'info, RewardWindow>>>,
}

#[derive(Accounts)]
pub struct StakeTokenOut<'info> {
    pub owner: Signer<'info>,
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(mut, seeds = [STAKE_SEED, owner.key().as_ref()], bump = stake_account.bump, has_one = owner @ SprentError::DepositorOwnerMismatch)]
    pub stake_account: Box<Account<'info, StakeAccount>>,
    #[account(address = stake_pool.mint @ SprentError::AccountSubstitution)]
    pub sprent_mint: Box<InterfaceAccount<'info, IMint>>,
    #[account(mut, token::mint = sprent_mint, token::authority = owner)]
    pub owner_sprent: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(mut, address = stake_pool.stake_vault @ SprentError::AccountSubstitution)]
    pub stake_vault: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(address = stake_pool.token_program @ SprentError::AccountSubstitution)]
    pub sprent_token_program: Interface<'info, TokenInterface>,
}

#[derive(Accounts)]
pub struct ClaimReward<'info> {
    pub owner: Signer<'info>,
    #[account(seeds = [PLATFORM_SEED], bump = platform.bump)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mut, seeds = [STAKE_POOL_SEED], bump = stake_pool.bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(mut, seeds = [STAKE_SEED, owner.key().as_ref()], bump = stake_account.bump, has_one = owner @ SprentError::DepositorOwnerMismatch)]
    pub stake_account: Box<Account<'info, StakeAccount>>,
    pub closed_window: Option<Box<Account<'info, RewardWindow>>>,
    #[account(mut, address = stake_pool.reward_vault @ SprentError::AccountSubstitution)]
    pub reward_vault: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint, token::authority = owner)]
    pub owner_quote: Box<Account<'info, TokenAccount>>,
    #[account(address = platform.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
}

pub(crate) fn stake(ctx: Context<StakeOp>, amount: u64) -> Result<()> {
    require!(amount > 0, SprentError::ZeroAmount);
    let owner = ctx.accounts.owner.key();
    {
        let p = &ctx.accounts.platform;
        let w = ctx.accounts.stake_pool.current_window;
        require!(
            !excluded_listed(p, &owner) && !excluded_at(p, &owner, w),
            SprentError::StakerExcluded
        );
    }
    let now = Clock::get()?.unix_timestamp;
    let before = ctx.accounts.stake_vault.amount;
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.sprent_token_program.to_account_info(),
            token_interface::TransferChecked {
                from: ctx.accounts.owner_sprent.to_account_info(),
                mint: ctx.accounts.sprent_mint.to_account_info(),
                to: ctx.accounts.stake_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.sprent_mint.decimals,
    )?;
    ctx.accounts.stake_vault.reload()?;
    let received = ctx
        .accounts
        .stake_vault
        .amount
        .checked_sub(before)
        .ok_or(SprentError::MathOverflow)?;
    let window = ctx.accounts.closed_window.as_ref().map(|w| &***w);
    let sp = &mut ctx.accounts.stake_pool;
    sp.update(now)?;
    let s = &mut ctx.accounts.stake_account;
    if s.owner == Pubkey::default() {
        s.owner = owner;
        s.bump = ctx.bumps.stake_account;
        s.sync_window = sp.current_window;
        s.sync_ts = now;
    }
    require_keys_eq!(s.owner, owner, SprentError::DepositorOwnerMismatch);
    sync_stake(s, sp, window, now)?;
    s.warming = s
        .warming
        .checked_add(received)
        .ok_or(SprentError::MathOverflow)?;
    s.warm_until = now
        .checked_add(ctx.accounts.platform.stake_warmup_s)
        .ok_or(SprentError::MathOverflow)?;
    sp.total_warming = sp
        .total_warming
        .checked_add(received as u128)
        .ok_or(SprentError::MathOverflow)?;
    emit!(Staked {
        owner,
        amount: received,
        warm_until: s.warm_until
    });
    Ok(())
}

pub(crate) fn activate_stake(ctx: Context<StakeCrank>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let window = ctx.accounts.closed_window.as_ref().map(|w| &***w);
    let sp = &mut ctx.accounts.stake_pool;
    let s = &mut ctx.accounts.stake_account;
    require!(
        s.warming > 0 && now >= s.warm_until,
        SprentError::StakeNotWarm
    );
    require!(
        !excluded_at(&ctx.accounts.platform, &s.owner, sp.current_window),
        SprentError::StakerExcluded
    );
    sp.update(now)?;
    sync_stake(s, sp, window, now)?;
    let w = s.warming;
    s.warming = 0;
    s.effective = s
        .effective
        .checked_add(w)
        .ok_or(SprentError::MathOverflow)?;
    sp.total_warming -= w as u128;
    sp.total_effective = sp
        .total_effective
        .checked_add(w as u128)
        .ok_or(SprentError::MathOverflow)?;
    emit!(StakeActivated {
        owner: s.owner,
        amount: w,
        effective: s.effective
    });
    Ok(())
}

pub(crate) fn request_unstake(ctx: Context<StakeOwnerOp>, amount: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let window = ctx.accounts.closed_window.as_ref().map(|w| &***w);
    let sp = &mut ctx.accounts.stake_pool;
    let s = &mut ctx.accounts.stake_account;
    require!(
        amount > 0 && amount <= s.effective,
        SprentError::InvalidWithdrawAmount
    );
    sp.update(now)?;
    sync_stake(s, sp, window, now)?;
    s.effective -= amount;
    sp.total_effective -= amount as u128;
    s.cooling = s
        .cooling
        .checked_add(amount)
        .ok_or(SprentError::MathOverflow)?;
    s.cool_until = now
        .checked_add(ctx.accounts.platform.stake_cooldown_s)
        .ok_or(SprentError::MathOverflow)?;
    sp.total_cooling = sp
        .total_cooling
        .checked_add(amount as u128)
        .ok_or(SprentError::MathOverflow)?;
    emit!(UnstakeRequested {
        owner: s.owner,
        amount,
        cool_until: s.cool_until
    });
    Ok(())
}

pub(crate) fn withdraw_unstaked(ctx: Context<StakeTokenOut>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let s = &mut ctx.accounts.stake_account;
    require!(
        s.cooling > 0 && now >= s.cool_until,
        SprentError::StakeCooling
    );
    let amount = s.cooling;
    s.cooling = 0;
    let sp = &mut ctx.accounts.stake_pool;
    sp.total_cooling -= amount as u128;
    let seeds: &[&[u8]] = &[STAKE_POOL_SEED, &[sp.bump]];
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.sprent_token_program.to_account_info(),
            token_interface::TransferChecked {
                from: ctx.accounts.stake_vault.to_account_info(),
                mint: ctx.accounts.sprent_mint.to_account_info(),
                to: ctx.accounts.owner_sprent.to_account_info(),
                authority: ctx.accounts.stake_pool.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        ctx.accounts.sprent_mint.decimals,
    )?;
    emit!(UnstakeWithdrawn {
        owner: ctx.accounts.owner.key(),
        amount
    });
    Ok(())
}

pub(crate) fn claim_reward(ctx: Context<ClaimReward>, nonce: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let window = ctx.accounts.closed_window.as_ref().map(|w| &***w);
    let sp = &mut ctx.accounts.stake_pool;
    let s = &mut ctx.accounts.stake_account;
    require!(nonce == s.claim_nonce, SprentError::ClaimNonceMismatch);
    sp.update(now)?;
    sync_stake(s, sp, window, now)?;
    let pay = s.reward_owed;
    require!(pay > 0, SprentError::NothingToClaim);
    s.reward_owed = 0;
    s.claim_nonce += 1;
    s.reward_claimed_total = s
        .reward_claimed_total
        .checked_add(pay as u128)
        .ok_or(SprentError::MathOverflow)?;
    sp.reward_claimed_total = sp
        .reward_claimed_total
        .checked_add(pay as u128)
        .ok_or(SprentError::MathOverflow)?;
    let seeds: &[&[u8]] = &[STAKE_POOL_SEED, &[sp.bump]];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.reward_vault.to_account_info(),
                mint: ctx.accounts.quote_mint.to_account_info(),
                to: ctx.accounts.owner_quote.to_account_info(),
                authority: ctx.accounts.stake_pool.to_account_info(),
            },
            &[seeds],
        ),
        pay,
        ctx.accounts.quote_mint.decimals,
    )?;
    emit!(RewardClaimed {
        owner: s.owner,
        amount: pay,
        nonce
    });
    Ok(())
}
