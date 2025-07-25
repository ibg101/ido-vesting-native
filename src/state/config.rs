use solana_program::{
    program_error::ProgramError,
    program_pack::{
        Pack,
        Sealed,
        IsInitialized
    }
};
use crate::{
    utils::reader::*,
    vesting::LinearVestingStrategy
};


#[repr(C)]
pub struct IDOConfigAccount {
    pub vesting_strategy: LinearVestingStrategy,
    /// This field is basically a LAMPORTS/TOKEN ratio.
    ///
    /// Example: 1000 LAMPORTS == 1 SPL TOKEN.
    pub lamports_per_token: u32,
    
    pub bump: u8,

    pub unlocks: u8,  // u8 is fine, because MAX_UNLOCKS: u8

    pub is_initialized: bool
}

impl Sealed for IDOConfigAccount {}

impl IsInitialized for IDOConfigAccount {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for IDOConfigAccount {
    const LEN: usize = 31;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        dst[..24].copy_from_slice(self.vesting_strategy.as_ref());
        dst[24..28].copy_from_slice(&self.lamports_per_token.to_le_bytes());
        dst[28] = self.bump;
        dst[29] = self.unlocks;
        dst[30] = self.is_initialized as u8;
    }

    /// Calling .unwrap() is safe, because LEN is validated in .unpack() or .unpack_unchecked() methods.
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut reader: Reader = Reader::new(src);

        Ok(Self {
            vesting_strategy: reader.read_linear_vesting_strategy(true)?,
            lamports_per_token: reader.read_u32()?,
            bump: reader.read_u8()?,
            unlocks: reader.read_u8()?,
            is_initialized: reader.read_bool()?
        })
    }
}