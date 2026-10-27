use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::token_2022::spl_token_2022;
use anchor_spl::token_interface::{Mint as IMint, TokenAccount as ITokenAccount, TokenInterface};

use crate::accounting::PPS_SCALE;
use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::manifest::*;
use crate::oracle;
use crate::state::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeArgs {
    pub desk_id: u16,

    pub oracle_mode: u8,

    pub allow_permanent_delegate: bool,

    pub deposit_cap_absolute_quote: u64,
    pub guardian: Pubkey,
    pub operator: Pubkey,
    pub oracle_feed_id: [u8; 32],
    pub network: u8,
    pub params: RiskParams,
    pub vault_sol_lamports: u64,
}

#[derive(Accounts)]
#[instruction(args: InitializeArgs)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + VaultConfig::INIT_SPACE, seeds = [CONFIG_SEED, args.desk_id.to_le_bytes().as_ref()], bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    /// CHECK: system-owned PDA (no data) that becomes the Manifest trader; funded here.
    #[account(mut, seeds = [VAULT_AUTH_SEED, config.key().as_ref()], bump)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: owner, discriminator, mints, decimals and vaults are verified in the handler.
    pub market: UncheckedAccount<'info>,
    #[account(mint::token_program = base_token_program)]
    pub base_mint: Box<InterfaceAccount<'info, IMint>>,
    pub quote_mint: Box<Account<'info, Mint>>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = base_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = base_token_program
    )]
    pub vault_base_ata: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(
        init,
        payer = admin,
        associated_token::mint = quote_mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program
    )]
    pub vault_quote_ata: Box<Account<'info, TokenAccount>>,
    /// CHECK: must equal the Manifest vault PDA and the market header.
    pub manifest_base_vault: UncheckedAccount<'info>,
    /// CHECK: must equal the Manifest vault PDA and the market header.
    pub manifest_quote_vault: UncheckedAccount<'info>,
    /// CHECK: Pyth PriceUpdateV2 (owner, layout, feed verified) or the system program id (peg desk).
    pub oracle: UncheckedAccount<'info>,
    /// CHECK: fixed Manifest program id.
    #[account(address = MANIFEST_PROGRAM_ID @ SprentError::ManifestProgramMismatch)]
    pub manifest_program: UncheckedAccount<'info>,
    #[account(constraint = program.programdata_address()? == Some(program_data.key()) @ SprentError::Unauthorized)]
    pub program: Program<'info, crate::program::SprentVault>,
    #[account(constraint = program_data.upgrade_authority_address == Some(admin.key()) @ SprentError::Unauthorized)]
    pub program_data: Box<Account<'info, ProgramData>>,
    pub base_token_program: Interface<'info, TokenInterface>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimSeat<'info> {
    pub signer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
    /// CHECK: pinned vault authority PDA.
    #[account(mut, address = config.vault_authority @ SprentError::AccountSubstitution)]
    pub vault_authority: UncheckedAccount<'info>,
    /// CHECK: pinned market.
    #[account(mut, address = config.manifest_market @ SprentError::AccountSubstitution)]
    pub market: UncheckedAccount<'info>,
    /// CHECK: fixed Manifest program id.
    #[account(address = MANIFEST_PROGRAM_ID @ SprentError::ManifestProgramMismatch)]
    pub manifest_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitTreasury<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump, has_one = admin @ SprentError::Unauthorized)]
    pub config: Box<Account<'info, VaultConfig>>,

    #[account(seeds = [PLATFORM_SEED], bump = platform.bump)]
    pub platform: Box<Account<'info, Platform>>,
}

