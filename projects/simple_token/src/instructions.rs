use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;


#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub enum Instruction {
    Mint {
        to: Pubkey,
        amount: u64,
    },
    Transfer {
        to: Pubkey,
        amount: u64,
    },
    Burn {
        from: Pubkey,
        amount: u64,
    },
    ChangeOwner {
        new_owner: Pubkey,
    },
    Initialize {
        owner: Pubkey,
        decimals: u8,
    },
}