use borsh::BorshSerialize;
use multisig::{multisig::MultisigError, proposal::Action, Instruction};
use solana_program::{
    instruction::{AccountMeta, Instruction as SolanaInstruction, InstructionError},
    system_program,
};
use solana_program_test::{tokio, BanksClientError};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::TransactionError};

mod helpers;
use crate::helpers::{
    approve_proposal, create_multisig, create_proposal, execute_transaction, get_multisig_data,
    prepare, sol, transfer_sol,
};

#[tokio::test]
async fn test_create_multisig() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;

    let multisig_data = get_multisig_data(&mut context.banks_client, multisig_pda)
        .await
        .unwrap();
    assert_eq!(multisig_data.name, multisig_name);
    assert_eq!(multisig_data.threshold, 1);
    assert_eq!(multisig_data.members, vec![owner.pubkey()]);
}

#[tokio::test]
async fn test_add_member_fail() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;

    let new_member = Keypair::new();

    let add_member_instruction = Instruction::AddMember {
        member: new_member.pubkey(),
    };
    let add_member_result = execute_transaction(
        &mut context.banks_client,
        vec![SolanaInstruction::new_with_bytes(
            program_id,
            &add_member_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;
    match add_member_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::InvalidAccountData,
        ))) => (),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_remove_member_fail() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;

    let remove_member_instruction = Instruction::RemoveMember {
        member: owner.pubkey(),
    };
    let remove_member_result = execute_transaction(
        &mut context.banks_client,
        vec![SolanaInstruction::new_with_bytes(
            program_id,
            &remove_member_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;
    match remove_member_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::InvalidAccountData,
        ))) => (),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_change_threshold_fail() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;

    let change_threshold_instruction = Instruction::ChangeThreshold { threshold: 2 };
    let change_threshold_result = execute_transaction(
        &mut context.banks_client,
        vec![SolanaInstruction::new_with_bytes(
            program_id,
            &change_threshold_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;
    match change_threshold_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::InvalidAccountData,
        ))) => (),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_remove_member_invalid_threshold() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;
    transfer_sol(&mut context.banks_client, &owner, &multisig_pda, sol(2.0))
        .await
        .unwrap();

    let new_member = Keypair::new();
    let add_member_action = Action {
        program_id,
        accounts: vec![multisig_pda, program_id, system_program::id()],
        data: Instruction::AddMember {
            member: new_member.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
    };
    let increase_threshold_action = Action {
        program_id,
        accounts: vec![multisig_pda, program_id, system_program::id()],
        data: Instruction::ChangeThreshold { threshold: 2 }
            .try_to_vec()
            .unwrap(),
    };
    let remove_member_action = Action {
        program_id,
        accounts: vec![multisig_pda, program_id, system_program::id()],
        data: Instruction::RemoveMember {
            member: new_member.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
    };

    let proposal_pda = create_proposal(
        &program_id,
        &mut context.banks_client,
        &multisig_pda,
        &owner,
        0,
        b"test".to_vec(),
        b"test description".to_vec(),
        vec![
            add_member_action.clone(),
            increase_threshold_action.clone(),
            remove_member_action.clone(),
        ],
    )
    .await
    .unwrap();

    let approve_proposal_result = approve_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        true,
    )
    .await;

    match approve_proposal_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, MultisigError::ThresholdTooHigh as u32),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_too_high_threshold() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;
    transfer_sol(&mut context.banks_client, &owner, &multisig_pda, sol(2.0))
        .await
        .unwrap();

    let increase_threshold_action = Action {
        program_id,
        accounts: vec![multisig_pda, program_id, system_program::id()],
        data: Instruction::ChangeThreshold { threshold: 2 }
            .try_to_vec()
            .unwrap(),
    };

    let proposal_pda = create_proposal(
        &program_id,
        &mut context.banks_client,
        &multisig_pda,
        &owner,
        0,
        b"test".to_vec(),
        b"test description".to_vec(),
        vec![increase_threshold_action.clone()],
    )
    .await
    .unwrap();

    let approve_proposal_result = approve_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        true,
    )
    .await;

    match approve_proposal_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, MultisigError::ThresholdTooHigh as u32),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_too_low_threshold() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;
    transfer_sol(&mut context.banks_client, &owner, &multisig_pda, sol(2.0))
        .await
        .unwrap();

    let increase_threshold_action = Action {
        program_id,
        accounts: vec![multisig_pda, program_id, system_program::id()],
        data: Instruction::ChangeThreshold { threshold: 0 }
            .try_to_vec()
            .unwrap(),
    };

    let proposal_pda = create_proposal(
        &program_id,
        &mut context.banks_client,
        &multisig_pda,
        &owner,
        0,
        b"test".to_vec(),
        b"test description".to_vec(),
        vec![increase_threshold_action.clone()],
    )
    .await
    .unwrap();

    let approve_proposal_result = approve_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        true,
    )
    .await;

    match approve_proposal_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, MultisigError::ThresholdTooLow as u32),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_no_member_left() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_name = b"test".to_vec();
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &multisig_name,
        vec![owner.pubkey()],
    )
    .await;
    transfer_sol(&mut context.banks_client, &owner, &multisig_pda, sol(2.0))
        .await
        .unwrap();

    let remove_member_action = Action {
        program_id,
        accounts: vec![multisig_pda, program_id, system_program::id()],
        data: Instruction::RemoveMember {
            member: owner.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
    };

    let proposal_pda = create_proposal(
        &program_id,
        &mut context.banks_client,
        &multisig_pda,
        &owner,
        0,
        b"test".to_vec(),
        b"test description".to_vec(),
        vec![remove_member_action.clone()],
    )
    .await
    .unwrap();

    let approve_proposal_result = approve_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        true,
    )
    .await;

    match approve_proposal_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, MultisigError::NoMembers as u32),
        _ => panic!("expected error"),
    }
}
