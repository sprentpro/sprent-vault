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
#[instruction(intent_id: u64)]
pub struct OpenCreatorBuyback<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    #[account(seeds = [PLATFORM_SEED], bump = platform.bump, constraint = platform.buyback_executor == executor.key() @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(init, payer = executor, space = 8 + BuybackLedger::INIT_SPACE, seeds = [BUYBACK_SEED, intent_id.to_le_bytes().as_ref()], bump)]
    pub ledger: Box<Account<'info, BuybackLedger>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(intent_id: u64)]
pub struct TakeServiceBuyback<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    #[account(seeds = [PLATFORM_SEED], bump = platform.bump, constraint = platform.buyback_executor == executor.key() @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(init, payer = executor, space = 8 + BuybackLedger::INIT_SPACE, seeds = [BUYBACK_SEED, intent_id.to_le_bytes().as_ref()], bump)]
    pub ledger: Box<Account<'info, BuybackLedger>>,
    /// CHECK: service buyback budget authority PDA
    #[account(seeds = [TREASURY_BB_SEED], bump = platform.bb_bump)]
    pub treasury_bb: UncheckedAccount<'info>,
    #[account(mut, address = platform.bb_quote_ata @ SprentError::AccountSubstitution)]
    pub bb_quote_ata: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = quote_mint, token::authority = executor)]
    pub executor_quote: Box<Account<'info, TokenAccount>>,
    #[account(address = platform.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Box<Account<'info, Mint>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RecordBuyback<'info> {
    pub executor: Signer<'info>,
    #[account(mut, seeds = [PLATFORM_SEED], bump = platform.bump, constraint = platform.buyback_executor == executor.key() @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mut, seeds = [BUYBACK_SEED, ledger.intent_id.to_le_bytes().as_ref()], bump = ledger.bump)]
    pub ledger: Box<Account<'info, BuybackLedger>>,
    #[account(address = platform.sprent_mint @ SprentError::AccountSubstitution)]
    pub sprent_mint: Box<InterfaceAccount<'info, IMint>>,
    #[account(mut, token::mint = sprent_mint, token::authority = executor)]
    pub executor_sprent: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(mut, address = platform.burn_vault_ata @ SprentError::AccountSubstitution)]
    pub burn_vault_ata: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(address = platform.sprent_token_program @ SprentError::AccountSubstitution)]
    pub sprent_token_program: Interface<'info, TokenInterface>,

    #[account(mut, token::mint = platform.quote_mint, token::authority = executor)]
    pub executor_quote: Option<Box<Account<'info, TokenAccount>>>,
    #[account(mut, address = platform.bb_quote_ata @ SprentError::AccountSubstitution)]
    pub bb_quote_ata: Option<Box<Account<'info, TokenAccount>>>,
    #[account(address = platform.quote_mint @ SprentError::AccountSubstitution)]
    pub quote_mint: Option<Box<Account<'info, Mint>>>,
    pub token_program: Option<Program<'info, Token>>,
}

#[derive(Accounts)]
pub struct BurnBought<'info> {
    pub executor: Signer<'info>,
    #[account(mut, seeds = [PLATFORM_SEED], bump = platform.bump, constraint = platform.buyback_executor == executor.key() @ SprentError::Unauthorized)]
    pub platform: Box<Account<'info, Platform>>,
    #[account(mut, seeds = [BUYBACK_SEED, ledger.intent_id.to_le_bytes().as_ref()], bump = ledger.bump)]
    pub ledger: Box<Account<'info, BuybackLedger>>,
    #[account(mut, address = platform.sprent_mint @ SprentError::AccountSubstitution)]
    pub sprent_mint: Box<InterfaceAccount<'info, IMint>>,
    /// CHECK: burn vault authority PDA
    #[account(seeds = [BURN_VAULT_SEED], bump = platform.burn_bump)]
    pub burn_vault: UncheckedAccount<'info>,
    #[account(mut, address = platform.burn_vault_ata @ SprentError::AccountSubstitution)]
    pub burn_vault_ata: Box<InterfaceAccount<'info, ITokenAccount>>,
    #[account(address = platform.sprent_token_program @ SprentError::AccountSubstitution)]
    pub sprent_token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn open_creator_buyback(
    ctx: Context<OpenCreatorBuyback>,
    intent_id: u64,
    budget_lamports: u64,
) -> Result<()> {
    let l = &mut ctx.accounts.ledger;
    l.intent_id = intent_id;
    l.source = buyback_source::CREATOR_FEE;
    l.state = buyback_state::INTENT;
    l.bump = ctx.bumps.ledger;
    l.budget = budget_lamports;
    l.opened_slot = Clock::get()?.slot;
    emit!(BuybackOpened {
        intent_id,
        source: l.source,
        budget: l.budget
    });
    Ok(())
}

pub(crate) fn take_service_buyback(
    ctx: Context<TakeServiceBuyback>,
    intent_id: u64,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, SprentError::ZeroAmount);
    let p = &ctx.accounts.platform;
    let seeds: &[&[u8]] = &[TREASURY_BB_SEED, &[p.bb_bump]];
    token::transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.bb_quote_ata.to_account_info(),
                mint: ctx.accounts.quote_mint.to_account_info(),
                to: ctx.accounts.executor_quote.to_account_info(),
                authority: ctx.accounts.treasury_bb.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        ctx.accounts.quote_mint.decimals,
    )?;
    let l = &mut ctx.accounts.ledger;
    l.intent_id = intent_id;
    l.source = buyback_source::SERVICE_FEE;
    l.state = buyback_state::INTENT;
    l.bump = ctx.bumps.ledger;
    l.budget = amount;
    l.opened_slot = Clock::get()?.slot;
    emit!(BuybackOpened {
        intent_id,
        source: l.source,
        budget: amount
    });
    Ok(())
}

