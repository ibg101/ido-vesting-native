use solana_program::pubkey::Pubkey;


/// Neat way of importing Reader utils.
pub mod reader {
    pub use solana_bytes_reader::*;

    use solana_program::program_error::ProgramError;
    use crate::vesting::LinearVestingStrategy;


    pub trait ReaderExt {
        type Error;

        /// # Safety
        /// The caller is responsible for ensuring that the range `[start..start + 24]`
        /// is within bounds of the `data` slice.
        fn read_linear_vesting_strategy(&self, start: usize) -> Result<LinearVestingStrategy, Self::Error>; 
    }

    impl ReaderExt for Reader<'_> {
        type Error = ProgramError;

        fn read_linear_vesting_strategy(&self, start: usize) -> Result<LinearVestingStrategy, Self::Error> {
            read_linear_vesting_strategy_slice(self.bytes, start)
        }
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
}

/// Note, this method derives only PDA that are owned by the current program.
/// ### `program_id = ido_with_vesting::ID`
pub fn derive_program_pda(seeds: &[&[u8]]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        seeds, 
        &crate::ID
    )
}