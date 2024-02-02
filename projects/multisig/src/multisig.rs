use std::vec;

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
    pub name: Vec<u8>,
    pub members: Vec<Pubkey>,
    pub threshold: u64,
}

impl Multisig {
    pub fn new(name: Vec<u8>, members: Vec<Pubkey>, threshold: u64) -> Self {
        Multisig {
            name,
            members,
            threshold,
        }
    }
    fn add_member(&mut self, member: Pubkey) -> ProgramResult {
        // if already a member, do nothing
        if self.is_member(&member) {
            return Ok(());
        }
        self.members.push(member);

        Ok(())
    }
    fn remove_member(&mut self, member: Pubkey) -> ProgramResult {
        self.members.retain(|x| *x != member);

        if self.members.len() == 0 {
            return Err(ProgramError::Custom(MultisigError::NoMembers as u32));
        }

        if self.threshold > self.members.len() as u64 {
            return Err(ProgramError::Custom(MultisigError::ThresholdTooHigh as u32));
        }

        Ok(())
    }
    fn set_threshold(&mut self, threshold: u64) -> ProgramResult {
        if threshold > self.members.len() as u64 {
            return Err(ProgramError::Custom(MultisigError::ThresholdTooHigh as u32));
        }
        if threshold == 0 {
            return Err(ProgramError::Custom(MultisigError::ThresholdTooLow as u32));
        }

        self.threshold = threshold;
        Ok(())
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
    fn save<'a>(&self, account: &AccountInfo<'a>, payer: &AccountInfo<'a>) -> ProgramResult {
        storage::resize_pda(account, self.size(), payer)?;
        let mut multisig_data = account.try_borrow_mut_data().unwrap();
        storage::write_to_pda(multisig_data.as_mut(), &self.try_to_vec().unwrap());
        Ok(())
    }
    fn create<'a, 'b>(
        &self,
        program_id: &Pubkey,
        payer: &'a AccountInfo<'b>,
        account: &'a AccountInfo<'b>,
    ) -> ProgramResult {
        let seeds = [b"multisig", program_id.as_ref(), &self.name];
        storage::create_pda(program_id, payer, &seeds, account, self.size())?;
        self.save(account, payer)?;
        Ok(())
    }
    pub fn get(program_id: &Pubkey, account: &AccountInfo) -> Result<Multisig, ProgramError> {
        let multisig_data = account.try_borrow_mut_data()?;
        let multisig = match Multisig::try_from_slice(&multisig_data) {
            Ok(multisig) => Ok(multisig),
            Err(_) => Err(ProgramError::InvalidAccountData),
        }?;
        storage::check_pda(
            program_id,
            &[b"multisig", program_id.as_ref(), &multisig.name],
            account,
        )?;
        Ok(multisig)
    }

    pub fn execute_action(
        &self,
        program_id: &Pubkey,
        action: &Action,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("Executing action {:?}", action);
        let accounts_iter = &mut accounts.iter();
        let mut account_meta: Vec<AccountMeta> = vec![];
        for account in action.accounts.iter() {
            let next_account = next_account_info(accounts_iter)?;
            if *next_account.key != account.0 {
                return Err(ProgramError::InvalidAccountData);
            }
            if account.2 {
                account_meta.push(AccountMeta::new(*next_account.key, account.1))
            } else {
                account_meta.push(AccountMeta::new_readonly(
                    *next_account.key,
                    account.1
                ))
            }
        }

        let seeds = [b"multisig", program_id.as_ref(), &self.name];
        let (_, pda_bump) = Pubkey::find_program_address(&seeds, program_id);
        let mut seeds_vec = seeds.to_vec();
        let pda_dump_slice = &[pda_bump];
        seeds_vec.push(pda_dump_slice);

        msg!("Invoking with accounts {:?}", accounts);
        msg!("Invoking with accounts meta {:?}", account_meta);
        invoke_signed(
            &Instruction::new_with_bytes(action.program_id, &action.data, account_meta),
            accounts,
            &[seeds_vec.as_slice()],
        )
    }
    pub fn size(&self) -> usize {
        // vecs have an additional 4 bytes
        let members_size = self.members.len() * std::mem::size_of::<Pubkey>() + 4;
        let name_size = self.name.len() + 4;

        // members_size + name_size + threshold size
        return members_size + name_size + 8;
    }
}

pub fn create<'a, 'b>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
    name: Vec<u8>,
    members: Vec<Pubkey>,
    threshold: u64,
) -> ProgramResult {
    msg!("Creating multisig");
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;

    let mut multisig = Multisig::new(name, vec![], threshold);
    // use add_member() to deduplicate members array
    for member in members.iter() {
        multisig.add_member(*member)?;
    }

    multisig.create(program_id, payer, multisig_account)?;
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
    multisig.add_member(*new_member)?;
    multisig.save(multisig_account, multisig_account)?;

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
    multisig.remove_member(*member_to_remove)?;
    multisig.save(multisig_account, multisig_account)?;

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
    multisig.set_threshold(new_threshold)?;
    multisig.save(multisig_account, multisig_account)?;

    Ok(())
}

// multisig related errors rang eis 0..99
pub enum MultisigError {
    NotAMember = 0,
    ThresholdTooHigh = 1,
    ThresholdTooLow = 2,
    NoMembers = 3,
}
