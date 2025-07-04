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
    accounts::{
        IDOInitializeIxAccounts, 
        IDOBuyWithVesting
    },
    constants::*,
    instruction::IDOInstruction,
    vesting::LinearVestingStrategy,
    state::{IDOConfigAccount, IDOVestingAccount}
};

use std::cell::Ref;


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
            
            IDOInstruction::BuyWithVesting { amount } => Self::process_buy_with_vesting_instruction(program_id, accounts, amount)?,
            
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

    fn process_buy_with_vesting_instruction(
        program_id: &Pubkey, 
        accounts: &[AccountInfo], 
        amount: u64
    ) -> ProgramResult {
        // 1. Check ownership & vesting account deterministic derivation; Validate that the authority & mint of treasury ATA
        let IDOBuyWithVesting { 
            signer_info, 
            vesting_info, 
            treasury_info, 
            config_info, 
            mint_info, 
            ..
        } = accounts.try_into()?;
        
        let (signer_pkey_bytes, mint_pkey_bytes) = (
            signer_info.key.as_ref(),
            mint_info.key.as_ref()
        );
        let (vesting_pda, vesting_bump) = Pubkey::find_program_address(
            &[
                IDO_VESTING_ACCOUNT_SEED,
                signer_pkey_bytes,
                mint_pkey_bytes        
            ], 
            program_id
        );

        let treasury_ata: Account = Account::unpack(*treasury_info.data.borrow())?;

        if config_info.owner != program_id
        || vesting_pda != *vesting_info.key
        || treasury_ata.mint != *mint_info.key
        || treasury_ata.owner != *treasury_info.key
        {
            return Err(ProgramError::InvalidInstructionData);
        }

        // 3. Get `lamports_per_token` from Config PDA & `calculate lamports_transfer_amount`
        let config: IDOConfigAccount = IDOConfigAccount::unpack(*config_info.data.borrow())?;
        let lamports_transfer_amount: u64 = amount * config.lamports_per_token as u64;
        let rent: Rent = Rent::get()?;

        // 4. Initialize Vesting PDA if needed OR unpack it and update necessary fields.
        let maybe_vesting_account  = {
            // gets dropped at the end of this scope, so we dont have to call `std::mem::drop` manually twice in if else blocks
            let vesting_data_ref: Ref<&mut [u8]> = vesting_info.data.borrow();
            IDOVestingAccount::unpack(*vesting_data_ref)
        };
        
        if let Ok(mut vesting_account) = maybe_vesting_account {        
            let updated_bought_amount: u64 = vesting_account.bought_amount
                .checked_add(amount)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            vesting_account.bought_amount = updated_bought_amount;
            vesting_account.amount_per_unlock = updated_bought_amount / config.unlocks as u64;

            vesting_account.pack_into_slice(*vesting_info.data.borrow_mut()); 
        } else {
            let vesting_rent_exempt: u64 = rent.minimum_balance(IDOVestingAccount::LEN);
            let create_vesting_account_ix: Instruction = system_instruction::create_account(
                signer_info.key, 
                &vesting_pda, 
                vesting_rent_exempt, 
                IDOVestingAccount::LEN as u64, 
                program_id
            );
            invoke_signed(
                &create_vesting_account_ix,
                &[
                    signer_info.clone(),
                    vesting_info.clone()
                ],
                &[&[IDO_VESTING_ACCOUNT_SEED, signer_pkey_bytes, mint_pkey_bytes, &[vesting_bump]]]
            )?;

            let vesting_account: IDOVestingAccount = IDOVestingAccount::new(
                amount, 
                amount / config.unlocks as u64, 
                vesting_bump
            );

            vesting_account.pack_into_slice(*vesting_info.data.borrow_mut());
        };

        // IMPROTANT: this code of block must be located below step 3, because we have to know the updated `signer_info.lamports` balance.
        // 
        // 5. Check if `signer` lamports balance is not lower than the required amount.
        // SystemProgram owned accounts have data len == 0 bytes.
        let signers_balance_without_rent: u64 = signer_info.lamports()
            .checked_sub(rent.minimum_balance(0))
            .ok_or(ProgramError::InsufficientFunds)?;

        if signers_balance_without_rent < lamports_transfer_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // 6. Transfer `lamports_transfer_amount` to `treasury ATA`.
        let transfer_ix: Instruction = system_instruction::transfer(
            signer_info.key, 
            treasury_info.key, 
            lamports_transfer_amount
        );
        invoke(
            &transfer_ix,
            &[
                signer_info.clone(),
                treasury_info.clone()
            ]
        )?;

        Ok(())
    }
}