use crate::constants::*;
use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
#[instruction(fundraiser_index: u8)]
pub struct InitializeFundraiser<'info> {
    #[account(mut)]
    pub creator: Signer<'info>, // fundraiser creator signs the trx

    #[account(
        init,
        payer = creator,
        space = 8 + Fundraiser::INIT_SPACE,
        seeds = [b"fundraiser", creator.key().as_ref(), fundraiser_index.to_le_bytes().as_ref()],
        bump
    )] // we're creating the fundraiser account here
    pub fundraiser: Account<'info, Fundraiser>,

    #[account(
        seeds = [b"vault", fundraiser.key().as_ref()],
        bump
    )]
    pub vault: SystemAccount<'info>, // holds the sol

    pub system_program: Program<'info, System>,
}

// initialises fundraiser
pub fn initialize_fundraiser(
    ctx: Context<InitializeFundraiser>,
    _fundraiser_index: u8,
    title: String,
    description: String,
    goal_amount: u64,
    deadline_timestamp: i64,
) -> Result<()> {
    let fundraiser = &mut ctx.accounts.fundraiser;

    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp;
    let duration = deadline_timestamp - current_timestamp;

    // validations
    require!(
        !title.is_empty() && title.len() <= MAX_TITLE_LENGTH,
        ErrorCode::TitleTooLong
    );

    require!(
        !description.is_empty() && description.len() <= MAX_DESCRIPTION_LENGTH,
        ErrorCode::DescriptionTooLong
    );

    require!(goal_amount >= MIN_GOAL_AMOUNT, ErrorCode::GoalTooSmall);

    require!(
        deadline_timestamp > current_timestamp,
        ErrorCode::DeadlineInPast
    );

    let min_duration = MIN_DEADLINE_DAYS * 24 * 60 * 60;
    let max_duration = MAX_DEADLINE_DAYS * 24 * 60 * 60;

    require!(
        duration >= min_duration && duration <= max_duration,
        ErrorCode::InvalidDeadline
    );

    fundraiser.creator = ctx.accounts.creator.key();
    fundraiser.title = title;
    fundraiser.description = description;
    fundraiser.goal_amount = goal_amount;
    fundraiser.deadline = deadline_timestamp;
    fundraiser.raised_amount = 0;
    fundraiser.status = FundraiserStatus::Active;
    fundraiser.vault = ctx.accounts.vault.key();
    fundraiser.bump = ctx.bumps.fundraiser;
    fundraiser.created_at = Clock::get()?.unix_timestamp;

    Ok(())
}
