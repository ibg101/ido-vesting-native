#[allow(deprecated)]
use solana_program::{
    rent::Rent,
    clock::Clock,
    sysvar::Sysvar,
    pubkey::Pubkey,
    system_instruction,
    instruction::{Instruction, AccountMeta},
    entrypoint::ProgramResult,
    account_info::AccountInfo,
    program_pack::Pack,
    program_error::ProgramError,
    program::{invoke, invoke_signed},
};
use spl_token_2022::state::{Account, Mint};
use super::{
    constants::*,
    error::IDOProgramError,
    utils::derive_program_pda,
    instruction::IDOInstruction,
    contexts::{
        IDOInitializeCtx, 
        IDOBuyWithVestingCtx,
        IDOClaimCtx
    },
    vesting::{
        LinearVestingStrategy,
        allow_claim_and_define_portion
    },
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
            
            IDOInstruction::Claim => Self::process_claim_instruction(accounts)?
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

        let IDOInitializeCtx { 
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

        let (expected_treasury_ata, treasury_bump) = derive_program_pda(&[
            IDO_TREASURY_ACCOUNT_SEED, 
            mint_pkey_bytes
        ]);

        if *treasury_info.key != expected_treasury_ata {
            return Err(ProgramError::InvalidInstructionData);
        }

        let treasury_pkey_bytes: &[u8] = expected_treasury_ata.as_ref();

        let (expected_config_pda, config_bump) = derive_program_pda(&[
            IDO_CONFIG_ACCOUNT_SEED, 
            treasury_pkey_bytes
        ]);

        if *config_info.key != expected_config_pda {
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
        let config_account: IDOConfigAccount = IDOConfigAccount {
            vesting_strategy,
            lamports_per_token,
            unlocks,
            bump: config_bump,
            is_initialized: true
        };
        config_account.pack_into_slice(*config_info.data.borrow_mut());

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
        // 1. Check deterministic derivation; Validate that the authority & mint of treasury ATA
        let IDOBuyWithVestingCtx { 
            signer_info, 
            vesting_info, 
            treasury_info, 
            config_info, 
            mint_info, 
            ..
        } = accounts.try_into()?;
        
        let signer_pkey: &Pubkey = signer_info.key;

        let (signer_pkey_bytes, mint_pkey_bytes) = (
            signer_pkey.as_ref(),
            mint_info.key.as_ref()
        );

        let (expected_vesting_pda, vesting_bump) = derive_program_pda(&[
            IDO_VESTING_ACCOUNT_SEED,
            signer_pkey_bytes,
            mint_pkey_bytes
        ]);

        let (expected_config_pda, _config_bump) = derive_program_pda(&[
            IDO_CONFIG_ACCOUNT_SEED,
            treasury_info.key.as_ref()
        ]);

        let treasury_ata: Account = Account::unpack(*treasury_info.data.borrow())?;

        if expected_vesting_pda != *vesting_info.key
        || expected_config_pda != *config_info.key
        || treasury_ata.mint != *mint_info.key
        || treasury_ata.owner != *treasury_info.key
        {
            return Err(ProgramError::InvalidInstructionData);
        }

        // 3. Get `lamports_per_token` from Config PDA & `calculate lamports_transfer_amount`
        let config_account: IDOConfigAccount = IDOConfigAccount::unpack(*config_info.data.borrow())?;
        let lamports_transfer_amount: u64 = amount * config_account.lamports_per_token as u64;
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
            vesting_account.amount_per_unlock = updated_bought_amount / config_account.unlocks as u64;

            vesting_account.pack_into_slice(*vesting_info.data.borrow_mut()); 
        } else {
            let vesting_rent_exempt: u64 = rent.minimum_balance(IDOVestingAccount::LEN);
            let create_vesting_account_ix: Instruction = system_instruction::create_account(
                signer_pkey, 
                &expected_vesting_pda, 
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
                amount / config_account.unlocks as u64, 
                vesting_bump
            );

            vesting_account.pack_into_slice(*vesting_info.data.borrow_mut());
        };

        // IMPROTANT: this code of block must be located below step 3, because we have to know the updated `signer_info.lamports` balance.
        // 
        // 5. Check if `signer` lamports balance is not smaller than the required amount.
        // SystemProgram owned accounts have data len == 0 bytes.
        let signers_balance_without_rent: u64 = signer_info.lamports()
            .checked_sub(rent.minimum_balance(0))
            .ok_or(ProgramError::InsufficientFunds)?;

        if signers_balance_without_rent < lamports_transfer_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        // 6. Transfer `lamports_transfer_amount` to `treasury ATA`.
        let transfer_ix: Instruction = system_instruction::transfer(
            signer_pkey, 
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

    fn process_claim_instruction(
        accounts: &[AccountInfo]
    ) -> ProgramResult {
        // 1. Check deterministic derivation
        let IDOClaimCtx { 
            signer_info,
            recipient_info, 
            recipient_ata_info, 
            vesting_info, 
            treasury_info, 
            config_info, 
            mint_info, 
            associated_token_program_info,
            token_program_info,
            system_program_info
        } = accounts.try_into()?;

        let signer_pkey: Pubkey = *signer_info.key;

        let mint_pkey_bytes: &[u8] = mint_info.key.as_ref();

        let (expected_vesting_pda, _vesting_bump) = derive_program_pda(&[
            IDO_VESTING_ACCOUNT_SEED,
            signer_pkey.as_ref(),
            mint_pkey_bytes
        ]);

        let (expected_config_pda, _config_bump) = derive_program_pda(&[
            IDO_CONFIG_ACCOUNT_SEED,
            treasury_info.key.as_ref()
        ]);

        // in this case we dont need to unpack treasury ata, but we still have to validate that the correct account is provided.
        let (expected_treasury_pda, treasury_bump) = derive_program_pda(&[
            IDO_TREASURY_ACCOUNT_SEED,
            mint_pkey_bytes
        ]);

        if expected_vesting_pda != *vesting_info.key
        || expected_config_pda != *config_info.key
        || expected_treasury_pda != *treasury_info.key {
            return Err(ProgramError::InvalidInstructionData);
        }

        // 2. Define and Check if the `transfer_amount` can be claimed.        
        let config_account: IDOConfigAccount = IDOConfigAccount::unpack(*config_info.data.borrow())?;
        
        let vesting_account_ref: Ref<&mut [u8]> = vesting_info.data.borrow();
        
        let mut vesting_account: IDOVestingAccount = match IDOVestingAccount::unpack(*vesting_account_ref) {
            Ok(acc) => acc,
            _ => return Err(IDOProgramError::ClaimBeforeBuy.into())
        };

        std::mem::drop(vesting_account_ref);

        let clock: Clock = Clock::get()?;
        let vesting_strategy: LinearVestingStrategy = config_account.vesting_strategy;
        
        let transfer_amount: u64 = allow_claim_and_define_portion(
            &clock, 
            &vesting_strategy, 
            &mut vesting_account
        )?;

        vesting_account.pack_into_slice(*vesting_info.data.borrow_mut());

        // 3. If `recipient_ata` is not initialized => create PDA & initialize account.  
        if recipient_ata_info.owner == system_program_info.key {
            // if spl_associated_token_account crate is added => remove this and use it's instruction builders
            let create_ata_ix: Instruction = Instruction::new_with_bytes(
                *associated_token_program_info.key, 
                &[0],  // AssociatedTokenAccount::Create (it will create PDA & initialize new ATA)
                vec![
                    AccountMeta::new(signer_pkey, true),
                    AccountMeta::new(*recipient_ata_info.key, false),
                    AccountMeta::new_readonly(*recipient_info.key, false),
                    AccountMeta::new_readonly(*mint_info.key, false),
                    AccountMeta::new_readonly(*system_program_info.key, false),
                    AccountMeta::new_readonly(*token_program_info.key, false)
                ]
            );
            invoke(
                &create_ata_ix,
                &[
                    signer_info.clone(),
                    recipient_ata_info.clone(),
                    recipient_info.clone(),
                    mint_info.clone(),
                    system_program_info.clone(),
                    token_program_info.clone()
                ]   
            )?;
        }

        // 4. Transfer `transfer_amount` to `recipient_ata`
        let mint_account: Mint = Mint::unpack(*mint_info.data.borrow())?;

        let transfer_checked_ix: Instruction = spl_token_2022::instruction::transfer_checked(
            token_program_info.key, 
            treasury_info.key, 
            mint_info.key, 
            recipient_ata_info.key, 
            treasury_info.key, 
            &[], 
            transfer_amount, 
            mint_account.decimals
        )?;
        invoke_signed(
            &transfer_checked_ix, 
            &[
                treasury_info.clone(),
                mint_info.clone(),
                recipient_ata_info.clone(),
                treasury_info.clone()
            ], 
            &[&[IDO_TREASURY_ACCOUNT_SEED, mint_pkey_bytes, &[treasury_bump]]]
        )?;

        Ok(())
    }
}