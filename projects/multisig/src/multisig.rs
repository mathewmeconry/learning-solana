use std::{mem, slice::Iter, vec};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{proposal::Action, storage};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Multisig {
    pub name: u8,
    pub members: Vec<Pubkey>,
    pub threshold: u64,
}

impl Multisig {
    pub fn new(name: u8, members: Vec<Pubkey>, threshold: u64) -> Self {
        Multisig {
            name,
            members,
            threshold,
        }
    }
    fn add_member(&mut self, member: Pubkey) {
        self.members.push(member);
    }
    fn remove_member(&mut self, member: Pubkey) {
        self.members.retain(|x| *x != member);
    }
    fn set_threshold(&mut self, threshold: u64) {
        self.threshold = threshold;
    }
    pub fn is_member(&self, member: &Pubkey) -> bool {
        return self.members.contains(member);
    }
    pub fn check_member(&self, member: &Pubkey) -> ProgramResult {
        if !self.is_member(member) {
            return Err(ProgramError::Custom(MultisigError::NotAMember as u32));
        }
        Ok(())
    }
    fn save(&self, account: &AccountInfo) {
        let mut multisig_data = account.try_borrow_mut_data().unwrap();
        storage::write_to_pda(multisig_data.as_mut(), &self.try_to_vec().unwrap());
    }
    pub fn get(program_id: &Pubkey, account: &AccountInfo) -> Result<Multisig, ProgramError> {
        storage::check_pda(program_id, account)?;
        let multisig_data = account.try_borrow_mut_data()?;
        match Multisig::try_from_slice(&multisig_data) {
            Ok(multisig) => Ok(multisig),
            Err(_) => Err(ProgramError::InvalidAccountData),
        }
    }
}

pub fn create(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: u8,
    members: Vec<Pubkey>,
    threshold: u64,
) -> ProgramResult {
    msg!("Creating multisig");
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;

    if payer.is_signer == false {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let seeds = [b"multisig", program_id.as_ref(), &[name]];
    let multisig_size = mem::size_of::<Multisig>();
    storage::create_pda(program_id, payer, &seeds, multisig_account, multisig_size)?;

    multisig_account.realloc(multisig_size, true)?;
    let multisig = Multisig::new(name, members, threshold);
    multisig.save(multisig_account);
    Ok(())
}

pub fn add_member(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_member: &Pubkey,
) -> ProgramResult {
    msg!("Adding member: {}", new_member.to_string());
    let accounts_iter = &mut accounts.iter();
    let multisig_account = next_account_info(accounts_iter)?;
    if multisig_account.is_signer == false {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut multisig = Multisig::get(program_id, multisig_account)?;
    multisig.add_member(*new_member);
    multisig.save(multisig_account);

    Ok(())
}

pub fn remove_member(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    member_to_remove: &Pubkey,
) -> ProgramResult {
    msg!("Removing member: {}", member_to_remove.to_string());
    let accounts_iter = &mut accounts.iter();
    let multisig_account = next_account_info(accounts_iter)?;
    if multisig_account.is_signer == false {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut multisig = Multisig::get(program_id, multisig_account)?;
    multisig.remove_member(*member_to_remove);
    multisig.save(multisig_account);

    Ok(())
}

pub fn set_threshold(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_threshold: u64,
) -> ProgramResult {
    msg!("Setting threshold: {}", new_threshold);
    let accounts_iter = &mut accounts.iter();
    let multisig_account = next_account_info(accounts_iter)?;
    if multisig_account.is_signer == false {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut multisig = Multisig::get(program_id, multisig_account)?;
    multisig.set_threshold(new_threshold);
    multisig.save(multisig_account);

    Ok(())
}

pub fn execute_action(
    program_id: &Pubkey,
    multisig_account: &AccountInfo,
    action: &Action,
    accounts_iter: &mut Iter<AccountInfo>,
) -> ProgramResult {
    let mut account_infos: Vec<AccountInfo> = vec![];
    let mut account_meta: Vec<AccountMeta> = vec![];
    for account in action.accounts.iter() {
        let next_account = next_account_info(accounts_iter)?;
        if next_account.key != account {
            return Err(ProgramError::InvalidAccountData);
        }
        account_infos.push(next_account.clone());
        account_meta.push(AccountMeta::new(*next_account.key, next_account.is_signer));
    }

    let multisig = Multisig::get(program_id, multisig_account)?;
    let seeds = [b"multisig", program_id.as_ref(), &[multisig.name]];
    let (_, pda_bump) = Pubkey::find_program_address(&seeds, program_id);
    let mut seeds_vec = seeds.to_vec();
    let pda_dump_slice = &[pda_bump];
    seeds_vec.push(pda_dump_slice);

    invoke_signed(
        &Instruction::new_with_bytes(action.program_id, &action.data, account_meta),
        &account_infos,
        &[seeds_vec.as_slice()],
    )
}

// multisig related errors rang eis 0..99
pub enum MultisigError {
    NotAMember = 0,
}
