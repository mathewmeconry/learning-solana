use borsh::{BorshDeserialize, BorshSerialize};
use proposal::Action;
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

pub mod multisig;
pub mod proposal;
mod storage;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum Instruction {
    Create {
        name: Vec<u8>,
        members: Vec<Pubkey>,
        threshold: u64,
    },
    AddMember {
        member: Pubkey,
    },
    RemoveMember {
        member: Pubkey,
    },
    Approve {
        try_execute: bool,
    },
    CreateProposal {
        id: u64,
        name: Vec<u8>,
        description: Vec<u8>,
        actions: Vec<Action>,
    },
    ExecuteProposal {},
    ChangeThreshold {
        threshold: u64,
    },
}

entrypoint!(process_instruction);

pub fn process_instruction<'a, 'b, 'c, 'd>(
    program_id: &'a Pubkey,
    accounts: &'b [AccountInfo<'c>],
    instruction_data: &'d [u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(instruction_data)?;
    match instruction {
        Instruction::Create {
            name,
            members,
            threshold,
        } => multisig::create(program_id, accounts, name, members, threshold),
        Instruction::AddMember { member } => multisig::add_member(program_id, accounts, &member),
        Instruction::RemoveMember { member } => {
            multisig::remove_member(program_id, accounts, &member)
        }
        Instruction::ChangeThreshold { threshold } => {
            multisig::set_threshold(program_id, accounts, threshold)
        }
        Instruction::CreateProposal {
            id,
            name,
            description,
            actions,
        } => proposal::create(program_id, accounts, id, name, description, actions),
        Instruction::ExecuteProposal {} => proposal::execute(program_id, accounts),
        Instruction::Approve { try_execute } => {
            proposal::approve(program_id, accounts, try_execute)
        }
    }
}
