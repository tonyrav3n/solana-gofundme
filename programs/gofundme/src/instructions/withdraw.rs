use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        mut,
        has_one = creator @ ErrorCode::NotCreator
    )]
    pub fundraiser: Account<'info, Fundraiser>,

    #[account(
        mut,
        seeds = [b"vault", fundraiser.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
    let vault = &ctx.accounts.vault;
    let rent_exemption = Rent::get()?.minimum_balance(0);

    // Calculate how much we can safely withdraw while keeping the account rent-exempt
    let available_amount = vault.lamports().saturating_sub(rent_exemption);

    require!(available_amount > 0, ErrorCode::InsufficientVaultBalance);

    let fundraiser_key = ctx.accounts.fundraiser.key();
    let vault_bump = ctx.bumps.vault;

    let signer_seeds: &[&[&[u8]]] = &[&[b"vault", fundraiser_key.as_ref(), &[vault_bump]]];

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        anchor_lang::system_program::Transfer {
            from: vault.to_account_info(),
            to: ctx.accounts.creator.to_account_info(),
        },
        signer_seeds,
    );

    anchor_lang::system_program::transfer(cpi_context, available_amount)?;

    Ok(())
}
