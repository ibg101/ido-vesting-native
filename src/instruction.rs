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
        
        match instr_discriminator[0] {
            0 => todo!(),
            1 => todo!(),
            2 => todo!(),
            _ => return Err(ProgramError::InvalidInstructionData)
        }
    }
}