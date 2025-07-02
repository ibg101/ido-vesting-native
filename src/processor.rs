#[allow(deprecated)]
use solana_program::{
    rent::Rent,
    sysvar::Sysvar,
    pubkey::Pubkey,
    system_instruction,
    instruction::Instruction,
    entrypoint::ProgramResult,
    account_info::AccountInfo,
    program_pack::Pack,
    program_error::ProgramError,
    program::{invoke, invoke_signed},
};
use spl_token_2022::state::Account;
use super::{
    accounts::IDOInitializeIxAccounts,
    constants::*,
    instruction::IDOInstruction,
    vesting::LinearVestingStrategy,
    state::IDOConfigAccount
};


pub struct Processor;

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8]
    ) -> ProgramResult {
        let ix: IDOInstruction = IDOInstruction::unpack(data)?;

        match ix {
            IDOInstruction::InitializeWithVesting { 
                amount, 
                lamports_per_token, 
                vesting_strategy 
            } => Self::process_initialize_ido_with_vesting_instruction(program_id, accounts, amount, lamports_per_token, vesting_strategy)?,
            
            IDOInstruction::BuyWithVesting { amount } => todo!(),
            
            IDOInstruction::Claim => todo!()
        };

        Ok(())
    }

    fn process_initialize_ido_with_vesting_instruction(
        program_id: &Pubkey, 
        accounts: &[AccountInfo],
        amount: u64,
        lamports_per_token: u32,
        vesting_strategy: LinearVestingStrategy 
    ) -> ProgramResult {
        let IDOInitializeIxAccounts { 
            signer_info, 
            signer_ata_info, 
            treasury_info,
            config_info,
            mint_info,
            token_program_info,
            system_program_info
        } = accounts.try_into()?;

        let signer_pkey: &Pubkey = signer_info.key;

        // 1. Check that the provided accounts are deterministic PDA
        if *treasury_info.key != Pubkey::find_program_address(&[
            IDO_TREASURY_ACCOUNT_SEED, 
            mint_info.key.as_ref()
        ], token_program_info.key).0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        if *config_info.key != Pubkey::find_program_address(&[
            IDO_CONFIG_ACCOUNT_SEED, 
            treasury_info.key.as_ref()
        ], program_id).0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        // 2. Create & Initialize accounts with SystemProgram
        let treasury_rent_exempt: u64 = Rent::get()?.minimum_balance(Account::LEN);
        let create_treasury_ix: Instruction = system_instruction::create_account(
            signer_pkey, 
            treasury_info.key, 
            treasury_rent_exempt + amount, 
            Account::LEN as u64, 
            token_program_info.key
        );

        invoke(
            &create_treasury_ix,
            &[
                signer_info.clone(),
                treasury_info.clone(),
                system_program_info.clone()
            ]
        )?;
        
        let config_rent_exempt: u64 = Rent::get()?.minimum_balance(IDOConfigAccount::LEN);
        let create_config_ix: Instruction = system_instruction::create_account(
            signer_pkey, 
            config_info.key, 
            config_rent_exempt, 
            IDOConfigAccount::LEN as u64, 
            program_id
        );

        invoke(
            &create_config_ix,
            &[
                signer_info.clone(),
                config_info.clone(),
                system_program_info.clone()
            ]
        )?;

        // 3. Initialize Treasury Token Account wtih SPL token program

        // todo..

        Ok(())
    }
}