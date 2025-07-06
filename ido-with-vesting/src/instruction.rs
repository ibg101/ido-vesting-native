use solana_program::{
    program_error::ProgramError,
};
use super::{
    utils::{
        Reader,
        ReadBytes,
        read_u64_slice
    },
    vesting::LinearVestingStrategy
};


pub enum IDOInstruction {
    InitializeWithVesting { 
        amount: u64, 
        lamports_per_token: u32,
        vesting_strategy: LinearVestingStrategy
    },

    BuyWithVesting {
        amount: u64
    },

    Claim
}

impl IDOInstruction {
    pub fn unpack(data: &[u8]) -> Result<Self, ProgramError> {
        let (instr_discriminator, data) = data.split_at(1);
        
        Ok(match instr_discriminator[0] {
            0 => Self::unpack_initialize_with_vesting(data)?,
            1 => Self::unpack_buy_with_vesting(data)?,
            2 => Self::unpack_claim(data)?,
            _ => return Err(ProgramError::InvalidInstructionData)
        })
    }

    fn unpack_initialize_with_vesting(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_expected_payload_len(data.len(), 36)?;
        
        let reader: Reader = data.into();

        Ok(Self::InitializeWithVesting {
            amount: reader.read_u64(0)?,
            lamports_per_token: reader.read_u32(8)?,
            vesting_strategy: reader.read_linear_vesting_strategy(12)?
        })
    }

    fn unpack_buy_with_vesting(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_expected_payload_len(data.len(), 8)?;
        
        Ok(Self::BuyWithVesting { 
            amount: read_u64_slice(data, 0)?
        })
    }

    fn unpack_claim(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_expected_payload_len(data.len(), 0)?;
        
        Ok(Self::Claim)
    }

    /// `expected_len` - ix's payload length without enum variant's discriminator.
    fn check_expected_payload_len(data_len: usize, expected_len: usize) -> Result<(), ProgramError> {
        if data_len != expected_len {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(())
    }
}

#[cfg(feature = "instruction")]
pub use builders::{create_initialize_with_vesting, create_buy_with_vesting, create_claim};

#[cfg(feature = "instruction")]
pub mod builders {
    use solana_program::{
        rent::Rent,
        pubkey::Pubkey,
        sysvar::SysvarId,
        system_program::ID as SYSTEM_PROGRAM_ID,
        instruction::{Instruction, AccountMeta},
    };
    use spl_token_2022::ID as SPL_TOKEN_2022_ID;
    use crate::{
        ID as IDO_PROGRAM_ID,
        external_ids::ATA_PROGRAM_ID,
        vesting::LinearVestingStrategy,
    };

    pub fn create_initialize_with_vesting(
        transfer_amount: u64,
        lamports_per_token: u32,
        vesting_strategy: &LinearVestingStrategy,
        payer_pkey: &Pubkey,
        ata_pda: &Pubkey, 
        treasury_pda: &Pubkey, 
        config_pda: &Pubkey,
        mint_pkey: &Pubkey
    ) -> Instruction {
        let mut init_ix_payload: Vec<u8> = Vec::with_capacity(37);         
        init_ix_payload.push(0);
        init_ix_payload.extend_from_slice(&transfer_amount.to_le_bytes());
        init_ix_payload.extend_from_slice(&lamports_per_token.to_le_bytes());
        init_ix_payload.extend_from_slice(vesting_strategy.as_ref());

        Instruction::new_with_bytes(
            IDO_PROGRAM_ID, 
            &init_ix_payload, 
            vec![
                AccountMeta::new(*payer_pkey, true),
                AccountMeta::new(*ata_pda, false),
                AccountMeta::new(*treasury_pda, false),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new_readonly(*mint_pkey, false),
                AccountMeta::new_readonly(Rent::id(), false),
                AccountMeta::new_readonly(SPL_TOKEN_2022_ID, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
            ]
        )
    }

    pub fn create_buy_with_vesting(
        buy_amount: u64,
        payer_pkey: &Pubkey,
        vesting_account: &Pubkey, 
        treasury_pda: &Pubkey, 
        config_pda: &Pubkey,
        mint_pkey: &Pubkey
    ) -> Instruction {
        let mut buy_ix_payload: Vec<u8> = Vec::with_capacity(9);
        buy_ix_payload.push(1); 
        buy_ix_payload.extend_from_slice(&buy_amount.to_le_bytes());

        Instruction::new_with_bytes(
            IDO_PROGRAM_ID, 
            &buy_ix_payload, 
            vec![
                AccountMeta::new(*payer_pkey, true),
                AccountMeta::new(*vesting_account, false),
                AccountMeta::new(*treasury_pda, false),
                AccountMeta::new_readonly(*config_pda, false),
                AccountMeta::new_readonly(*mint_pkey, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
            ]
        )
    }

    pub fn create_claim(
        payer_pkey: &Pubkey, 
        recipient: &Pubkey,
        recipient_ata: &Pubkey, 
        vesting_account: &Pubkey, 
        treasury_pda: &Pubkey,
        config_pda: &Pubkey,
        mint_pkey: &Pubkey
    ) -> Instruction {
        Instruction::new_with_bytes(
            IDO_PROGRAM_ID, 
            &[2], 
            vec![
                AccountMeta::new(*payer_pkey, true),
                AccountMeta::new_readonly(*recipient, false),
                AccountMeta::new(*recipient_ata, false),
                AccountMeta::new(*vesting_account, false),
                AccountMeta::new(*treasury_pda, false),
                AccountMeta::new_readonly(*config_pda, false),
                AccountMeta::new_readonly(*mint_pkey, false),
                AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
                AccountMeta::new_readonly(SPL_TOKEN_2022_ID, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
            ]
        )
    }
}