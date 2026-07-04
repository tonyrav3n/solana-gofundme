use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(donor: Pubkey)]
pub struct ProcessRefunds<'info> {
    #[account(mut)]
    pub fundraiser: Account<'info, Fundraiser>,

    #[account(
        mut,
        seeds = [b"vault", fundraiser.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"donation", fundraiser.key().as_ref(), donor.as_ref()],
        bump,
        has_one = fundraiser,
    )]
    pub donation: Account<'info, Donation>,

    #[account(mut)]
    pub donor_account: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn process_refunds(ctx: Context<ProcessRefunds>, donor: Pubkey) -> Result<()> {
    let fundraiser = &ctx.accounts.fundraiser;
    let donation = &mut ctx.accounts.donation;
    let vault = &ctx.accounts.vault;
    let clock = Clock::get()?;

    // 7 days after the deadline
    let abandonment_period = 7 * 24 * 60 * 60;

    // Check time condition: must be at least 7 days after deadline
    require!(
        clock.unix_timestamp > fundraiser.deadline + abandonment_period,
        ErrorCode::NotAbandoned
    );

    // Check if already refunded
    require!(!donation.refunded, ErrorCode::AlreadyRefunded);

    // Check vault balance (rent-exempt minimum)
    let rent_exemption = Rent::get()?.minimum_balance(0);
    let available_amount = vault.lamports().saturating_sub(rent_exemption);

    require!(
        available_amount >= donation.amount,
        ErrorCode::InsufficientVaultBalance
    );

    // Verify donor account matches the donation record
    require_eq!(donation.donor, donor, ErrorCode::InvalidDonor);

    // Transfer from vault to donor
    let fundraiser_key = fundraiser.key();
    let vault_bump = ctx.bumps.vault;

    let signer_seeds: &[&[&[u8]]] = &[&[b"vault", fundraiser_key.as_ref(), &[vault_bump]]];

    let cpi_context = CpiContext::new_with_signer(
        ctx.accounts.system_program.key(),
        anchor_lang::system_program::Transfer {
            from: vault.to_account_info(),
            to: ctx.accounts.donor_account.to_account_info(),
        },
        signer_seeds,
    );

    anchor_lang::system_program::transfer(cpi_context, donation.amount)?;

    // Mark as refunded
    donation.refunded = true;

    Ok(())
}
