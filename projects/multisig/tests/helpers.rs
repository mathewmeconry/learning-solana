use borsh::{BorshDeserialize, BorshSerialize};
use multisig::{
    multisig::Multisig,
    process_instruction,
    proposal::{Action, Proposal},
    Instruction,
};
use solana_program::{
    instruction::{AccountMeta, Instruction as SolanaInstruction},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use solana_program_test::{
    processor, BanksClient, BanksClientError, ProgramTest, ProgramTestContext,
};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};

pub fn sol(amount: f64) -> u64 {
    (amount * LAMPORTS_PER_SOL as f64) as u64
}

pub async fn execute_transaction(
    client: &mut BanksClient,
    instructions: Vec<SolanaInstruction>,
    signers: Vec<&Keypair>,
) -> Result<Signature, BanksClientError> {
    let mut tx = Transaction::new_with_payer(&instructions, Some(&signers[0].pubkey()));
    tx.sign(&signers, client.get_latest_blockhash().await?);
    let sig = tx.signatures[0];
    let result = client.process_transaction(tx).await;

    return match result {
        Err(_) => Err(result.unwrap_err()),
        Ok(_) => Ok(sig),
    };
}

pub async fn transfer_sol(
    context: &mut BanksClient,
    payer: &Keypair,
    receiver: &Pubkey,
    amount: u64,
) -> Result<Signature, BanksClientError> {
    let ixs = vec![system_instruction::transfer(
        &payer.pubkey(),
        receiver,
        amount,
    )];
    execute_transaction(context, ixs, vec![payer]).await
}

pub async fn prepare<'a>() -> (ProgramTestContext, Pubkey, Keypair) {
    let program_id = Pubkey::new_unique();
    let mut context = ProgramTest::new("multisig", program_id, processor!(process_instruction))
        .start_with_context()
        .await;

    let owner = Keypair::new();
    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &owner.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    return (context, program_id, owner);
}

pub async fn get_multisig_data(
    banks_client: &mut BanksClient,
    pda_account: Pubkey,
) -> Result<Multisig, std::io::Error> {
    let account = banks_client
        .get_account_with_commitment(pda_account, CommitmentLevel::Finalized)
        .await
        .unwrap()
        .unwrap();

    return Multisig::try_from_slice(&account.data);
}

pub async fn get_proposal_data(
    banks_client: &mut BanksClient,
    pda_account: Pubkey,
) -> Result<Proposal, std::io::Error> {
    let account = banks_client
        .get_account_with_commitment(pda_account, CommitmentLevel::Finalized)
        .await
        .unwrap()
        .unwrap();

    return Proposal::try_from_slice(&account.data);
}

pub async fn create_multisig(
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    owner: &Keypair,
    name: &Vec<u8>,
    members: Vec<Pubkey>,
) -> Pubkey {
    let (multisig_pda, _) =
        Pubkey::find_program_address(&[b"multisig", program_id.as_ref(), name], &program_id);
    let create_multisig_instruction = Instruction::Create {
        name: name.clone(),
        members: members,
        threshold: 1,
    };

    execute_transaction(
        banks_client,
        vec![SolanaInstruction::new_with_bytes(
            *program_id,
            &create_multisig_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(multisig_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await
    .unwrap();

    return multisig_pda;
}

pub async fn create_proposal(
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    multisig: &Pubkey,
    creator: &Keypair,
    id: u64,
    name: Vec<u8>,
    description: Vec<u8>,
    actions: Vec<Action>,
) -> Result<Pubkey, BanksClientError> {
    let (proposal_pda, _) = Pubkey::find_program_address(
        &[
            b"proposal",
            program_id.as_ref(),
            multisig.as_ref(),
            &id.to_be_bytes(),
        ],
        program_id,
    );
    let create_proposal_instruction = Instruction::CreateProposal {
        id: id,
        name: name,
        description: description,
        actions: actions,
    };
    let transaction_result = execute_transaction(
        banks_client,
        vec![SolanaInstruction::new_with_bytes(
            *program_id,
            &create_proposal_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new(*multisig, false),
                AccountMeta::new(proposal_pda, false),
                AccountMeta::new(system_program::ID, false),
            ],
        )],
        vec![&creator],
    )
    .await;

    match transaction_result {
        Ok(_) => Ok(proposal_pda),
        Err(e) => Err(e),
    }
}

pub async fn approve_proposal(
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    creator: &Keypair,
    accounts: Vec<AccountMeta>,
    try_execute: bool,
) -> Result<(), BanksClientError> {
    let approve_proposal_instruction = Instruction::Approve { try_execute };
    let transaction_result = execute_transaction(
        banks_client,
        vec![SolanaInstruction::new_with_bytes(
            *program_id,
            &approve_proposal_instruction.try_to_vec().unwrap(),
            accounts,
        )],
        vec![&creator],
    )
    .await;

    match transaction_result {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

pub async fn execute_proposal(
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    creator: &Keypair,
    accounts: Vec<AccountMeta>,
) -> Result<(), BanksClientError> {
    let execute_proposal_instruction = Instruction::ExecuteProposal {};

    let transaction_result = execute_transaction(
        banks_client,
        vec![SolanaInstruction::new_with_bytes(
            *program_id,
            &execute_proposal_instruction.try_to_vec().unwrap(),
            accounts,
        )],
        vec![&creator],
    )
    .await;

    match transaction_result {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}
