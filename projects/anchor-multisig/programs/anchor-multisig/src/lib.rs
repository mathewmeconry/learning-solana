use action::Action;
use anchor_lang::prelude::*;
use errors::CustomErrors;
use multisig::Multisig;
use proposal::Proposal;

declare_id!("5Lvr9CwXgUHXrNnwBnGzENSYbxvhgjVT4kF8bKgnhQxv");

pub mod action;
pub mod errors;
pub mod multisig;
pub mod proposal;

#[program]
pub mod anchor_multisig {
    use std::borrow::{Borrow, BorrowMut};

    use anchor_lang::solana_program::{
        account_info::next_account_infos, entrypoint::ProgramResult,
    };

    use super::*;

    pub fn create(
        ctx: Context<Create>,
        name: Vec<u8>,
        members: Vec<Pubkey>,
        threshold: u64,
    ) -> Result<()> {
        let multisig = ctx.accounts.multisig.borrow_mut();
        multisig.name = name;
        multisig.update_members(members)?;
        multisig.update_threshold(threshold)?;
        multisig.bump = ctx.bumps.multisig;
        Ok(())
    }

    pub fn add_member(ctx: Context<AddMember>, member_to_add: Pubkey) -> ProgramResult {
        ctx.accounts.multisig.add_member(member_to_add)?;
        Ok(())
    }

    pub fn remove_member(ctx: Context<RemoveMember>, member_to_remove: Pubkey) -> ProgramResult {
        ctx.accounts.multisig.remove_member(member_to_remove)?;
        Ok(())
    }

    pub fn update_threshold(ctx: Context<UpdateThreshold>, new_threshold: u64) -> ProgramResult {
        ctx.accounts.multisig.update_threshold(new_threshold)?;
        Ok(())
    }

    pub fn create_proposal(
        ctx: Context<CreateProposal>,
        id: u64,
        actions: Vec<Action>,
    ) -> ProgramResult {
        let proposal = ctx.accounts.proposal.borrow_mut();
        proposal.id = id;
        proposal.actions = actions;
        proposal.bump = ctx.bumps.proposal;
        Ok(())
    }

    pub fn approve_proposal(ctx: Context<ApproveProposal>) -> ProgramResult {
        let proposal = ctx.accounts.proposal.borrow_mut();
        let signer_key = ctx.accounts.signer.key();

        proposal.approve(signer_key)?;
        Ok(())
    }

    pub fn execute_proposal(ctx: Context<ExecuteProposal>) -> ProgramResult {
        let multisig = ctx.accounts.multisig.borrow();
        let proposal = ctx.accounts.proposal.borrow_mut();

        proposal.check_executed()?;
        proposal.check_threshold(&multisig)?;
        proposal.executed = true;

        let accounts_iter = &mut ctx.remaining_accounts.iter();
        for action in proposal.actions.iter() {
            multisig.execute(
                action,
                next_account_infos(accounts_iter, action.accounts.len())?,
            )?;
        }

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(name: Vec<u8>, members: Vec<Pubkey>)]
pub struct Create<'info> {
    #[account(mut)]
    /// CHECK: only used to pay for the PDA
    pub payer: UncheckedAccount<'info>,
    #[account(init, seeds = [b"multisig", name.as_slice()], bump, payer = payer, space = Multisig::static_size(name.len(), members.len()))]
    pub multisig: Account<'info, Multisig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AddMember<'info> {
    // adds the new member to the size
    #[account(mut, signer, realloc = multisig.size() + std::mem::size_of::<Pubkey>(), realloc::payer = multisig, realloc::zero = false)]
    pub multisig: Account<'info, Multisig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveMember<'info> {
    // removes the previous member from the size
    #[account(mut, signer, realloc = multisig.size() - std::mem::size_of::<Pubkey>(), realloc::payer = multisig, realloc::zero = false)]
    pub multisig: Account<'info, Multisig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateThreshold<'info> {
    #[account(mut, signer)]
    pub multisig: Account<'info, Multisig>,
}

#[derive(Accounts)]
#[instruction(id: u64, actions: Vec<Action>)]
pub struct CreateProposal<'info> {
    #[account(signer, mut)]
    pub signer: Signer<'info>,
    #[account(constraint = multisig.is_member(&signer.key) @ CustomErrors::NotAMember)]
    pub multisig: Account<'info, Multisig>,
    #[account(init, seeds = [b"proposal", multisig.key().as_ref(), id.to_le_bytes().as_ref()], bump, payer = signer, space = Proposal::static_size(&actions, 0))]
    pub proposal: Account<'info, Proposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveProposal<'info> {
    #[account(signer, mut)]
    pub signer: Signer<'info>,
    #[account(constraint = multisig.is_member(&signer.key) @ CustomErrors::NotAMember)]
    pub multisig: Account<'info, Multisig>,
    // adds the future approver to the size
    #[account(mut, seeds = [b"proposal", multisig.key().as_ref(), proposal.id.to_le_bytes().as_ref()], bump = proposal.bump, realloc = proposal.size() + std::mem::size_of::<Pubkey>(), realloc::payer = signer, realloc::zero = false)]
    pub proposal: Account<'info, Proposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ExecuteProposal<'info> {
    #[account()]
    pub multisig: Account<'info, Multisig>,
    #[account(mut, seeds = [b"proposal", multisig.key().as_ref(), proposal.id.to_le_bytes().as_ref()], bump = proposal.bump)]
    pub proposal: Account<'info, Proposal>,
}
