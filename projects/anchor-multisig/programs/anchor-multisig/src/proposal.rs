use anchor_lang::prelude::*;

use crate::{action::Action, errors::CustomErrors, multisig::Multisig};

#[account]
#[derive(Default)]
pub struct Proposal {
    pub id: u64,
    pub actions: Vec<Action>,
    pub executed: bool,
    pub approvers: Vec<Pubkey>,
    pub bump: u8
}

impl Proposal {
    pub fn default() -> Proposal {
        return Proposal {
            id: 0,
            actions: Vec::new(),
            executed: false,
            approvers: Vec::new(),
            bump: 0
        };
    }

    pub fn approve(&mut self, signer: Pubkey) -> Result<()> {
        if self.approvers.contains(&signer) {
            return err!(CustomErrors::AlreadyApproved);
        }
        self.approvers.push(signer);
        Ok(())
    }

    pub fn check_executed(&self) -> Result<()> {
        if self.executed {
            return err!(CustomErrors::AlreadyExecuted);
        }
        Ok(())
    }

    pub fn check_threshold(&self, multisig: &Multisig) -> Result<()> {
        if self.approvers.len() < multisig.threshold as usize {
            return err!(CustomErrors::NotEnoughApprovals);
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        return Proposal::static_size(&self.actions, self.approvers.len());
    }

    pub fn static_size(actions: &Vec<Action>, approvers_len: usize) -> usize {
        let mut actions_size = 4;
        for action in actions {
            actions_size += action.size();
        }

        // 8 byte discriminator + 8 bytes id + actions + 1 byte executed + approvers + bump
        return 8 + 8 + actions_size + 1 + 4 + (approvers_len * std::mem::size_of::<Pubkey>()) + 1;
    }
}
