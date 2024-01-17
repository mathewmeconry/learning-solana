use std::mem;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::errors::SimpleTokenErrors;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    pub balance: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Config {
    pub owner: Pubkey,
    pub decimals: u8,
}

pub fn remove<'a>(amount: u64, from_pda: &AccountInfo<'a>) -> ProgramResult {
    msg!("Removing {} tokens", amount);
    let mut pda_data = from_pda.try_borrow_mut_data()?;
    let mut account = Account::try_from_slice(&pda_data)?;
    if account.balance < amount {
        msg!("Insufficient funds");
        return Err(ProgramError::InsufficientFunds);
    }

    let new_balance = account.balance.checked_sub(amount);
    match new_balance {
        Some(new_balance) => account.balance = new_balance,
        None => {
            msg!("Underflow");
            return Err(ProgramError::InsufficientFunds);
        }
    }
    wriite_to_pda(pda_data.as_mut(), &account.try_to_vec()?);
    Ok(())
}

pub fn add<'a>(amount: u64, to_pda: &AccountInfo<'a>) -> ProgramResult {
    msg!("Adding {} tokens", amount);
    let mut pda_data = to_pda.try_borrow_mut_data()?;
    let mut account = Account::try_from_slice(&pda_data)?;

    let new_balance = account.balance.checked_add(amount);
    match new_balance {
        Some(new_balance) => account.balance = new_balance,
        None => {
            msg!("Overflow");
            return Err(ProgramError::ArithmeticOverflow);
        }
    }
    wriite_to_pda(pda_data.as_mut(), &account.try_to_vec()?);
    Ok(())
}

pub fn update_owner<'a>(new_owner: Pubkey, config_pda: &AccountInfo<'a>) -> ProgramResult {
    msg!("Changing owner");
    let mut pda_data = config_pda.try_borrow_mut_data()?;
    let mut config = Config::try_from_slice(&pda_data)?;
    config.owner = new_owner;
    wriite_to_pda(pda_data.as_mut(), &config.try_to_vec()?);
    Ok(())
}

pub fn initialize_config<'a>(
    program_id: &Pubkey,
    owner_info: &AccountInfo<'a>,
    owner: &Pubkey,
    decimals: u8,
    config_pda: &AccountInfo<'a>,
) -> ProgramResult {
    msg!("Initializing config");
    create_pda(
        program_id,
        owner_info,
        &[b"config"],
        config_pda,
        mem::size_of::<Config>(),
    )?;
    let mut pda_data = config_pda.try_borrow_mut_data()?;
    let mut config = Config::try_from_slice(&pda_data)?;
    config.owner = *owner;
    config.decimals = decimals;
    wriite_to_pda(pda_data.as_mut(), &config.try_to_vec()?);
    pda_data[..config.try_to_vec()?.len()].copy_from_slice(&config.try_to_vec()?);
    Ok(())
}

pub fn check_config_pda<'a>(program_id: &Pubkey, config_pda: &AccountInfo<'a>) -> ProgramResult {
    verify_pda(program_id, &[b"config"], config_pda)
}

pub fn check_owner(
    owner: &AccountInfo,
    config_pda: &AccountInfo,
    program_id: &Pubkey,
) -> ProgramResult {
    if !owner.is_signer {
        msg!("Invalid owner");
        return Err(ProgramError::Custom(SimpleTokenErrors::InvalidOwner as u32));
    }

    check_config_pda(program_id, config_pda)?;

    let pda_data = config_pda.try_borrow_data()?;
    let account = Config::try_from_slice(&pda_data)?;
    if account.owner != *owner.key {
        msg!("Invalid owner");
        return Err(ProgramError::Custom(SimpleTokenErrors::InvalidOwner as u32));
    }
    msg!("Owner verified");
    Ok(())
}

pub fn verify_user_pda(
    program_id: &Pubkey,
    user: &Pubkey,
    user_pda: &AccountInfo,
) -> ProgramResult {
    return verify_pda(program_id, &[user.as_ref()], user_pda);
}

pub fn verify_pda(program_id: &Pubkey, seeds: &[&[u8]], pda: &AccountInfo) -> ProgramResult {
    let (pda_key, _) = Pubkey::find_program_address(seeds, program_id);
    if pda_key != *pda.key {
        msg!("Accounts don't match");
        return Err(ProgramError::Custom(SimpleTokenErrors::InvalidPda as u32));
    }

    if pda.owner != program_id && *pda.owner != system_program::id() {
        msg!("Owner doesn't match");
        return Err(ProgramError::Custom(SimpleTokenErrors::InvalidPda as u32));
    }

    Ok(())
}

pub fn create_user_pda<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    user_key: &Pubkey,
    user_pda: &AccountInfo<'a>,
) -> ProgramResult {
    return create_pda(
        program_id,
        payer,
        &[user_key.as_ref()],
        user_pda,
        mem::size_of::<Account>(),
    );
}

pub fn create_pda<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    seeds: &[&[u8]],
    pda: &AccountInfo<'a>,
    account_size: usize,
) -> ProgramResult {
    let (pda_key, pda_bump) = Pubkey::find_program_address(seeds, program_id);
    if pda.owner != &solana_program::system_program::id() {
        msg!("Account already existing");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let mut seeds_vec = seeds.to_vec();
    let pda_dump_slice = &[pda_bump];
    seeds_vec.push(pda_dump_slice);
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(account_size);
    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            &pda_key,
            rent_lamports,
            account_size.try_into().unwrap(),
            program_id,
        ),
        &[payer.clone(), pda.clone()],
        &[seeds_vec.as_slice()],
    )
    .unwrap();
    msg!("PDA ({}) created with size: {}", pda_key, account_size);
    return Ok(());
}

fn wriite_to_pda(pda_data: &mut [u8], data: &[u8]) {
    pda_data[0..data.len()].copy_from_slice(data);
}
