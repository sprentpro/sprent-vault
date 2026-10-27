use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::token_interface::{Mint as IMint, TokenAccount as ITokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PlatformArgs {
    pub buyback_executor: Pubkey,
    pub holder_bps: u16,
    pub buyback_bps: u16,
    pub stake_warmup_s: i64,
    pub stake_cooldown_s: i64,
    pub excluded: Vec<Pubkey>,
}

#[derive(Accounts)]
pub struct InitPlatform<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + Platform::INIT_SPACE, seeds = [PLATFORM_SEED], bump)]
    pub platform: Box<Account<'info, Platform>>,
    pub quote_mint: Box<Account<'info, Mint>>,
    /// CHECK: S authority PDA (no data)
    #[account(seeds = [TREASURY_S_SEED], bump)]
    pub treasury_s: UncheckedAccount<'info>,
    /// CHECK: T authority PDA (no data)
    #[account(seeds = [TREASURY_T_SEED], bump)]
    pub treasury_t: UncheckedAccount<'info>,
    /// CHECK: service buyback budget authority PDA (no data)
    #[account(seeds = [TREASURY_BB_SEED], bump)]
    pub treasury_bb: UncheckedAccount<'info>,
    #[account(init, payer = admin, associated_token::mint = quote_mint, associated_token::authority = treasury_s, associated_token::token_program = token_program)]
    pub s_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(init, payer = admin, associated_token::mint = quote_mint, associated_token::authority = treasury_t, associated_token::token_program = token_program)]
    pub t_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(init, payer = admin, associated_token::mint = quote_mint, associated_token::authority = treasury_bb, associated_token::token_program = token_program)]
    pub bb_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ SprentError::Unauthorized)]
    pub program: Program<'info, crate::program::SprentVault>,
    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()) @ SprentError::Unauthorized)]
    pub program_data: Box<Account<'info, ProgramData>>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitTokenSide<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [PLATFORM_SEED], bump = platform.bump, has_one = admin @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mint::token_program = sprent_token_program)]
    pub sprent_mint: Box<InterfaceAccount<'info, IMint>>,
    #[account(address = platform.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    #[account(init, payer = admin, space = 8 + StakePool::INIT_SPACE, seeds = [STAKE_POOL_SEED], bump)]
    pub stake_pool: Box<Account<'info, StakePool>>,
    #[account(init, payer = admin, associated_token::mint = sprent_mint, associated_token::authority = stake_pool, associated_token::token_program = sprent_token_program)]
    pub stake_vault: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(init, payer = admin, associated_token::mint = quote_mint, associated_token::authority = stake_pool, associated_token::token_program = token_program)]
    pub reward_vault: Box<Account<'info, TokenAccount>>,
    /// CHECK: burn vault authority PDA (no data)
    #[account(seeds = [BURN_VAULT_SEED], bump)]
    pub burn_vault: UncheckedAccount<'info>,
    #[account(init, payer = admin, associated_token::mint = sprent_mint, associated_token::authority = burn_vault, associated_token::token_program = sprent_token_program)]
    pub burn_vault_ata: Box<InterfaceAccount<'info, ITokenAccount>>,
    pub sprent_token_program: Interface<'info, TokenInterface>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn init_platform(ctx: Context<InitPlatform>, args: PlatformArgs) -> Result<()> {
    require!(
        args.holder_bps as u64 + args.buyback_bps as u64 <= BPS_DENOMINATOR
            && args.excluded.len() <= MAX_EXCLUDED
            && args.stake_warmup_s >= 0
            && args.stake_cooldown_s >= 0,
        SprentError::InvalidParams
    );
    let p = &mut ctx.accounts.platform;
    p.admin = ctx.accounts.admin.key();
    p.buyback_executor = args.buyback_executor;
    p.quote_mint = ctx.accounts.quote_mint.key();
    p.s_quote_ata = ctx.accounts.s_quote_ata.key();
    p.t_quote_ata = ctx.accounts.t_quote_ata.key();
    p.bb_quote_ata = ctx.accounts.bb_quote_ata.key();
    p.holder_bps = args.holder_bps;
    p.buyback_bps = args.buyback_bps;
    p.stake_warmup_s = args.stake_warmup_s;
    p.stake_cooldown_s = args.stake_cooldown_s;
    for (i, k) in args.excluded.iter().enumerate() {
        p.excluded[i] = *k;
    }
    p.excluded_len = args.excluded.len() as u8;

    p.bump = ctx.bumps.platform;
    p.s_bump = ctx.bumps.treasury_s;
    p.t_bump = ctx.bumps.treasury_t;
    p.bb_bump = ctx.bumps.treasury_bb;
    emit!(PlatformInitialized {
        admin: p.admin,
        buyback_executor: p.buyback_executor,
        s_quote_ata: p.s_quote_ata
    });
    Ok(())
}

pub(crate) fn init_token_side(ctx: Context<InitTokenSide>) -> Result<()> {
    let p = &mut ctx.accounts.platform;
    require!(
        p.sprent_mint == Pubkey::default(),
        SprentError::InvalidParams
    );
    p.sprent_mint = ctx.accounts.sprent_mint.key();
    p.sprent_token_program = ctx.accounts.sprent_token_program.key();
    p.burn_vault_ata = ctx.accounts.burn_vault_ata.key();
    p.burn_bump = ctx.bumps.burn_vault;
    let now = Clock::get()?.unix_timestamp;
    let sp = &mut ctx.accounts.stake_pool;
    sp.mint = p.sprent_mint;
    sp.token_program = p.sprent_token_program;
    sp.stake_vault = ctx.accounts.stake_vault.key();
    sp.reward_vault = ctx.accounts.reward_vault.key();
    sp.bump = ctx.bumps.stake_pool;
    sp.window_start = now;
    sp.last_update = now;
    emit!(TokenSideInitialized {
        mint: p.sprent_mint,
        token_program: p.sprent_token_program,
        stake_pool: sp.key()
    });
    Ok(())
}
