use anchor_lang::prelude::Clock;
use anchor_litesvm::{AnchorLiteSVM, Signer, TestHelpers};

use crate::gofundme::client::accounts::{Donate, InitializeFundraiser, Withdraw};
use crate::gofundme::client::args::{
    Donate as DonateArgs, InitializeFundraiser as InitializeFundraiserArgs, Withdraw as WithdrawArgs,
};

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

fn withdraw_helper(
    ctx: &mut anchor_litesvm::AnchorContext,
    creator: &anchor_litesvm::Keypair,
    fundraiser: anchor_litesvm::Pubkey,
    vault: anchor_litesvm::Pubkey,
) -> anchor_litesvm::TransactionResult {
    let ix = ctx
        .program()
        .accounts(Withdraw {
            creator: creator.pubkey(),
            fundraiser,
            vault,
            system_program: anchor_lang::system_program::ID,
        })
        .args(WithdrawArgs {})
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[creator]).unwrap()
}

// ============================================================================
// Happy path
// ============================================================================

#[test]
fn test_withdraw_success_all_funds() {
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
    donate_helper(&mut ctx, &donor, fundraiser, vault, donation_amount).assert_success();

    let pre_withdraw_creator_balance = ctx.svm.get_balance(&creator.pubkey()).unwrap();
    let pre_withdraw_vault_balance = ctx.svm.get_balance(&vault).unwrap();

    let result = withdraw_helper(&mut ctx, &creator, fundraiser, vault);
    result.assert_success();

    let post_withdraw_creator_balance = ctx.svm.get_balance(&creator.pubkey()).unwrap();
    let post_withdraw_vault_balance = ctx.svm.get_balance(&vault).unwrap();

    // Vault should have only rent exemption remaining
    let rent_exemption = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Rent>()
        .minimum_balance(0);
    assert_eq!(post_withdraw_vault_balance, rent_exemption);

    let amount_withdrawn = pre_withdraw_vault_balance - rent_exemption;

    // Creator balance should increase by the withdrawn amount (minus transaction fee)
    assert!(post_withdraw_creator_balance > pre_withdraw_creator_balance);
    assert!(post_withdraw_creator_balance <= pre_withdraw_creator_balance + amount_withdrawn);
}

// ============================================================================
// Errors & Validation
// ============================================================================

#[test]
fn test_withdraw_not_creator_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let donor = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let hacker = ctx.svm.create_funded_account(10_000_000_000).unwrap();

    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    donate_helper(&mut ctx, &donor, fundraiser, vault, 1_000_000_000).assert_success();

    // Hacker tries to withdraw
    let result = withdraw_helper(&mut ctx, &hacker, fundraiser, vault);
    result.assert_failure();
    // NotCreator (6009), returned via the `has_one = creator @ ErrorCode::NotCreator` constraint
    assert!(format!("{:?}", result).contains("Custom(6009)"));
}

#[test]
fn test_withdraw_empty_vault_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let goal_amount = 5_000_000_000;
    let (fundraiser, vault) = setup_fundraiser(&mut ctx, &creator, 0, goal_amount, 14);

    // Vault has 0 donations, just rent exemption.
    let result = withdraw_helper(&mut ctx, &creator, fundraiser, vault);
    result.assert_failure();

    // InsufficientVaultBalance (6010)
    assert!(format!("{:?}", result).contains("Custom(6010)"));
}
