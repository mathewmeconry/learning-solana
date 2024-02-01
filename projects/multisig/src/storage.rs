use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed, program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, system_program, sysvar::Sysvar
};

pub fn verify_pda(program_id: &Pubkey, seeds: &[&[u8]], pda: &AccountInfo) -> ProgramResult {
    let (pda_key, _) = Pubkey::find_program_address(seeds, program_id);
    if pda_key != *pda.key {
        msg!("Accounts don't match");
        return Err(ProgramError::Custom(StorageError::InvalidPda as u32));
    }

    if pda.owner != program_id && *pda.owner != system_program::id() {
        msg!("Owner doesn't match");
        return Err(ProgramError::Custom(StorageError::InvalidPda as u32));
    }

    Ok(())
}

pub fn check_pda(program_id: &Pubkey, pda: &AccountInfo) -> ProgramResult {
    if pda.owner != program_id {
        return Err(ProgramError::Custom(StorageError::InvalidPda as u32));
    }

    if pda.data_is_empty() {
        return Err(ProgramError::Custom(StorageError::InvalidPda as u32));
    }

    Ok(())
}

pub fn create_pda<'a>(
    program_id: &Pubkey,
    payer: &AccountInfo<'a>,
    seeds: &[&[u8]],
    pda: &AccountInfo<'a>,
    account_size: usize,
) -> ProgramResult {
    let (pda_key, pda_bump) = Pubkey::find_program_address(seeds, program_id);
    if pda.owner != &solana_program::system_program::id() {
        msg!("Account already existing");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let mut seeds_vec = seeds.to_vec();
    let pda_dump_slice = &[pda_bump];
    seeds_vec.push(pda_dump_slice);
    let rent = Rent::get().unwrap();
    let rent_lamports = rent.minimum_balance(account_size);

    if pda.lamports() > 0 {
        if rent_lamports > pda.lamports() {
            let missing_rent = rent_lamports - pda.lamports();
            invoke_signed(
                &system_instruction::transfer(payer.key, &pda_key, missing_rent),
                &[payer.clone(), pda.clone()],
                &[seeds_vec.as_slice()],
            )
            .unwrap();
        }
        invoke_signed(
            &system_instruction::assign(&pda.key, program_id),
            &[pda.clone()],
            &[seeds_vec.as_slice()],
        )
        .unwrap();
        return Ok(());
    }

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            &pda_key,
            rent_lamports,
            account_size.try_into().unwrap(),
            program_id,
        ),
        &[payer.clone(), pda.clone()],
        &[seeds_vec.as_slice()],
    )
    .unwrap();
    msg!("PDA ({}) created with size: {}", pda_key, account_size);
    return Ok(());
}

pub fn write_to_pda(pda_data: &mut [u8], data: &[u8]) {
    pda_data[0..data.len()].copy_from_slice(data);
}

// storage related errors range is 100...199
pub enum StorageError {
    InvalidPda = 100,
}
