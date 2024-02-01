use std::mem;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    multisig::{self, Multisig},
    storage,
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Action {
    pub program_id: Pubkey,
    pub accounts: Vec<Pubkey>,
    pub data: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Proposal {
    id: u64,
    name: String,
    description: String,
    actions: Vec<Action>,
    approvers: Vec<Pubkey>,
    executed: bool,
    multisig: Pubkey,
}

impl Proposal {
    fn new(
        id: u64,
        name: String,
        description: String,
        actions: Vec<Action>,
        multisig: Pubkey,
    ) -> Self {
        Proposal {
            id,
            name,
            description,
            actions,
            approvers: vec![],
            executed: false,
            multisig,
        }
    }
    fn approve(&mut self, multisig: Multisig, approver: &Pubkey) -> ProgramResult {
        multisig.check_member(approver)?;
        if self.has_approved(*approver) {
            return Err(ProgramError::from(ProposalError::AlreadyApproved as u64));
        }
        self.approvers.push(approver.clone());
        Ok(())
    }
    fn has_reached_threshold(&self, multisig: Multisig) -> bool {
        self.approvers.len() >= multisig.threshold as usize
    }
    fn has_approved(&self, approver: Pubkey) -> bool {
        self.approvers.contains(&approver)
    }
    fn save(&self, account: &AccountInfo) {
        let mut proposal_data = account.try_borrow_mut_data().unwrap();
        storage::write_to_pda(proposal_data.as_mut(), &self.try_to_vec().unwrap());
    }
    fn get(program_id: &Pubkey, account: &AccountInfo) -> Result<Proposal, ProgramError> {
        storage::check_pda(program_id, account)?;
        let proposal_data = account.try_borrow_mut_data()?;
        match Proposal::try_from_slice(&proposal_data) {
            Ok(proposal) => Ok(proposal),
            Err(_) => Err(ProgramError::InvalidAccountData),
        }
    }
}

pub fn create(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    id: u64,
    name: String,
    description: String,
    actions: Vec<Action>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let member = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    if !member.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    storage::check_pda(program_id, multisig_account)?;

    let proposal_size = mem::size_of::<Proposal>();
    let seeds = [
        b"proposal",
        program_id.as_ref(),
        multisig_account.key.as_ref(),
        &id.to_be_bytes(),
    ];
    storage::verify_pda(program_id, &seeds, proposal_account)?;
    storage::create_pda(program_id, member, &seeds, proposal_account, proposal_size)?;
    proposal_account.realloc(proposal_size, true)?;
    let proposal = Proposal::new(id, name, description, actions, *multisig_account.key);
    proposal.save(proposal_account);

    Ok(())
}

pub fn approve(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let member = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    if !member.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let multisig = Multisig::get(program_id, multisig_account)?;
    let mut proposal = Proposal::get(program_id, proposal_account)?;
    if proposal.multisig != *multisig_account.key {
        return Err(ProgramError::from(
            ProposalError::WrongMultisigAccount as u64,
        ));
    }

    proposal.approve(multisig, member.key)?;
    proposal.save(proposal_account);

    Ok(())
}

pub fn execute(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let multisig_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let mut proposal = Proposal::get(program_id, proposal_account)?;

    if proposal.multisig != *multisig_account.key {
        return Err(ProgramError::Custom(
            ProposalError::WrongMultisigAccount as u32,
        ));
    }

    let multisig = Multisig::get(program_id, multisig_account)?;
    if !proposal.has_reached_threshold(multisig) {
        return Err(ProgramError::from(ProposalError::NotEnoughApprovals as u64));
    }

    if proposal.executed {
        return Err(ProgramError::Custom(ProposalError::AlreadyExecuted as u32));
    }

    proposal.executed = true;
    proposal.save(proposal_account);

    for action in proposal.actions.iter() {
        multisig::execute_action(program_id, multisig_account, action, accounts_iter)?;
    }

    Ok(())
}

// Proposal errors range is 200...299
pub enum ProposalError {
    AlreadyApproved = 200,
    WrongMultisigAccount = 201,
    AlreadyExecuted = 202,
    NotEnoughApprovals = 204,
}
