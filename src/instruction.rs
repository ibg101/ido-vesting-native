use solana_program::program_error::ProgramError;
use super::vesting::LinearVestingStrategy;


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

        Ok(Self::InitializeWithVesting { 
            amount: u64::from_le_bytes(data[..8].try_into().map_err(|_| ProgramError::InvalidInstructionData)?), 
            lamports_per_token: u32::from_le_bytes(data[8..12].try_into().map_err(|_| ProgramError::InvalidInstructionData)?), 
            vesting_strategy: data[12..36].try_into().map_err(|_| ProgramError::InvalidInstructionData)? 
        })
    }

    fn unpack_buy_with_vesting(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_expected_payload_len(data.len(), 8)?;
        
        todo!()
    }

    fn unpack_claim(data: &[u8]) -> Result<Self, ProgramError> {
        Self::check_expected_payload_len(data.len(), 0)?;
        
        todo!()
    }

    /// `expected_len` - ix's payload length without enum variant's discriminator.
    fn check_expected_payload_len(data_len: usize, expected_len: usize) -> Result<(), ProgramError> {
        if data_len != expected_len {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(())
    }
}