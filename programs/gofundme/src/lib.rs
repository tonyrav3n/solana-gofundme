use anchor_lang::prelude::*;
pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("DqFsjUeJduCUGx9PiGAdTgReRyU1oRfEX5jMctXcHCC8");

#[program]
pub mod gofundme {
    use super::*;

    pub fn initialize_fundraiser(
        ctx: Context<InitializeFundraiser>,
        fundraiser_index: u8,
        title: String,
        description: String,
        goal_amount: u64,
        deadline: i64,
    ) -> Result<()> {
        instructions::initialize_fundraiser::initialize_fundraiser(
            ctx,
            fundraiser_index,
            title,
            description,
            goal_amount,
            deadline,
        )
    }

    pub fn donate(ctx: Context<Donate>, amount: u64) -> Result<()> {
        instructions::donate::donate(ctx, amount)
    }

    pub fn withdraw(ctx: Context<Withdraw>) -> Result<()> {
        instructions::withdraw::withdraw(ctx)
    }

    pub fn process_refunds(
        ctx: Context<ProcessRefunds>,
        donor: Pubkey,
    ) -> Result<()> {
        instructions::process_refunds::process_refunds(ctx, donor)
    }
}
