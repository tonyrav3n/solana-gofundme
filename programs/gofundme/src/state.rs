use anchor_lang::prelude::*; // fundraiser details

#[account]
#[derive(InitSpace)]
pub struct Fundraiser {
    pub creator: Pubkey,
    #[max_len(100)]
    pub title: String,
    #[max_len(500)]
    pub description: String,
    pub goal_amount: u64,
    pub deadline: i64,
    pub raised_amount: u64,
    pub status: FundraiserStatus,
    pub vault: Pubkey,
    pub bump: u8,
    pub created_at: i64,
}

// donation details
#[account]
#[derive(InitSpace)]

pub struct Donation {
    pub fundraiser: Pubkey,
    pub donor: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
    pub refunded: bool,
}

#[derive(InitSpace, AnchorSerialize, AnchorDeserialize, PartialEq, Eq, Clone)]
pub enum FundraiserStatus {
    Active,
    Closed,
}