pub(crate) fn record_buyback(
    ctx: Context<RecordBuyback>,
    buy_sig: [u8; 64],
    spent: u64,
    bought: u64,
) -> Result<()> {
    require!(bought > 0, SprentError::ZeroAmount);
    let l = &ctx.accounts.ledger;
    require!(
        l.state == buyback_state::INTENT,
        SprentError::BuybackStateMismatch
    );
    let mut refund = 0u64;
    if l.source == buyback_source::SERVICE_FEE {
        require!(spent <= l.budget, SprentError::BuybackOverBudget);
        refund = l.budget - spent;
        if refund > 0 {
            let quote_mint = ctx
                .accounts
                .quote_mint
                .as_ref()
                .ok_or(SprentError::AccountSubstitution)?;
            let from = ctx
                .accounts
                .executor_quote
                .as_ref()
                .ok_or(SprentError::AccountSubstitution)?;
            let to = ctx
                .accounts
                .bb_quote_ata
                .as_ref()
                .ok_or(SprentError::AccountSubstitution)?;
            let tp = ctx
                .accounts
                .token_program
                .as_ref()
                .ok_or(SprentError::AccountSubstitution)?;
            token::transfer_checked(
                CpiContext::new(
                    tp.to_account_info(),
                    TransferChecked {
                        from: from.to_account_info(),
                        mint: quote_mint.to_account_info(),
                        to: to.to_account_info(),
                        authority: ctx.accounts.executor.to_account_info(),
                    },
                ),
                refund,
                quote_mint.decimals,
            )?;
        }
    }
    let before = ctx.accounts.burn_vault_ata.amount;
    token_interface::transfer_checked(
        CpiContext::new(
            ctx.accounts.sprent_token_program.to_account_info(),
            token_interface::TransferChecked {
                from: ctx.accounts.executor_sprent.to_account_info(),
                mint: ctx.accounts.sprent_mint.to_account_info(),
                to: ctx.accounts.burn_vault_ata.to_account_info(),
                authority: ctx.accounts.executor.to_account_info(),
            },
        ),
        bought,
        ctx.accounts.sprent_mint.decimals,
    )?;
    ctx.accounts.burn_vault_ata.reload()?;
    let received = ctx
        .accounts
        .burn_vault_ata
        .amount
        .checked_sub(before)
        .ok_or(SprentError::MathOverflow)?;
    let l = &mut ctx.accounts.ledger;
    l.spent = spent;
    l.refunded = refund;
    l.bought = received;
    l.buy_sig = buy_sig;
    l.state = buyback_state::BOUGHT;
    l.bought_slot = Clock::get()?.slot;
    let p = &mut ctx.accounts.platform;
    if l.source == buyback_source::SERVICE_FEE {
        p.bought_total_service = p
            .bought_total_service
            .checked_add(received as u128)
            .ok_or(SprentError::MathOverflow)?;
    } else {
        p.bought_total_creator = p
            .bought_total_creator
            .checked_add(received as u128)
            .ok_or(SprentError::MathOverflow)?;
    }
    emit!(BuybackRecorded {
        intent_id: l.intent_id,
        source: l.source,
        spent,
        refunded: refund,
        bought: received,
        buy_sig
    });
    Ok(())
}

pub(crate) fn burn_bought(ctx: Context<BurnBought>, amount: u64) -> Result<()> {
    let l = &ctx.accounts.ledger;
    require!(
        l.state == buyback_state::BOUGHT,
        SprentError::BuybackStateMismatch
    );
    require!(
        amount > 0 && amount <= l.pending_burn(),
        SprentError::InvalidWithdrawAmount
    );
    let p = &ctx.accounts.platform;
    let seeds: &[&[u8]] = &[BURN_VAULT_SEED, &[p.burn_bump]];
    let supply_before = ctx.accounts.sprent_mint.supply;
    token_interface::burn(
        CpiContext::new_with_signer(
            ctx.accounts.sprent_token_program.to_account_info(),
            token_interface::Burn {
                mint: ctx.accounts.sprent_mint.to_account_info(),
                from: ctx.accounts.burn_vault_ata.to_account_info(),
                authority: ctx.accounts.burn_vault.to_account_info(),
            },
            &[seeds],
        ),
        amount,
    )?;
    ctx.accounts.sprent_mint.reload()?;
    require!(
        supply_before.checked_sub(ctx.accounts.sprent_mint.supply) == Some(amount),
        SprentError::BurnMismatch
    );
    let l = &mut ctx.accounts.ledger;
    l.burned += amount;
    l.last_burn_slot = Clock::get()?.slot;
    if l.burned == l.bought {
        l.state = buyback_state::BURNED;
    }
    let p = &mut ctx.accounts.platform;
    if l.source == buyback_source::SERVICE_FEE {
        p.burned_total_service = p
            .burned_total_service
            .checked_add(amount as u128)
            .ok_or(SprentError::MathOverflow)?;
    } else {
        p.burned_total_creator = p
            .burned_total_creator
            .checked_add(amount as u128)
            .ok_or(SprentError::MathOverflow)?;
    }
    emit!(BuybackBurned {
        intent_id: l.intent_id,
        source: l.source,
        amount,
        burned: l.burned,
        pending_burn: l.pending_burn()
    });
    Ok(())
}
