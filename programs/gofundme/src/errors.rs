use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Title must be between 1 and 50 characters")]
    TitleTooLong,

    #[msg("Description must be between 1 and 1000 characters")]
    DescriptionTooLong,

    #[msg("Goal amount must be at least 1 SOL")]
    GoalTooSmall,

    #[msg("Deadline must be in the future")]
    DeadlineInPast,

    #[msg("Deadline must be between 7 and 30 days from now")]
    InvalidDeadline,

    #[msg("Campaign is no longer active")]
    CampaignNotActive,

    #[msg("The deadline for this campaign has already passed")]
    DeadlinePassed,

    #[msg("Donation amount must be greater than 0")]
    InvalidDonationAmount,

    #[msg("Math overflow occurred")]
    MathOverflow,

    #[msg("Only the creator can withdraw funds")]
    NotCreator,

    #[msg("Insufficient balance in the vault")]
    InsufficientVaultBalance,

    #[msg("Campaign is not yet abandoned (must wait 7 days after deadline)")]
    NotAbandoned,

    #[msg("Donation already refunded")]
    AlreadyRefunded,

    #[msg("Donor account mismatch")]
    InvalidDonor,
}
