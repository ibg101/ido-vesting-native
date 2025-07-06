use solana_program::{
    pubkey::Pubkey,
    program_error::ProgramError
};
use super::vesting::LinearVestingStrategy;


pub trait ReadBytes {
    type Error;

    /// # Safety
    /// The caller is responsible for ensuring that the range `[start..start + 8]`
    /// is within bounds of the `data` slice.
    fn read_u64(&self, start: usize) -> Result<u64, Self::Error>;

    /// # Safety
    /// The caller is responsible for ensuring that the range `[start..start + 8]`
    /// is within bounds of the `data` slice.
    fn read_i64(&self, start: usize) -> Result<i64, Self::Error>;

    /// # Safety
    /// The caller is responsible for ensuring that the range `[start..start + 4]`
    /// is within bounds of the `data` slice.
    fn read_u32(&self, start: usize) -> Result<u32, Self::Error>;

    /// # Safety
    /// The caller is responsible for ensuring that the range `[start..start + 24]`
    /// is within bounds of the `data` slice.
    fn read_linear_vesting_strategy(&self, start: usize) -> Result<LinearVestingStrategy, Self::Error>; 
}

pub struct Reader<'a> {
    pub bytes: &'a [u8]
}

impl<'a> From<&'a [u8]> for Reader<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl ReadBytes for Reader<'_> {
    type Error = ProgramError;

    fn read_u64(&self, start: usize) -> Result<u64, Self::Error> {
        read_u64_slice(self.bytes, start)
    }

    fn read_i64(&self, start: usize) -> Result<i64, Self::Error> {
        read_i64_slice(self.bytes, start)
    }

    fn read_u32(&self, start: usize) -> Result<u32, Self::Error> {
        read_u32_slice(self.bytes, start)
    }

    fn read_linear_vesting_strategy(&self, start: usize) -> Result<LinearVestingStrategy, Self::Error> {
        read_linear_vesting_strategy_slice(self.bytes, start)
    }
}

/// # Safety
/// The caller is responsible for ensuring that the range `[start..start + 8]`
/// is within bounds of the `data` slice.
pub fn read_u64_slice(data: &[u8], start: usize) -> Result<u64, ProgramError> {
    Ok(
        u64::from_le_bytes(data[start..start + 8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?
        )
    )
}

/// # Safety
/// The caller is responsible for ensuring that the range `[start..start + 8]`
/// is within bounds of the `data` slice.
pub fn read_i64_slice(data: &[u8], start: usize) -> Result<i64, ProgramError> {
    Ok(
        i64::from_le_bytes(data[start..start + 8]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?
        )
    )
}

/// # Safety
/// The caller is responsible for ensuring that the range `[start..start + 4]`
/// is within bounds of the `data` slice.
pub fn read_u32_slice(data: &[u8], start: usize) -> Result<u32, ProgramError> {
    Ok(
        u32::from_le_bytes(data[start..start + 4]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?
        )
    )
}

/// # Safety
/// The caller is responsible for ensuring that the range `[start..start + 24]`
/// is within bounds of the `data` slice.
pub fn read_linear_vesting_strategy_slice(data: &[u8], start: usize) -> Result<LinearVestingStrategy, ProgramError> {
    Ok(LinearVestingStrategy {
        cliff_end_ts: read_i64_slice(data, start)?,
        vesting_end_ts: read_i64_slice(data, start + 8)?,
        unlock_period: read_i64_slice(data, start + 16)?
    })
}

/// Note, this method derives only PDA that are owned by the current program.
/// ### `program_id = ido_with_vesting::ID`
pub fn derive_program_pda(seeds: &[&[u8]]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        seeds, 
        &crate::ID
    )
}