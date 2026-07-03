use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Donate<'info> {
    #[account(mut)]
    pub donor: Signer<'info>, // donor pays sol

    #[account(mut)]
    pub fundraiser: Account<'info, Fundraiser>, // fundraiser being donated to

    #[account(mut, address = fundraiser.vault)]
    pub vault: SystemAccount<'info>, // holds the sol

    #[account(
        init_if_needed,
        payer = donor,
        space = 8 + Donation::INIT_SPACE,
        seeds = [b"donation", fundraiser.key().as_ref(), donor.key().as_ref()],
        bump
    )] // we're creating the fundraiser account here
    pub donation: Account<'info, Donation>,

    pub system_program: Program<'info, System>,
}

// donate function
pub fn donate(ctx: Context<Donate>, amount: u64) -> Result<()> {
    let fundraiser = &mut ctx.accounts.fundraiser;

    // validations
    require!(
        fundraiser.status == FundraiserStatus::Active,
        ErrorCode::CampaignNotActive
    );

    let clock = Clock::get()?;
    require!(
        clock.unix_timestamp < fundraiser.deadline,
        ErrorCode::DeadlinePassed
    );

    require!(amount > 0, ErrorCode::InvalidDonationAmount);

    // transfer SOL: donor -> vault
    let cpi_context = CpiContext::new(
        ctx.accounts.system_program.key(),
        anchor_lang::system_program::Transfer {
            from: ctx.accounts.donor.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        },
    );
    anchor_lang::system_program::transfer(cpi_context, amount)?;

    // update fundraiser total
    fundraiser.raised_amount = fundraiser
        .raised_amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?;

    let donation = &mut ctx.accounts.donation;
    donation.fundraiser = fundraiser.key();
    donation.donor = ctx.accounts.donor.key();
    donation.amount = donation
        .amount
        .checked_add(amount)
        .ok_or(ErrorCode::MathOverflow)?; // checked_add in case this is their 2nd donation
    donation.timestamp = clock.unix_timestamp;
    donation.refunded = false;

    // flip status if goal reached or exceeded
    if fundraiser.raised_amount >= fundraiser.goal_amount {
        fundraiser.status = FundraiserStatus::Closed;
    }

    Ok(())
}
