use solana_program::pubkey::Pubkey;


/// Neat way of importing Reader utils.
pub mod reader {
    pub use solana_bytes_reader::*;

    use solana_program::program_error::ProgramError;
    use crate::vesting::LinearVestingStrategy;


    pub trait ReaderExt {
        type Error;

        /// # Safety
        /// The caller is responsible for ensuring that the range `[self.offset..self.offset + 24]`
        /// is within bounds of the `bytes` slice.
        fn read_linear_vesting_strategy(&mut self, move_cursor: bool) -> Result<LinearVestingStrategy, Self::Error>; 
    }

    impl ReaderExt for Reader<'_> {
        type Error = ProgramError;

        fn read_linear_vesting_strategy(&mut self, move_cursor: bool) -> Result<LinearVestingStrategy, Self::Error> {
            let strategy: LinearVestingStrategy = read_linear_vesting_strategy_slice(self.bytes(), self.offset())?;
            
            if move_cursor {
                self.set_offset(self.offset() + 24);
            }

            Ok(strategy)
        }
    }

    /// # Safety
    /// The caller is responsible for ensuring that the range `[start..start + 24]`
    /// is within bounds of the `data` slice.
    pub fn read_linear_vesting_strategy_slice(data: &[u8], start: usize) -> Result<LinearVestingStrategy, ProgramError> {
        let mut reader: Reader = Reader::new_with_offset(data, start);

        Ok(LinearVestingStrategy {
            cliff_end_ts: reader.read_i64()?,
            vesting_end_ts: reader.read_i64()?,
            unlock_period: reader.read_i64()?
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