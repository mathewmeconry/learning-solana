use borsh::BorshSerialize;
use helpers::{create_multisig, prepare};
use multisig::{
    multisig::MultisigError,
    proposal::{Action, ProposalError},
    Instruction,
};
use solana_program::{
    instruction::{AccountMeta, Instruction as SolanaInstruction, InstructionError},
    system_program,
};
use solana_program_test::{tokio, BanksClientError};
use solana_sdk::{signature::Keypair, signer::Signer, transaction::TransactionError};

use crate::helpers::{
    approve_proposal, create_proposal, execute_proposal, execute_transaction, get_multisig_data,
    get_proposal_data, sol, transfer_sol,
};

mod helpers;

#[tokio::test]
async fn test_create_proposal() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &b"test".to_vec(),
        vec![owner.pubkey()],
    )
    .await;

    let new_member = Keypair::new();
    let add_member_action = Action {
        program_id,
        accounts: vec![
            (multisig_pda, true, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
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
        vec![add_member_action.clone()],
    )
    .await
    .unwrap();

    let proposal = get_proposal_data(&mut context.banks_client, proposal_pda)
        .await
        .unwrap();
    assert_eq!(proposal.name, b"test");
    assert_eq!(proposal.description, b"test description");
    assert_eq!(proposal.approvers, vec![]);
    assert_eq!(proposal.executed, false);
    assert_eq!(proposal.multisig, multisig_pda);

    for action in proposal.actions {
        assert_eq!(action.program_id, add_member_action.program_id);
        assert_eq!(action.accounts, add_member_action.accounts);
        assert_eq!(action.data, add_member_action.data);
    }
}

#[tokio::test]
async fn test_create_proposal_non_member() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &b"test".to_vec(),
        vec![owner.pubkey()],
    )
    .await;

    let new_member = Keypair::new();
    let not_member = Keypair::new();
    transfer_sol(
        &mut context.banks_client,
        &owner,
        &not_member.pubkey(),
        sol(1.0),
    )
    .await
    .unwrap();
    let add_member_action = Action {
        program_id,
        accounts: vec![
            (multisig_pda, true, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
            member: new_member.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
    };

    let create_proposal_result = create_proposal(
        &program_id,
        &mut context.banks_client,
        &multisig_pda,
        &not_member,
        0,
        b"test".to_vec(),
        b"test description".to_vec(),
        vec![add_member_action.clone()],
    )
    .await;

    match create_proposal_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, MultisigError::NotAMember as u32),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_approve_proposal() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &b"test".to_vec(),
        vec![owner.pubkey()],
    )
    .await;

    let new_member = Keypair::new();
    let add_member_action = Action {
        program_id,
        accounts: vec![
            (multisig_pda, true, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
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
        vec![add_member_action.clone()],
    )
    .await
    .unwrap();

    let approve_proposal_instruction = Instruction::Approve { try_execute: false };
    execute_transaction(
        &mut context.banks_client,
        vec![SolanaInstruction::new_with_bytes(
            program_id,
            &approve_proposal_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(multisig_pda, false),
                AccountMeta::new(proposal_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await
    .unwrap();

    let proposal = get_proposal_data(&mut context.banks_client, proposal_pda)
        .await
        .unwrap();
    assert_eq!(proposal.approvers, vec![owner.pubkey()]);
    assert_eq!(proposal.executed, false);
}

#[tokio::test]
async fn test_approve_proposal_non_member() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &b"test".to_vec(),
        vec![owner.pubkey()],
    )
    .await;

    let new_member = Keypair::new();
    let non_member = Keypair::new();
    transfer_sol(
        &mut context.banks_client,
        &owner,
        &non_member.pubkey(),
        sol(1.0),
    )
    .await
    .unwrap();
    let add_member_action = Action {
        program_id,
        accounts: vec![
            (multisig_pda, true, false),
            (system_program::id(), true, false),
        ],
        data: Instruction::AddMember {
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
        vec![add_member_action.clone()],
    )
    .await
    .unwrap();

    let approve_proposal_instruction = Instruction::Approve { try_execute: false };
    let approve_result = execute_transaction(
        &mut context.banks_client,
        vec![SolanaInstruction::new_with_bytes(
            program_id,
            &approve_proposal_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(non_member.pubkey(), true),
                AccountMeta::new(multisig_pda, false),
                AccountMeta::new(proposal_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&non_member],
    )
    .await;

    match approve_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, MultisigError::NotAMember as u32),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_approve_proposal_and_execute() {
    let (mut context, program_id, owner) = prepare().await;
    let multisig_pda = create_multisig(
        &program_id,
        &mut context.banks_client,
        &owner,
        &b"test".to_vec(),
        vec![owner.pubkey()],
    )
    .await;
    transfer_sol(&mut context.banks_client, &owner, &multisig_pda, sol(2.0))
        .await
        .unwrap();

    let new_member = Keypair::new();
    let add_member_action = Action {
        program_id: program_id,
        accounts: vec![
            (multisig_pda, true, true),
            (program_id, false, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
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
        vec![add_member_action.clone()],
    )
    .await
    .unwrap();

    let approve_proposal_instruction = Instruction::Approve { try_execute: true };
    execute_transaction(
        &mut context.banks_client,
        vec![SolanaInstruction::new_with_bytes(
            program_id,
            &approve_proposal_instruction.try_to_vec().unwrap(),
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
        )],
        vec![&owner],
    )
    .await
    .unwrap();

    let proposal = get_proposal_data(&mut context.banks_client, proposal_pda)
        .await
        .unwrap();
    assert_eq!(proposal.approvers, vec![owner.pubkey()]);
    assert_eq!(proposal.executed, true);

    let multisig = get_multisig_data(&mut context.banks_client, multisig_pda)
        .await
        .unwrap();
    assert_eq!(multisig.members, vec![owner.pubkey(), new_member.pubkey()]);
}

#[tokio::test]
async fn test_no_execute_without_threshold() {
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
        accounts: vec![
            (multisig_pda, true, true),
            (program_id, false, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
            member: new_member.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
    };
    let increase_threshold_action = Action {
        program_id,
        accounts: vec![
            (multisig_pda, true, true),
            (program_id, false, false),
            (system_program::id(), false, false),
        ],
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
        vec![add_member_action.clone(), increase_threshold_action.clone()],
    )
    .await
    .unwrap();

    approve_proposal(
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
        ],
        true,
    )
    .await
    .unwrap();

    let new_member_2 = Keypair::new();
    let add_member_2_action = Action {
        program_id,
        accounts: vec![
            (multisig_pda, true, true),
            (program_id, false, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
            member: new_member_2.pubkey(),
        }
        .try_to_vec()
        .unwrap(),
    };
    let proposal_pda_2 = create_proposal(
        &program_id,
        &mut context.banks_client,
        &multisig_pda,
        &owner,
        1,
        b"test".to_vec(),
        b"test description".to_vec(),
        vec![add_member_2_action.clone()],
    )
    .await
    .unwrap();

    approve_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda_2, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        false,
    )
    .await
    .unwrap();

    let execute_result = execute_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    )
    .await;

    match execute_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, ProposalError::NotEnoughApprovals as u32),
        _ => panic!("expected error"),
    }

    transfer_sol(
        &mut context.banks_client,
        &owner,
        &new_member.pubkey(),
        sol(1.0),
    )
    .await
    .unwrap();
    approve_proposal(
        &program_id,
        &mut context.banks_client,
        &new_member,
        vec![
            AccountMeta::new(new_member.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda_2, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(new_member.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda_2, false),
            AccountMeta::new(multisig_pda, false),
            AccountMeta::new_readonly(program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        true,
    )
    .await
    .unwrap();

    let multisig = get_multisig_data(&mut context.banks_client, multisig_pda)
        .await
        .unwrap();
    assert_eq!(multisig.threshold, 2);
    assert_eq!(
        multisig.members,
        vec![owner.pubkey(), new_member.pubkey(), new_member_2.pubkey()]
    );
}

#[tokio::test]
async fn test_execute_proposal_once() {
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
        accounts: vec![
            (multisig_pda, true, false),
            (program_id, false, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
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
        vec![add_member_action.clone()],
    )
    .await
    .unwrap();

    approve_proposal(
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
    .await
    .unwrap();

    let execute_result = execute_proposal(
        &program_id,
        &mut context.banks_client,
        &owner,
        vec![
            AccountMeta::new(owner.pubkey(), true),
            AccountMeta::new_readonly(multisig_pda, false),
            AccountMeta::new(proposal_pda, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    )
    .await;

    match execute_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, ProposalError::AlreadyExecuted as u32),
        _ => panic!("expected error"),
    }
}

#[tokio::test]
async fn test_cannot_approve_twice() {
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
        accounts: vec![
            (multisig_pda, true, true),
            (program_id, false, false),
            (system_program::id(), false, false),
        ],
        data: Instruction::AddMember {
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
        vec![add_member_action.clone()],
    )
    .await
    .unwrap();

    approve_proposal(
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
        false,
    )
    .await
    .unwrap();

    let approve_2_result = approve_proposal(
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
        false,
    )
    .await;

    match approve_2_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, ProposalError::AlreadyApproved as u32),
        _ => panic!("expected error"),
    }
}
