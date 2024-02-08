use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct Action {
    pub program_id: Pubkey,
    // Pubkey, signer, writable
    pub accounts: Vec<ActionAccount>,
    pub data: Vec<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone)]
pub struct ActionAccount {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl Action {
    pub fn size(&self) -> usize {
        return 8
            + std::mem::size_of::<Pubkey>()
            + 4
            + (self.accounts.len() * (std::mem::size_of::<Pubkey>() + 2))
            + 4
            + self.data.len();
    }
}
