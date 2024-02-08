use anchor_lang::prelude::*;

#[error_code]
pub enum CustomErrors {
    #[msg("Not a member of this multisig")]
    NotAMember,
    #[msg("Threshold too high")]
    ThresholdTooHigh,
    #[msg("Threshold too low")]
    ThresholdTooLow,
    #[msg("No members")]
    NoMembers,
    #[msg("Already member")]
    AlreadyMember,
    #[msg("Invalid account")]
    InvalidAccount,
    #[msg("Already approved")]
    AlreadyApproved,
    #[msg("Already executed")]
    AlreadyExecuted,
    #[msg("Not enough approvals")]
    NotEnoughApprovals,
}
