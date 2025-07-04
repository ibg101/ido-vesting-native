#[allow(deprecated)]
use solana_program::{
    rent::Rent,
    clock::Clock,
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
use spl_token_2022::state::{Account, Mint};
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
        // 1. Validate the provided Vesting Strategy.
        let clock: Clock = Clock::get()?;
        let vesting_strategy: LinearVestingStrategy = vesting_strategy.reinit_with_checked_cliff(&clock);
        vesting_strategy.is_valid(&clock)?;

        let IDOInitializeIxAccounts { 
            signer_info, 
            signer_ata_info, 
            treasury_info,
            config_info,
            mint_info,
            rent_info,
            token_program_info,
            ..
        } = accounts.try_into()?;

        let signer_pkey: &Pubkey = signer_info.key;

        // 2. Check that the provided accounts are deterministic PDA
        let mint_pkey_bytes: &[u8] = mint_info.key.as_ref();

        let (treasury_ata, treasury_bump) = Pubkey::find_program_address(&[
            IDO_TREASURY_ACCOUNT_SEED, 
            mint_pkey_bytes
        ], program_id);

        if *treasury_info.key != treasury_ata {
            return Err(ProgramError::InvalidInstructionData);
        }

        let treasury_pkey_bytes: &[u8] = treasury_ata.as_ref();

        let (config_pda, config_bump) = Pubkey::find_program_address(&[
            IDO_CONFIG_ACCOUNT_SEED, 
            treasury_pkey_bytes
        ], program_id);

        if *config_info.key != config_pda {
            return Err(ProgramError::InvalidInstructionData);
        }

        let mint: Mint = Mint::unpack(*mint_info.data.borrow())?;

        // 3. Create accounts with SystemProgram
        let rent_sysvar: Rent = Rent::from_account_info(rent_info)?;

        let treasury_rent_exempt: u64 = rent_sysvar.minimum_balance(Account::LEN);
        let create_treasury_ix: Instruction = system_instruction::create_account(
            signer_pkey, 
            treasury_info.key, 
            treasury_rent_exempt, 
            Account::LEN as u64, 
            token_program_info.key
        );

        invoke_signed(
            &create_treasury_ix,
            &[
                signer_info.clone(),
                treasury_info.clone(),
            ],
            &[&[IDO_TREASURY_ACCOUNT_SEED, mint_pkey_bytes, &[treasury_bump]]]
        )?;
        
        let config_rent_exempt: u64 = rent_sysvar.minimum_balance(IDOConfigAccount::LEN);
        let create_config_ix: Instruction = system_instruction::create_account(
            signer_pkey, 
            config_info.key, 
            config_rent_exempt, 
            IDOConfigAccount::LEN as u64, 
            program_id
        );
        invoke_signed(
            &create_config_ix,
            &[
                signer_info.clone(),
                config_info.clone(),
            ],
            &[&[IDO_CONFIG_ACCOUNT_SEED, treasury_pkey_bytes, &[config_bump]]]
        )?;

        // 4. Initialize Treasury Token Account wtih SPL token program
        let initialize_treasury_ix: Instruction = spl_token_2022::instruction::initialize_account(
            token_program_info.key, 
            treasury_info.key, 
            mint_info.key, 
            treasury_info.key
        )?;
        invoke(
            &initialize_treasury_ix,
            &[
                treasury_info.clone(),
                mint_info.clone(),
                treasury_info.clone(),
                rent_info.clone()
            ]
        )?;

        // 5. Initialize Config PDA Account
        let unlocks: u8 = ((vesting_strategy.vesting_end_ts - vesting_strategy.cliff_end_ts) / vesting_strategy.unlock_period) as u8; 
        let ido_config_account: IDOConfigAccount = IDOConfigAccount {
            vesting_strategy,
            lamports_per_token,
            unlocks,
            bump: config_bump,
            is_initialized: true
        };
        ido_config_account.pack_into_slice(*config_info.data.borrow_mut());

        // 6. Transfer provided supply from `signer_ata` to `treasury`
        let transfer_checked_ix: Instruction = spl_token_2022::instruction::transfer_checked(
            token_program_info.key, 
            signer_ata_info.key, 
            mint_info.key, 
            treasury_info.key, 
            signer_pkey, 
            &[], 
            amount, 
            mint.decimals
        )?;
        invoke(
            &transfer_checked_ix,
            &[
                signer_ata_info.clone(),
                mint_info.clone(),
                treasury_info.clone(),
                signer_info.clone()
            ]
        )?;

        Ok(())
    }
}