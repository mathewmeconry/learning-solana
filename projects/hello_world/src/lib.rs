use solana_program:: {
    account_info::AccountInfo,
    pubkey::Pubkey,
    entrypoint,
    entrypoint::ProgramResult, msg
};

entrypoint!(process_instruction);

fn process_instruction(
    programm_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello, world!");
    msg!("Instruction data: {:?}", instruction_data);

    Ok(())
}
