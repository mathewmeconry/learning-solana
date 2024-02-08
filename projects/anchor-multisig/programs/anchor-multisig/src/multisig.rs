use std::vec;

use anchor_lang::{
    prelude::*,
    solana_program::{instruction::Instruction, program::invoke_signed},
};

use crate::{action::Action, errors::CustomErrors};

#[account]
#[derive(Default)]
pub struct Multisig {
    pub name: Vec<u8>,
    pub members: Vec<Pubkey>,
    pub threshold: u64,
    pub bump: u8,
}

impl Multisig {
    pub fn is_member(&self, member: &Pubkey) -> bool {
        self.members.contains(member)
    }
    pub fn update_threshold(&mut self, new_threshold: u64) -> Result<()> {
        if new_threshold > self.members.len() as u64 {
            return err!(CustomErrors::ThresholdTooHigh);
        }
        if new_threshold < 1 {
            return err!(CustomErrors::ThresholdTooLow);
        }
        self.threshold = new_threshold;
        Ok(())
    }
    pub fn update_members(&mut self, new_members: Vec<Pubkey>) -> Result<()> {
        if new_members.is_empty() {
            return err!(CustomErrors::NoMembers);
        }
        if new_members.len() < self.threshold as usize {
            return err!(CustomErrors::ThresholdTooHigh);
        }
        self.members = vec![];
        for member in new_members {
            self.add_member(member)?;
        }
        Ok(())
    }
    pub fn add_member(&mut self, member: Pubkey) -> Result<()> {
        if self.is_member(&member) {
            return err!(CustomErrors::AlreadyMember);
        }
        self.members.push(member);
        Ok(())
    }
    pub fn remove_member(&mut self, member: Pubkey) -> Result<()> {
        if !self.is_member(&member) {
            return err!(CustomErrors::NotAMember);
        }
        self.members.retain(|x| *x != member);

        if self.members.len() < self.threshold as usize {
            return err!(CustomErrors::ThresholdTooHigh);
        }
        if self.members.len() == 0 {
            return err!(CustomErrors::NoMembers);
        }
        Ok(())
    }
    pub fn execute(&self, action: &Action, accounts: &[AccountInfo]) -> Result<()> {
        msg!("Executing action {:?}", action);
        let accounts_iter = &mut accounts.iter();
        let mut account_meta: Vec<AccountMeta> = vec![];
        for action_account in action.accounts.iter() {
            let next_account = next_account_info(accounts_iter)?;
            if *next_account.key != action_account.pubkey {
                return err!(CustomErrors::InvalidAccount);
            }
            if action_account.is_writable {
                account_meta.push(AccountMeta::new(
                    *next_account.key,
                    action_account.is_signer,
                ))
            } else {
                account_meta.push(AccountMeta::new_readonly(
                    *next_account.key,
                    action_account.is_signer,
                ))
            }
        }
        let seeds = [b"multisig", self.name.as_slice(), &[self.bump]];
        invoke_signed(
            &Instruction::new_with_bytes(action.program_id, &action.data, account_meta),
            accounts,
            &[seeds.as_slice()],
        )?;
        Ok(())
    }
    pub fn size(&self) -> usize {
        return Multisig::static_size(self.name.len(), self.members.len());
    }
    pub fn static_size(name_len: usize, members_len: usize) -> usize {
        // 8 byte discriminator + 4 byte name length + name length + 4 byte members length + members length * pubkey size + 8 byte threshold + bump
        return 8 + 4 + name_len + 4 + (members_len * std::mem::size_of::<Pubkey>()) + 8 + 1;
    }
}
