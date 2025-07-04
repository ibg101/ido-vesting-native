use solana_program::program_error::ProgramError;
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