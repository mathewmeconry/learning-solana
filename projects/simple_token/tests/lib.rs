use borsh::{BorshDeserialize, BorshSerialize};
use simple_token::{
    errors::SimpleTokenErrors,
    instructions as simple_token_instructions, process_instruction,
    storage::{Account, Config},
};
use solana_program::{
    instruction::{AccountMeta, Instruction, InstructionError},
    native_token::LAMPORTS_PER_SOL,
    pubkey::Pubkey,
    system_instruction, system_program,
};
use solana_program_test::{
    processor,
    tokio::{self},
    BanksClient, BanksClientError, ProgramTest, ProgramTestContext,
};
use solana_sdk::{
    account::ReadableAccount,
    commitment_config::CommitmentLevel,
    signature::{Keypair, Signature, Signer},
    transaction::{Transaction, TransactionError},
};

pub fn sol(amount: f64) -> u64 {
    (amount * LAMPORTS_PER_SOL as f64) as u64
}

async fn process_transaction(
    client: &mut BanksClient,
    instructions: Vec<Instruction>,
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

async fn transfer_sol(
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
    process_transaction(context, ixs, vec![payer]).await
}

async fn prepare() -> (ProgramTestContext, Pubkey, Keypair) {
    let program_id = Pubkey::new_unique();
    let mut context = ProgramTest::new("simple_token", program_id, processor!(process_instruction))
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

async fn get_config(banks_client: &mut BanksClient, program_id: &Pubkey) -> Config {
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);

    let config_account = banks_client
        .get_account_with_commitment(config_pda, CommitmentLevel::Finalized)
        .await
        .unwrap()
        .unwrap();

    return Config::try_from_slice(config_account.data()).unwrap();
}

async fn get_account(
    banks_client: &mut BanksClient,
    program_id: &Pubkey,
    account_pub_key: &Pubkey,
) -> Account {
    let (to_pda, _) = Pubkey::find_program_address(&[account_pub_key.as_ref()], &program_id);

    let account_account = banks_client
        .get_account_with_commitment(to_pda, CommitmentLevel::Finalized)
        .await
        .unwrap()
        .unwrap();

    return Account::try_from_slice(account_account.data()).unwrap();
}

async fn initialize(owner: &Keypair, program_id: &Pubkey, banks_client: &mut BanksClient) {
    let initialize_instruction = simple_token_instructions::Instruction::Initialize {
        owner: owner.pubkey(),
        decimals: 18,
    };
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], program_id);

    process_transaction(
        banks_client,
        vec![Instruction::new_with_bytes(
            *program_id,
            &initialize_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await
    .unwrap();
}

async fn mint_to(
    owner: &Keypair,
    to: &Keypair,
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    amount: u64,
) -> Result<Signature, BanksClientError> {
    let (to_pda, _) = Pubkey::find_program_address(&[to.pubkey().as_ref()], &program_id);
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let mint_instruction = simple_token_instructions::Instruction::Mint {
        to: to.pubkey(),
        amount: amount,
    };

    process_transaction(
        banks_client,
        vec![Instruction::new_with_bytes(
            *program_id,
            &mint_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(to_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await
}

async fn transfer_token(
    from: &Keypair,
    to: &Keypair,
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    amount: u64,
) -> Result<Signature, BanksClientError> {
    let (from_pda, _) = Pubkey::find_program_address(&[from.pubkey().as_ref()], &program_id);
    let (to_pda, _) = Pubkey::find_program_address(&[to.pubkey().as_ref()], &program_id);

    let transfer_instruction = simple_token_instructions::Instruction::Transfer {
        to: to.pubkey(),
        amount: amount,
    };

    process_transaction(
        banks_client,
        vec![Instruction::new_with_bytes(
            *program_id,
            &transfer_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(from.pubkey(), true),
                AccountMeta::new(from_pda, false),
                AccountMeta::new(to_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&from],
    )
    .await
}

async fn burn_from(
    owner: &Keypair,
    from: &Keypair,
    program_id: &Pubkey,
    banks_client: &mut BanksClient,
    amount: u64,
) -> Result<Signature, BanksClientError> {
    let (from_pda, _) = Pubkey::find_program_address(&[from.pubkey().as_ref()], &program_id);
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let burn_instruction = simple_token_instructions::Instruction::Burn {
        from: from.pubkey(),
        amount: amount,
    };

    process_transaction(
        banks_client,
        vec![Instruction::new_with_bytes(
            *program_id,
            &burn_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(from_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await
}

#[tokio::test]
async fn test_initialize() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;
    let config_data: Config = get_config(&mut context.banks_client, &program_id).await;
    assert_eq!(config_data.decimals, 18);
    assert_eq!(config_data.owner, owner.pubkey());
}

#[tokio::test]
async fn test_pda_has_lamports_initialize() {
    let (mut context, program_id, owner) = prepare().await;
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &config_pda,
        sol(1.0),
    )
    .await
    .unwrap();

    initialize(&owner, &program_id, &mut context.banks_client).await;
    let config_data: Config = get_config(&mut context.banks_client, &program_id).await;
    assert_eq!(config_data.decimals, 18);
    assert_eq!(config_data.owner, owner.pubkey());
}

#[tokio::test]
async fn test_change_owner() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let new_owner = Keypair::new();
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let change_owner_instruction = simple_token_instructions::Instruction::ChangeOwner {
        new_owner: new_owner.pubkey(),
    };

    process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &change_owner_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await
    .unwrap();

    let config_data: Config = get_config(&mut context.banks_client, &program_id).await;
    assert_eq!(config_data.decimals, 18);
    assert_eq!(config_data.owner, new_owner.pubkey());
}

#[tokio::test]
async fn test_fail_not_owner_change_owner() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let new_owner = Keypair::new();
    let not_owner = Keypair::new();
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &not_owner.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    let change_owner_instruction = simple_token_instructions::Instruction::ChangeOwner {
        new_owner: new_owner.pubkey(),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &change_owner_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(not_owner.pubkey(), true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&not_owner],
    )
    .await;
    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidOwner as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_invalid_pda_change_owner() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let new_owner = Keypair::new();
    let (not_config_pda, _) = Pubkey::find_program_address(&[b"not_config"], &program_id);

    let change_owner_instruction = simple_token_instructions::Instruction::ChangeOwner {
        new_owner: new_owner.pubkey(),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &change_owner_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(not_config_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_mint() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let to = Keypair::new();

    mint_to(
        &owner,
        &to,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await
    .unwrap();

    let account = get_account(&mut context.banks_client, &program_id, &to.pubkey()).await;
    assert_eq!(account.balance, sol(10.0));
}

#[tokio::test]
async fn test_fail_not_owner_mint() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let not_owner = Keypair::new();
    let to = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &not_owner.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    let transaction_result = mint_to(
        &not_owner,
        &to,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidOwner as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_invalid_config_pda_mint() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let to = Keypair::new();
    let (to_pda, _) = Pubkey::find_program_address(&[to.pubkey().as_ref()], &program_id);
    let (not_config_pda, _) = Pubkey::find_program_address(&[b"not_config"], &program_id);

    let mint_instruction = simple_token_instructions::Instruction::Mint {
        to: to.pubkey(),
        amount: sol(10.0),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &mint_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(not_config_pda, false),
                AccountMeta::new(to_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_invalid_to_pda_mint() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let to = Keypair::new();
    let (to_pda, _) = Pubkey::find_program_address(&[owner.pubkey().as_ref()], &program_id);
    let (config_pda, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    let mint_instruction = simple_token_instructions::Instruction::Mint {
        to: to.pubkey(),
        amount: sol(10.0),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &mint_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(config_pda, false),
                AccountMeta::new(to_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_overflow_mint() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &from.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(5.0),
    )
    .await
    .unwrap();

    let transaction_result = mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(u64::MAX as f64),
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::ArithmeticOverflow,
        ))) => assert_eq!(true, true),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_transfer() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    let to = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &from.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await
    .unwrap();

    transfer_token(&from, &to, &program_id, &mut context.banks_client, sol(5.0))
        .await
        .unwrap();

    let from_account = get_account(&mut context.banks_client, &program_id, &from.pubkey()).await;
    let to_account = get_account(&mut context.banks_client, &program_id, &from.pubkey()).await;

    assert_eq!(from_account.balance, sol(5.0));
    assert_eq!(to_account.balance, sol(5.0));
}

#[tokio::test]
async fn test_fail_not_enough_funds_transfer() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    let to = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &from.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(1.0),
    )
    .await
    .unwrap();

    let transaction_result =
        transfer_token(&from, &to, &program_id, &mut context.banks_client, sol(5.0)).await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::InsufficientFunds,
        ))) => assert_eq!(true, true),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_invalid_from_pda_transfer() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    let to = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &from.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(1.0),
    )
    .await
    .unwrap();

    let (to_pda, _) = Pubkey::find_program_address(&[to.pubkey().as_ref()], &program_id);

    let transfer_instruction = simple_token_instructions::Instruction::Transfer {
        to: to.pubkey(),
        amount: sol(5.0),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &transfer_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(from.pubkey(), true),
                AccountMeta::new(to_pda, false),
                AccountMeta::new(to_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&from],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_invalid_to_pda_transfer() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    let to = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &from.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(1.0),
    )
    .await
    .unwrap();

    let (from_pda, _) = Pubkey::find_program_address(&[from.pubkey().as_ref()], &program_id);

    let transfer_instruction = simple_token_instructions::Instruction::Transfer {
        to: to.pubkey(),
        amount: sol(5.0),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &transfer_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(from.pubkey(), true),
                AccountMeta::new(from_pda, false),
                AccountMeta::new(from_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&from],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_overflow_transfer() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    let to = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &from.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(u64::MAX as f64),
    )
    .await
    .unwrap();

    transfer_token(&from, &to, &program_id, &mut context.banks_client, sol(5.0))
        .await
        .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(5.0),
    )
    .await
    .unwrap();

    let transaction_result =
        transfer_token(&from, &to, &program_id, &mut context.banks_client, u64::MAX).await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::ArithmeticOverflow,
        ))) => assert_eq!(true, true),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_burn() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await
    .unwrap();

    burn_from(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(1.0),
    )
    .await
    .unwrap();

    let from_account = get_account(&mut context.banks_client, &program_id, &from.pubkey()).await;
    assert_eq!(from_account.balance, sol(9.0));
}

#[tokio::test]
async fn test_fail_not_owner_burn() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    let not_owner = Keypair::new();

    transfer_sol(
        &mut context.banks_client,
        &context.payer,
        &not_owner.pubkey(),
        sol(10.0),
    )
    .await
    .unwrap();

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await
    .unwrap();

    let transaction_result = burn_from(
        &not_owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(1.0),
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidOwner as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_invalid_config_pda_burn() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();

    let (from_pda, _) = Pubkey::find_program_address(&[from.pubkey().as_ref()], &program_id);
    let (not_config, _) = Pubkey::find_program_address(&[b"not_config"], &program_id);

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await
    .unwrap();

    let burn_instruction = simple_token_instructions::Instruction::Burn {
        from: from.pubkey(),
        amount: sol(10.0),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &burn_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(not_config, false),
                AccountMeta::new(from_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_invalid_from_pda_burn() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();

    let (from_pda, _) = Pubkey::find_program_address(&[owner.pubkey().as_ref()], &program_id);
    let (not_config, _) = Pubkey::find_program_address(&[b"config"], &program_id);

    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await
    .unwrap();

    let burn_instruction = simple_token_instructions::Instruction::Burn {
        from: from.pubkey(),
        amount: sol(10.0),
    };

    let transaction_result = process_transaction(
        &mut context.banks_client,
        vec![Instruction::new_with_bytes(
            program_id,
            &burn_instruction.try_to_vec().unwrap(),
            vec![
                AccountMeta::new(owner.pubkey(), true),
                AccountMeta::new(not_config, false),
                AccountMeta::new(from_pda, false),
                AccountMeta::new(system_program::id(), false),
            ],
        )],
        vec![&owner],
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_code),
        ))) => assert_eq!(error_code, SimpleTokenErrors::InvalidPda as u32),
        _ => panic!("Should fail"),
    }
}

#[tokio::test]
async fn test_fail_underflow_burn() {
    let (mut context, program_id, owner) = prepare().await;
    initialize(&owner, &program_id, &mut context.banks_client).await;

    let from = Keypair::new();
    mint_to(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(1.0),
    )
    .await
    .unwrap();

    let transaction_result = burn_from(
        &owner,
        &from,
        &program_id,
        &mut context.banks_client,
        sol(10.0),
    )
    .await;

    match transaction_result {
        Err(BanksClientError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::InsufficientFunds,
        ))) => assert_eq!(true, true),
        _ => panic!("Should fail"),
    }
}
