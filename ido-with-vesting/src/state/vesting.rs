use solana_program::{
    program_error::ProgramError,
    program_pack::{
        Pack,
        Sealed,
        IsInitialized
    }
};
use crate::utils::{
    Reader, 
    ReadBytes
};


#[repr(C)]
#[derive(Default)]
pub struct IDOVestingAccount {
    pub last_claim_ts: i64,

    pub claimed_amount: u64, 
    
    pub bought_amount: u64,
    /// this field must be advanced based on the updated bought tokens amount
    pub amount_per_unlock: u64,
    
    pub bump: u8,

    pub is_initialized: bool    
}

impl IDOVestingAccount {
    /// ### Use this builder method instead of Self::default()
    pub fn new(bought_amount: u64, amount_per_unlock: u64, bump: u8) -> Self {
        Self { 
            bought_amount, 
            amount_per_unlock, 
            bump, 
            is_initialized: true,
            ..Self::default() 
        }
    } 
}

impl Sealed for IDOVestingAccount {}

impl IsInitialized for IDOVestingAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for IDOVestingAccount {
    const LEN: usize = 34;

    fn pack_into_slice(&self, dst: &mut [u8]) -> () {
        dst[..8].copy_from_slice(&self.last_claim_ts.to_le_bytes());
        dst[8..16].copy_from_slice(&self.claimed_amount.to_le_bytes());
        dst[16..24].copy_from_slice(&self.bought_amount.to_le_bytes());
        dst[24..32].copy_from_slice(&self.amount_per_unlock.to_le_bytes());
        dst[32] = self.bump;
        dst[33] = self.is_initialized as u8;
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let reader: Reader = src.into();

        Ok(Self {
            last_claim_ts: reader.read_i64(0)?,
            claimed_amount: reader.read_u64(8)?,
            bought_amount: reader.read_u64(16)?,
            amount_per_unlock: reader.read_u64(24)?,
            bump: src[32],
            is_initialized: src[33] != 0 
        })        
    }
}