use borsh::BorshDeserialize;
use errors::SimpleTokenErrors;
use instructions::Instruction;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};
use storage::{
    add, check_owner, create_user_pda, initialize_config, remove, update_owner, verify_user_pda,
};

use crate::storage::check_config_pda;

pub mod errors;
pub mod instructions;
pub mod storage;

entrypoint!(process_instruction);

pub fn process_instruction(
    programm_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(instruction_data)?;
    msg!("Instruction: {:?}", instruction);
    return match instruction {
        instructions::Instruction::Mint {  to, amount } => {
            self::mint(programm_id, accounts, to, amount)
        }
        instructions::Instruction::Transfer {  to, amount } => {
            self::transfer(programm_id, accounts, to, amount)
        }
        instructions::Instruction::Burn { from, amount } => {
            self::burn(programm_id, accounts, from, amount)
        }

        instructions::Instruction::ChangeOwner { new_owner } => {
            self::change_owner(programm_id, accounts, new_owner)
        }
        instructions::Instruction::Initialize { owner, decimals } => {
            self::initialize(programm_id, accounts, owner, decimals)
        }
    };
}

fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    owner: Pubkey,
    decimals: u8,
) -> ProgramResult {
    msg!("Initialize");
    let accounts_iter = &mut accounts.into_iter();
    let owner_info = next_account_info(accounts_iter).unwrap();
    let config_pda = next_account_info(accounts_iter).unwrap();
    check_config_pda(program_id, config_pda)?;
    initialize_config(program_id, &owner_info, &owner, decimals, config_pda)?;

    Ok(())
}

fn mint<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    to: Pubkey,
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.into_iter();
    let owner = next_account_info(accounts_iter).unwrap();
    let config_pda = next_account_info(accounts_iter).unwrap();
    check_owner(owner, config_pda, program_id)?;

    let to_pda = next_account_info(accounts_iter).unwrap();

    verify_user_pda(program_id, &to, to_pda)?;
    if *to_pda.owner == system_program::id() {
        create_user_pda(program_id, owner, &to, to_pda)?;
    }

    add(amount, to_pda)?;

    Ok(())
}

fn transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    to: Pubkey,
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.into_iter();
    let from = next_account_info(accounts_iter).unwrap();
    let from_pda = next_account_info(accounts_iter).unwrap();
    let to_pda = next_account_info(accounts_iter).unwrap();

    if !from.is_signer {
        return Err(ProgramError::Custom(
            SimpleTokenErrors::InvalidSigner as u32,
        ));
    }

    verify_user_pda(program_id, &from.key, from_pda)?;
    verify_user_pda(program_id, &to, to_pda)?;
    if *from_pda.owner == system_program::id() {
        return Err(ProgramError::InsufficientFunds);
    }
    if *to_pda.owner == system_program::id() {
        create_user_pda(program_id, from, &to, to_pda)?;
    }

    remove(amount, from_pda)?;
    add(amount, to_pda)?;

    Ok(())
}

fn burn(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    from_key: Pubkey,
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.into_iter();
    let owner = next_account_info(accounts_iter).unwrap();
    let config_pda = next_account_info(accounts_iter).unwrap();
    check_owner(owner, config_pda, program_id)?;

    let from_pda = next_account_info(accounts_iter).unwrap();

    verify_user_pda(program_id, &from_key, from_pda)?;

    if *from_pda.owner != system_program::id() {
        remove(amount, from_pda)?;
    }
    Ok(())
}

fn change_owner(program_id: &Pubkey, accounts: &[AccountInfo], new_owner: Pubkey) -> ProgramResult {
    let accounts_iter = &mut accounts.into_iter();
    let owner = next_account_info(accounts_iter).unwrap();
    let config_pda = next_account_info(accounts_iter).unwrap();

    check_owner(owner, config_pda, program_id)?;
    update_owner(new_owner, config_pda)?;

    Ok(())
}
