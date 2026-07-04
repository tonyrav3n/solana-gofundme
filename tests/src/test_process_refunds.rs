use anchor_lang::prelude::Clock;
use anchor_litesvm::{AnchorLiteSVM, Signer, TestHelpers};

use crate::gofundme::client::accounts::{Donate, InitializeFundraiser, ProcessRefunds, Withdraw};
use crate::gofundme::client::args::{
    Donate as DonateArgs, InitializeFundraiser as InitializeFundraiserArgs, ProcessRefunds as ProcessRefundsArgs,
};

fn setup_campaign_and_donate(
    ctx: &mut anchor_litesvm::AnchorContext,
    creator: &anchor_litesvm::Keypair,
    donor: &anchor_litesvm::Keypair,
    fundraiser_index: u8,
    goal_amount: u64,
    deadline_offset_days: i64,
    donation_amount: u64,
) -> (anchor_litesvm::Pubkey, anchor_litesvm::Pubkey, anchor_litesvm::Pubkey) {
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

    // Pre-fund vault PDA so it exists before transactions
    ctx.svm.airdrop(&vault_pda, 1_000_000_000).unwrap(); // 1 SOL to vault for safety margin

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

    let (donation_pda, _) = ctx.svm.get_pda_with_bump(
        &[b"donation", fundraiser_pda.as_ref(), donor.pubkey().as_ref()],
        &crate::gofundme::ID,
    );

    let donate_ix = ctx
        .program()
        .accounts(Donate {
            donor: donor.pubkey(),
            fundraiser: fundraiser_pda,
            vault: vault_pda,
            donation: donation_pda,
            system_program: anchor_lang::system_program::ID,
        })
        .args(DonateArgs { amount: donation_amount })
        .instruction()
        .unwrap();

    ctx.execute_instruction(donate_ix, &[donor])
        .unwrap()
        .assert_success();

    (fundraiser_pda, vault_pda, donation_pda)
}

fn process_refund_helper(
    ctx: &mut anchor_litesvm::AnchorContext,
    donor: &anchor_litesvm::Keypair,
    fundraiser: anchor_litesvm::Pubkey,
    vault: anchor_litesvm::Pubkey,
    donation: anchor_litesvm::Pubkey,
) -> anchor_litesvm::TransactionResult {
    let ix = ctx
        .program()
        .accounts(ProcessRefunds {
            fundraiser,
            vault,
            donation,
            donor_account: donor.pubkey(),
            system_program: anchor_lang::system_program::ID,
        })
        .args(ProcessRefundsArgs {
            donor: donor.pubkey(),
        })
        .instruction()
        .unwrap();

    // Create a temporary signer just to sign the transaction (instruction itself is permissionless)
    let temp_signer = ctx.svm.create_funded_account(1_000_000).unwrap();
    ctx.execute_instruction(ix, &[&temp_signer]).unwrap()
}

#[test]
fn test_process_refunds_success() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1_000_000_000;
    ctx.svm.set_sysvar(&clock);

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let donation_amount = 2_000_000_000;
    let deadline_offset = 14;

    let (fundraiser, vault, donation) = setup_campaign_and_donate(
        &mut ctx,
        &creator,
        &donor,
        0,
        goal_amount,
        deadline_offset,
        donation_amount,
    );

    // Fast forward to 7+ days after the deadline
    clock.unix_timestamp += (deadline_offset + 8) * 24 * 60 * 60;
    ctx.svm.set_sysvar(&clock);

    let initial_donor_balance = ctx.svm.get_balance(&donor.pubkey()).unwrap();

    let result = process_refund_helper(&mut ctx, &donor, fundraiser, vault, donation);
    result.assert_success();

    let final_donor_balance = ctx.svm.get_balance(&donor.pubkey()).unwrap();
    // Donor gets back the donation amount (minus trx fee)
    assert!(final_donor_balance > initial_donor_balance + donation_amount - 100_000);

    // Check that the donation record is marked as refunded
    let donation_account: crate::gofundme::accounts::Donation = ctx.get_account(&donation).unwrap();
    assert!(donation_account.refunded);
}

#[test]
fn test_process_refunds_not_abandoned_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1_000_000_000;
    ctx.svm.set_sysvar(&clock);

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let donation_amount = 2_000_000_000;
    let deadline_offset = 14;

    let (fundraiser, vault, donation) = setup_campaign_and_donate(
        &mut ctx,
        &creator,
        &donor,
        0,
        goal_amount,
        deadline_offset,
        donation_amount,
    );

    // Fast forward to exactly the deadline, not abandoned yet
    clock.unix_timestamp += deadline_offset * 24 * 60 * 60;
    ctx.svm.set_sysvar(&clock);

    let result = process_refund_helper(&mut ctx, &donor, fundraiser, vault, donation);
    result.assert_failure();
    // ErrorCode::NotAbandoned -> 6011
    assert!(format!("{:?}", result).contains("Custom(6011)"));
}

