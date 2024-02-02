use std::mem;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, next_account_infos, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    multisig::{self, Multisig},
    storage,
};

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct Action {
    pub program_id: Pubkey,
    pub accounts: Vec<Pubkey>,
    pub data: Vec<u8>,
}

impl Action {
    fn size(&self) -> usize {
        let program_id_size = mem::size_of::<Pubkey>();
        // vecs have an additional 4 bytes
        let accounts_size = self.accounts.len() * mem::size_of::<Pubkey>() + 4;
        let data_size = self.data.len() + 4;

        return program_id_size + accounts_size + data_size;
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Proposal {
    pub id: u64,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub actions: Vec<Action>,
    pub approvers: Vec<Pubkey>,
    pub executed: bool,
    pub multisig: Pubkey,
}

impl Proposal {
    fn new(
        id: u64,
        name: Vec<u8>,
        description: Vec<u8>,
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
    fn approve(&mut self, multisig: &Multisig, approver: &Pubkey) -> ProgramResult {
        multisig.check_member(approver)?;
        if self.has_approved(*approver) {
            return Err(ProgramError::from(ProposalError::AlreadyApproved as u64));
        }
        self.approvers.push(approver.clone());
        Ok(())
    }
    fn has_reached_threshold(&self, multisig: &Multisig) -> bool {
        self.approvers.len() >= multisig.threshold as usize
    }
    fn has_approved(&self, approver: Pubkey) -> bool {
        self.approvers.contains(&approver)
    }
    fn save(&self, account: &AccountInfo) -> ProgramResult {
        let mut proposal_data = account.try_borrow_mut_data().unwrap();
        storage::write_to_pda(proposal_data.as_mut(), &self.try_to_vec().unwrap());
        Ok(())
    }
    fn get(program_id: &Pubkey, account: &AccountInfo) -> Result<Proposal, ProgramError> {
        let proposal_data = account.try_borrow_mut_data()?;
        let proposal = match Proposal::try_from_slice(&proposal_data) {
            Ok(proposal) => Ok(proposal),
            Err(_) => Err(ProgramError::InvalidAccountData),
        }
        .unwrap();
        let seeds = [
            b"proposal",
            program_id.as_ref(),
            &proposal.multisig.as_ref(),
            &proposal.id.to_be_bytes(),
        ];
        storage::check_pda(program_id, &seeds, account)?;
        Ok(proposal)
    }
    fn create<'a, 'b>(
        &self,
        program_id: &Pubkey,
        payer: &'a AccountInfo<'b>,
        account: &'a AccountInfo<'b>,
    ) -> ProgramResult {
        let seeds = [
            b"proposal",
            program_id.as_ref(),
            self.multisig.as_ref(),
            &self.id.to_be_bytes(),
        ];
        storage::create_pda(program_id, payer, &seeds, account, self.size())?;
        account.realloc(self.size(), true)?;
        self.save(account)?;
        Ok(())
    }
    fn size(&self) -> usize {
        // vecs have an additional 4 bytes
        let name_size = self.name.len() + 4;
        let description_size = self.description.len() + 4;
        let approvers_size = self.approvers.len() * mem::size_of::<Pubkey>() + 4;

        let mut actions_size = 4;
        for action in self.actions.iter() {
            actions_size += action.size();
        }

        // id + name + description + actions + approvers + executed + multisig
        return 8
            + name_size
            + description_size
            + actions_size
            + approvers_size
            + 1
            + mem::size_of::<Pubkey>();
    }
}

pub fn create<'a, 'b>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
    id: u64,
    name: Vec<u8>,
    description: Vec<u8>,
    actions: Vec<Action>,
) -> ProgramResult {
    msg!("Creating proposal {} with {} actions", id, actions.len());
    let accounts_iter = &mut accounts.iter();
    let member = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    if !member.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let multisig = Multisig::get(program_id, multisig_account)?;
    multisig.check_member(member.key)?;

    let proposal = Proposal::new(id, name, description, actions, *multisig_account.key);
    proposal.create(program_id, member, proposal_account)?;

    Ok(())
}

pub fn approve(program_id: &Pubkey, accounts: &[AccountInfo], try_execute: bool) -> ProgramResult {
    msg!("Approving proposal");
    let accounts_iter = &mut accounts.iter();
    let member = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;
    let _system_program_account = next_account_info(accounts_iter)?;

    if !member.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let multisig = Multisig::get(program_id, multisig_account)?;
    let mut proposal = Proposal::get(program_id, proposal_account)?;


    proposal.approve(&multisig, member.key)?;
    storage::resize_pda(proposal_account, proposal.size(), member)?;
    proposal.save(proposal_account)?;

    if try_execute && proposal.has_reached_threshold(&multisig) {
        execute(
            program_id,
            next_account_infos(accounts_iter, accounts.len() - 4)?,
        )?;
    }

    Ok(())
}

pub fn execute(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Executing proposal");
    let accounts_iter = &mut accounts.iter();
    let _signer = next_account_info(accounts_iter)?;
    let multisig_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let mut proposal = Proposal::get(program_id, proposal_account)?;


    let multisig = Multisig::get(program_id, multisig_account)?;
    if !proposal.has_reached_threshold(&multisig) {
        return Err(ProgramError::from(ProposalError::NotEnoughApprovals as u64));
    }

    if proposal.executed {
        return Err(ProgramError::Custom(ProposalError::AlreadyExecuted as u32));
    }

    proposal.executed = true;
    proposal.save(proposal_account)?;

    for action in proposal.actions.iter() {
        multisig::execute_action(
            program_id,
            multisig_account,
            action,
            next_account_infos(accounts_iter, action.accounts.len())?,
        )?;
    }

    Ok(())
}

// Proposal errors range is 200...299
pub enum ProposalError {
    AlreadyApproved = 200,
    AlreadyExecuted = 201,
    NotEnoughApprovals = 203,
}
