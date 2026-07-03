use anchor_lang::prelude::Clock;
use anchor_litesvm::{AnchorLiteSVM, Signer, TestHelpers};

use crate::gofundme::client::accounts::{Donate, InitializeFundraiser};
use crate::gofundme::client::args::{
    Donate as DonateArgs, InitializeFundraiser as InitializeFundraiserArgs,
};
use crate::gofundme::types::FundraiserStatus;

// Helper to create a fundraiser
fn setup_fundraiser(
    ctx: &mut anchor_litesvm::AnchorContext,
    creator: &anchor_litesvm::Keypair,
    fundraiser_index: u8,
    goal_amount: u64,
    deadline_offset_days: i64,
) -> (anchor_litesvm::Pubkey, anchor_litesvm::Pubkey) {
    let clock = ctx.svm.get_sysvar::<Clock>();
    let deadline = clock.unix_timestamp + (deadline_offset_days * 24 * 60 * 60);

    let (fundraiser_pda, _) = ctx.svm.get_pda_with_bump(
        &[
            b"fundraiser",
            creator.pubkey().as_ref(),
            fundraiser_index.to_le_bytes().as_ref(),
        ],
        &crate::gofundme::ID,
    );

    let (vault_pda, _) = ctx
        .svm
        .get_pda_with_bump(&[b"vault", fundraiser_pda.as_ref()], &crate::gofundme::ID);

    let ix = ctx
        .program()
        .accounts(InitializeFundraiser {
            creator: creator.pubkey(),
            fundraiser: fundraiser_pda,
            vault: vault_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(InitializeFundraiserArgs {
            fundraiser_index,
            title: "Test Campaign".to_string(),
            description: "Test description".to_string(),
            goal_amount,
            deadline,
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[creator])
        .unwrap()
        .assert_success();

    (fundraiser_pda, vault_pda)
}

fn donate_helper(
    ctx: &mut anchor_litesvm::AnchorContext,
    donor: &anchor_litesvm::Keypair,
    fundraiser: anchor_litesvm::Pubkey,
    vault: anchor_litesvm::Pubkey,
    amount: u64,
) -> anchor_litesvm::TransactionResult {
    let (donation_pda, _) = ctx.svm.get_pda_with_bump(
        &[b"donation", fundraiser.as_ref(), donor.pubkey().as_ref()],
        &crate::gofundme::ID,
    );

    let ix = ctx
        .program()
        .accounts(Donate {
            donor: donor.pubkey(),
            fundraiser,
            vault,
            donation: donation_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(DonateArgs { amount })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[donor]).unwrap()
}

// ============================================================================
// Happy path
// ============================================================================

#[test]
fn test_donate_success() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1_000_000_000;
    ctx.svm.set_sysvar(&clock);

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap(); // 10 SOL
    let donor = ctx.svm.create_funded_account(20_000_000_000).unwrap(); // 20 SOL

    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    let donation_amount = 2_000_000_000;
    let initial_donor_balance = ctx.svm.get_balance(&donor.pubkey()).unwrap();
    let initial_vault_balance = ctx.svm.get_balance(&vault).unwrap_or(0);

    let result = donate_helper(&mut ctx, &donor, fundraiser, vault, donation_amount);
    result.assert_success();

    // Verify SOL balances moved correctly
    let final_donor_balance = ctx.svm.get_balance(&donor.pubkey()).unwrap();
    let final_vault_balance = ctx.svm.get_balance(&vault).unwrap();

    // Donor balance should be reduced by donation_amount + transaction fee (so slightly less)
    assert!(final_donor_balance <= initial_donor_balance - donation_amount);
    // Vault balance should increase exactly by donation_amount
    assert_eq!(final_vault_balance, initial_vault_balance + donation_amount);

    // Verify Fundraiser state
    let account: crate::gofundme::accounts::Fundraiser = ctx.get_account(&fundraiser).unwrap();
    assert_eq!(account.raised_amount, donation_amount);
    assert!(matches!(account.status, FundraiserStatus::Active));

    // Verify Donation state
    let (donation_pda, _) = ctx.svm.get_pda_with_bump(
        &[b"donation", fundraiser.as_ref(), donor.pubkey().as_ref()],
        &crate::gofundme::ID,
    );
    let donation_account: crate::gofundme::accounts::Donation =
        ctx.get_account(&donation_pda).unwrap();
    assert_eq!(donation_account.amount, donation_amount);
    assert_eq!(donation_account.donor, donor.pubkey());
    assert_eq!(donation_account.fundraiser, fundraiser);
    assert!(!donation_account.refunded);
    assert!(donation_account.timestamp > 0);
}

#[test]
fn test_donate_goal_met_completes_campaign() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(20_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    let donation_amount = 5_000_000_000; // Donate exactly the goal amount
    let result = donate_helper(&mut ctx, &donor, fundraiser, vault, donation_amount);
    result.assert_success();

    let account: crate::gofundme::accounts::Fundraiser = ctx.get_account(&fundraiser).unwrap();
    assert_eq!(account.raised_amount, donation_amount);
    // Should now be Closed! (Goal met/exceeded rule)
    assert!(matches!(account.status, FundraiserStatus::Closed));
}

// ============================================================================
// Errors & Validation
// ============================================================================

#[test]
fn test_donate_inactive_campaign_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor1 = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor2 = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    // First donation completes the campaign
    donate_helper(&mut ctx, &donor1, fundraiser, vault, 5_000_000_000).assert_success();

    // Second donation should fail because campaign is now Closed (not Active)
    let result = donate_helper(&mut ctx, &donor2, fundraiser, vault, 1_000_000_000);
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6005)")); // CampaignNotActive
}

#[test]
fn test_donate_deadline_passed_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    // Warp clock into the future (past 14 days)
    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp += 15 * 24 * 60 * 60; // Advance 15 days
    ctx.svm.set_sysvar(&clock);

    // Donation should fail due to deadline
    let result = donate_helper(&mut ctx, &donor, fundraiser, vault, 1_000_000_000);
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6006)")); // DeadlinePassed
}

#[test]
fn test_donate_invalid_amount_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    // Donate 0 SOL
    let result = donate_helper(&mut ctx, &donor, fundraiser, vault, 0);
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6007)")); // InvalidDonationAmount
}

#[test]
fn test_donate_invalid_vault_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let (fundraiser, _correct_vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    // Create a malicious vault (just some random account)
    let malicious_vault = ctx.svm.create_funded_account(0).unwrap();

    let result = donate_helper(
        &mut ctx,
        &donor,
        fundraiser,
        malicious_vault.pubkey(),
        1_000_000_000,
    );
    result.assert_failure();
    // Anchor will fail with ConstraintAddress because address != fundraiser.vault
}