pub(crate) fn initialize(ctx: Context<Initialize>, args: InitializeArgs) -> Result<()> {
    require!(
        args.network <= network::LOCALNET,
        SprentError::InvalidParams
    );
    require!(
        args.vault_sol_lamports >= MIN_VAULT_SOL_BUFFER,
        SprentError::VaultSolBufferTooSmall
    );

    let market_key = ctx.accounts.market.key();
    let hdr = read_market_header(&ctx.accounts.market.to_account_info())?;
    require!(
        args.deposit_cap_absolute_quote > 0,
        SprentError::InvalidParams
    );
    validate_params(&args.params, args.deposit_cap_absolute_quote)?;
    let base_mint = ctx.accounts.base_mint.key();
    let quote_mint = ctx.accounts.quote_mint.key();
    require!(base_mint != quote_mint, SprentError::MarketMintMismatch);
    check_base_mint_policy(
        &ctx.accounts.base_mint.to_account_info(),
        args.allow_permanent_delegate,
    )?;
    require_keys_eq!(hdr.base_mint, base_mint, SprentError::MarketMintMismatch);
    require_keys_eq!(hdr.quote_mint, quote_mint, SprentError::MarketMintMismatch);
    require!(
        hdr.base_decimals == ctx.accounts.base_mint.decimals
            && hdr.quote_decimals == ctx.accounts.quote_mint.decimals,
        SprentError::MarketMintMismatch
    );
    let base_vault = market_vault_address(&market_key, &base_mint);
    let quote_vault = market_vault_address(&market_key, &quote_mint);
    require!(
        base_vault == hdr.base_vault
            && quote_vault == hdr.quote_vault
            && ctx.accounts.manifest_base_vault.key() == base_vault
            && ctx.accounts.manifest_quote_vault.key() == quote_vault,
        SprentError::MarketVaultMismatch
    );

    oracle::read_oracle(
        args.oracle_mode,
        &ctx.accounts.oracle.to_account_info(),
        &args.oracle_feed_id,
        Clock::get()?.unix_timestamp,
        args.params.oracle_max_age_s,
        args.params.oracle_max_conf_bps,
        hdr.base_decimals,
        hdr.quote_decimals,
    )?;

    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.admin.to_account_info(),
                to: ctx.accounts.vault_authority.to_account_info(),
            },
        ),
        args.vault_sol_lamports,
    )?;

    let c = &mut ctx.accounts.config;
    c.desk_id = args.desk_id;
    c.oracle_mode = args.oracle_mode;
    c.admin = ctx.accounts.admin.key();
    c.guardian = args.guardian;
    c.operator = args.operator;
    c.manifest_program = MANIFEST_PROGRAM_ID;
    c.manifest_market = market_key;
    c.base_mint = base_mint;
    c.quote_mint = quote_mint;
    c.base_token_program = ctx.accounts.base_token_program.key();
    c.quote_token_program = ctx.accounts.token_program.key();
    c.vault_authority = ctx.accounts.vault_authority.key();
    c.vault_base_ata = ctx.accounts.vault_base_ata.key();
    c.vault_quote_ata = ctx.accounts.vault_quote_ata.key();
    c.manifest_base_vault = base_vault;
    c.manifest_quote_vault = quote_vault;
    c.oracle = ctx.accounts.oracle.key();
    c.oracle_feed_id = args.oracle_feed_id;
    c.base_mint_decimals = hdr.base_decimals;
    c.quote_mint_decimals = hdr.quote_decimals;
    c.config_bump = ctx.bumps.config;
    c.vault_authority_bump = ctx.bumps.vault_authority;
    c.network = args.network;
    c.state = vault_state::RUNNING;
    c.params = args.params;
    c.last_epoch_pps = PPS_SCALE;
    c.day_ref_pps = PPS_SCALE;
    c.peak_pps = PPS_SCALE;
    c.deposit_cap_absolute_quote = args.deposit_cap_absolute_quote;
    c.last_close_ts = Clock::get()?.unix_timestamp;

    emit!(VaultInitialized {
        desk_id: c.desk_id,
        admin: c.admin,
        operator: c.operator,
        guardian: c.guardian,
        market: market_key,
        vault_authority: c.vault_authority,
    });
    Ok(())
}

pub(crate) fn claim_seat(ctx: Context<ClaimSeat>) -> Result<()> {
    let c = &ctx.accounts.config;
    let signer = ctx.accounts.signer.key();
    require!(
        signer == c.admin || signer == c.operator,
        SprentError::Unauthorized
    );
    let ix = claim_seat_ix(c.vault_authority, c.manifest_market, system_program::ID);
    invoke_as_vault(
        &ix,
        &[
            ctx.accounts.vault_authority.to_account_info(),
            ctx.accounts.market.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.manifest_program.to_account_info(),
        ],
        &ctx.accounts.config.key(),
        c.vault_authority_bump,
    )
}

pub(crate) fn init_treasury(ctx: Context<InitTreasury>) -> Result<()> {
    let p = &ctx.accounts.platform;
    let c = &mut ctx.accounts.config;
    require!(
        c.treasury_s_quote_ata == Pubkey::default(),
        SprentError::InvalidParams
    );
    require_keys_eq!(p.quote_mint, c.quote_mint, SprentError::AccountSubstitution);
    c.treasury_s_quote_ata = p.s_quote_ata;
    emit!(TreasuryInitialized {
        treasury_s: p.key(),
        quote_ata: c.treasury_s_quote_ata
    });
    Ok(())
}

fn check_base_mint_policy(mint: &AccountInfo, allow_permanent_delegate: bool) -> Result<()> {
    use spl_token_2022::extension::{
        permanent_delegate::PermanentDelegate, transfer_fee::TransferFeeConfig,
        transfer_hook::TransferHook, BaseStateWithExtensions, StateWithExtensions,
    };
    if *mint.owner != spl_token_2022::ID {
        return Ok(());
    }
    let data = mint.try_borrow_data()?;
    let st = StateWithExtensions::<spl_token_2022::state::Mint>::unpack(&data)
        .map_err(|_| error!(SprentError::UnsupportedTokenProgram))?;
    if let Ok(fee) = st.get_extension::<TransferFeeConfig>() {
        let older: u16 = fee.older_transfer_fee.transfer_fee_basis_points.into();
        let newer: u16 = fee.newer_transfer_fee.transfer_fee_basis_points.into();
        require!(
            older == 0 && newer == 0,
            SprentError::UnsupportedMintExtension
        );
    }
    if let Ok(hook) = st.get_extension::<TransferHook>() {
        require!(
            Option::<Pubkey>::from(hook.program_id).is_none(),
            SprentError::UnsupportedMintExtension
        );
    }
    if let Ok(pd) = st.get_extension::<PermanentDelegate>() {
        require!(
            Option::<Pubkey>::from(pd.delegate).is_none() || allow_permanent_delegate,
            SprentError::UnsupportedMintExtension
        );
    }
    Ok(())
}
