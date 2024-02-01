use borsh::{BorshDeserialize, BorshSerialize};
use proposal::Action;
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, pubkey::Pubkey,
};

mod multisig;
mod proposal;
mod storage;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
enum Instruction {
    Create {
        name: u8,
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
    },
    CreateProposal {
        id: u64,
        name: String,
        description: String,
        actions: Vec<Action>,
    },
    ExecuteProposal {
    },
    ChangeThreshold {
        threshold: u64,
    },
}

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
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
        Instruction::ExecuteProposal {  } => {
            proposal::execute(program_id, accounts)
        }
        Instruction::Approve {  } => proposal::approve(program_id, accounts),
    }
}