#[test]
fn test_process_refunds_already_refunded_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1_000_000_000;
    ctx.svm.set_sysvar(&clock);

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let donation_amount = 2_000_000_000;
    let deadline_offset = 14;

    let (fundraiser, vault, donation) = setup_campaign_and_donate(
        &mut ctx,
        &creator,
        &donor,
        0,
        goal_amount,
        deadline_offset,
        donation_amount,
    );

    // Fast forward to 8 days after deadline
    clock.unix_timestamp += (deadline_offset + 8) * 24 * 60 * 60;
    ctx.svm.set_sysvar(&clock);

    // First refund succeeds
    process_refund_helper(&mut ctx, &donor, fundraiser, vault, donation).assert_success();

    // Second refund fails (permissionless, so no signer change needed)
    let result = process_refund_helper(&mut ctx, &donor, fundraiser, vault, donation);
    result.assert_failure();
    // ErrorCode::AlreadyRefunded -> 6012
    assert!(format!("{:?}", result).contains("Custom(6012)"));
}

#[test]
fn test_process_refunds_insufficient_vault_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1_000_000_000;
    ctx.svm.set_sysvar(&clock);

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let donation_amount = 2_000_000_000;
    let deadline_offset = 14;

    let (fundraiser, vault, donation) = setup_campaign_and_donate(
        &mut ctx,
        &creator,
        &donor,
        0,
        goal_amount,
        deadline_offset,
        donation_amount,
    );

    // The creator withdraws the funds!
    let ix = ctx
        .program()
        .accounts(Withdraw {
            creator: creator.pubkey(),
            fundraiser,
            vault,
            system_program: anchor_lang::system_program::ID,
        })
        .args(crate::gofundme::client::args::Withdraw {})
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[&creator])
        .unwrap()
        .assert_success();

    // Fast forward to 8 days after deadline
    clock.unix_timestamp += (deadline_offset + 8) * 24 * 60 * 60;
    ctx.svm.set_sysvar(&clock);

    // Refund should fail because vault is empty
    let result = process_refund_helper(&mut ctx, &donor, fundraiser, vault, donation);
    result.assert_failure();
    // ErrorCode::InsufficientVaultBalance -> 6010
    assert!(format!("{:?}", result).contains("Custom(6010)"));
}

#[test]
fn test_process_refunds_multiple_donors() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let mut clock = ctx.svm.get_sysvar::<Clock>();
    clock.unix_timestamp = 1_000_000_000;
    ctx.svm.set_sysvar(&clock);

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor1 = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor2 = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let donation1 = 1_500_000_000;
    let donation2 = 1_500_000_000;
    let deadline_offset = 14;

    // Setup campaign with first donor
    let (fundraiser, vault, donation_pda1) = setup_campaign_and_donate(
        &mut ctx,
        &creator,
        &donor1,
        0,
        goal_amount,
        deadline_offset,
        donation1,
    );

    // Second donor donates
    let (donation_pda2, _) = ctx.svm.get_pda_with_bump(
        &[b"donation", fundraiser.as_ref(), donor2.pubkey().as_ref()],
        &crate::gofundme::ID,
    );

    let donate_ix = ctx
        .program()
        .accounts(Donate {
            donor: donor2.pubkey(),
            fundraiser,
            vault,
            donation: donation_pda2,
            system_program: anchor_lang::system_program::ID,
        })
        .args(DonateArgs { amount: donation2 })
        .instruction()
        .unwrap();

    ctx.execute_instruction(donate_ix, &[&donor2])
        .unwrap()
        .assert_success();

    // Fast forward to 8 days after deadline
    clock.unix_timestamp += (deadline_offset + 8) * 24 * 60 * 60;
    ctx.svm.set_sysvar(&clock);

    let initial_donor1_balance = ctx.svm.get_balance(&donor1.pubkey()).unwrap();
    let initial_donor2_balance = ctx.svm.get_balance(&donor2.pubkey()).unwrap();

    // Process refund for donor1
    process_refund_helper(&mut ctx, &donor1, fundraiser, vault, donation_pda1).assert_success();

    // Process refund for donor2
    process_refund_helper(&mut ctx, &donor2, fundraiser, vault, donation_pda2).assert_success();

    let final_donor1_balance = ctx.svm.get_balance(&donor1.pubkey()).unwrap();
    let final_donor2_balance = ctx.svm.get_balance(&donor2.pubkey()).unwrap();

    // Both donors should get their donations back
    assert!(final_donor1_balance > initial_donor1_balance + donation1 - 100_000);
    assert!(final_donor2_balance > initial_donor2_balance + donation2 - 100_000);

    // Check both marked as refunded
    let donation1_account: crate::gofundme::accounts::Donation = ctx.get_account(&donation_pda1).unwrap();
    let donation2_account: crate::gofundme::accounts::Donation = ctx.get_account(&donation_pda2).unwrap();
    assert!(donation1_account.refunded);
    assert!(donation2_account.refunded);
}
