use anchor_litesvm::{AnchorLiteSVM, Signer, TestHelpers};

use crate::gofundme::client::accounts::InitializeFundraiser;
use crate::gofundme::client::args::InitializeFundraiser as InitializeFundraiserArgs;

/// Helper: build and execute an initialize_fundraiser instruction
fn create_fundraiser(
    ctx: &mut anchor_litesvm::AnchorContext,
    creator: &anchor_litesvm::Keypair,
    fundraiser_index: u8,
    title: String,
    description: String,
    goal_amount: u64,
    deadline: i64,
) -> anchor_litesvm::TransactionResult {
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
            title,
            description,
            goal_amount,
            deadline,
        })
        .instruction()
        .unwrap();

    ctx.execute_instruction(ix, &[creator]).unwrap()
}

// ============================================================================
// Happy path
// ============================================================================

#[test]
fn test_initialize_fundraiser_success() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let fundraiser_index: u8 = 0;
    let goal_amount: u64 = 5_000_000_000; // 5 SOL
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        fundraiser_index,
        "Save the Whales".to_string(),
        "Help us protect whale habitats worldwide.".to_string(),
        goal_amount,
        deadline,
    );
    result.assert_success();

    // Deserialize and verify on-chain state
    let (fundraiser_pda, _) = ctx.svm.get_pda_with_bump(
        &[
            b"fundraiser",
            creator.pubkey().as_ref(),
            fundraiser_index.to_le_bytes().as_ref(),
        ],
        &crate::gofundme::ID,
    );

    let account: crate::gofundme::accounts::Fundraiser = ctx.get_account(&fundraiser_pda).unwrap();
    assert_eq!(account.creator, creator.pubkey());
    assert_eq!(account.title, "Save the Whales");
    assert_eq!(
        account.description,
        "Help us protect whale habitats worldwide."
    );
    assert_eq!(account.goal_amount, goal_amount);
    assert_eq!(account.deadline, deadline);
    assert_eq!(account.raised_amount, 0);
}

// ============================================================================
// Validation error tests
// ============================================================================

#[test]
fn test_empty_title_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "".to_string(),
        "Valid description".to_string(),
        5_000_000_000,
        deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6000)")); // TitleTooLong
}

#[test]
fn test_title_too_long_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "A".repeat(51),
        "Valid description".to_string(),
        5_000_000_000,
        deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6000)")); // TitleTooLong
}

#[test]
fn test_empty_description_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Valid Title".to_string(),
        "".to_string(),
        5_000_000_000,
        deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6001)")); // DescriptionTooLong
}

#[test]
fn test_goal_too_small_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Valid Title".to_string(),
        "Valid description".to_string(),
        999_999_999,
        deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6002)")); // GoalTooSmall
}

#[test]
fn test_deadline_in_past_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let past_deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        - 1;

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Valid Title".to_string(),
        "Valid description".to_string(),
        5_000_000_000,
        past_deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6003)")); // DeadlineInPast
}

#[test]
fn test_deadline_too_short_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let short_deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (3 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Valid Title".to_string(),
        "Valid description".to_string(),
        5_000_000_000,
        short_deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6004)")); // InvalidDeadline
}

#[test]
fn test_deadline_too_long_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let long_deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (60 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Valid Title".to_string(),
        "Valid description".to_string(),
        5_000_000_000,
        long_deadline,
    );
    result.assert_failure();
    assert!(format!("{:?}", result).contains("Custom(6004)")); // InvalidDeadline
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_multiple_fundraisers_same_creator() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result0 = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "First Campaign".to_string(),
        "First campaign description".to_string(),
        2_000_000_000,
        deadline,
    );
    result0.assert_success();

    let result1 = create_fundraiser(
        &mut ctx,
        &creator,
        1,
        "Second Campaign".to_string(),
        "Second campaign description".to_string(),
        3_000_000_000,
        deadline,
    );
    result1.assert_success();
}

#[test]
fn test_duplicate_fundraiser_index_fails() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (14 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "First Campaign".to_string(),
        "First description".to_string(),
        2_000_000_000,
        deadline,
    );
    result.assert_success();

    let result2 = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Duplicate Campaign".to_string(),
        "Duplicate description".to_string(),
        2_000_000_000,
        deadline,
    );
    result2.assert_failure();
}

#[test]
fn test_minimum_valid_deadline() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (7 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Edge Case".to_string(),
        "Testing minimum deadline boundary".to_string(),
        1_000_000_000,
        deadline,
    );
    result.assert_success();
}

#[test]
fn test_maximum_valid_deadline() {
    let mut ctx = AnchorLiteSVM::build_with_program(
        crate::gofundme::ID,
        include_bytes!("../../target/deploy/gofundme.so"),
    );

    let creator = ctx.svm.create_funded_account(10_000_000_000).unwrap();
    let deadline = ctx
        .svm
        .get_sysvar::<anchor_lang::prelude::Clock>()
        .unix_timestamp
        + (30 * 24 * 60 * 60);

    let result = create_fundraiser(
        &mut ctx,
        &creator,
        0,
        "Edge Case".to_string(),
        "Testing maximum deadline boundary".to_string(),
        1_000_000_000,
        deadline,
    );
    result.assert_success();
}
