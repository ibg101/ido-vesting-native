#![allow(unexpected_cfgs)]
use solana_program::{
    pubkey::Pubkey,
    account_info::AccountInfo,
    entrypoint::{
        entrypoint, 
        ProgramResult
    }
};
use super::processor::Processor;


entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8]
) -> ProgramResult {
    Processor::process(program_id, accounts, data)?;

    Ok(())
}