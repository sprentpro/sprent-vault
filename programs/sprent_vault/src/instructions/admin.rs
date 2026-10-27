use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::SprentError;
use crate::events::*;
use crate::helpers::*;
use crate::state::*;

#[derive(Accounts)]
pub struct AdminOnly<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump, has_one = admin @ SprentError::Unauthorized)]
    pub config: Box<Account<'info, VaultConfig>>,
}

#[derive(Accounts)]
pub struct SetState<'info> {
    pub authority: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, config.desk_id.to_le_bytes().as_ref()], bump = config.config_bump)]
    pub config: Box<Account<'info, VaultConfig>>,
}

pub(crate) fn set_params(ctx: Context<AdminOnly>, params: RiskParams) -> Result<()> {
    validate_params(&params, ctx.accounts.config.deposit_cap_absolute_quote)?;
    ctx.accounts.config.params = params;
    emit!(ParamsUpdated { params });
    Ok(())
}

pub(crate) fn set_roles(ctx: Context<AdminOnly>, operator: Pubkey, guardian: Pubkey) -> Result<()> {
    require!(
        operator != Pubkey::default() && guardian != Pubkey::default(),
        SprentError::InvalidParams
    );
    let c = &mut ctx.accounts.config;
    c.operator = operator;
    c.guardian = guardian;
    emit!(RolesUpdated { operator, guardian });
    Ok(())
}

pub(crate) fn set_state(ctx: Context<SetState>, new_state: u8) -> Result<()> {
    require!(
        new_state <= vault_state::RECONCILE_FROZEN,
        SprentError::InvalidParams
    );
    let c = &mut ctx.accounts.config;
    let signer = ctx.accounts.authority.key();
    let ok = signer == c.admin || (signer == c.guardian && new_state == vault_state::HALTED);
    require!(ok, SprentError::Unauthorized);
    c.state = new_state;
    emit!(StateChanged {
        authority: signer,
        state: new_state
    });
    Ok(())
}
