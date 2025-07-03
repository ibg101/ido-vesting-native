use std::array::TryFromSliceError;
use solana_program::{sysvar::clock::Clock, entrypoint::ProgramResult};

use super::{
    error::IDOProgramError,
    constants::MAX_UNLOCKS
};


#[repr(C)]
pub struct LinearVestingStrategy {
    pub cliff_end_ts: i64,      // timestamp in secs   
    pub vesting_end_ts: i64,    // timestamp in secs
    pub unlock_period: i64      // DAYS/MONTHS in secs
}

impl AsRef<[u8]> for LinearVestingStrategy {
    fn as_ref(&self) -> &[u8] {
        let ptr: *const u8 = &raw const self as *const u8;
        let len: usize = std::mem::size_of::<Self>();

        unsafe {
            std::slice::from_raw_parts(ptr, len)
        }
    }
}

impl TryFrom<&[u8]> for LinearVestingStrategy {
    type Error = TryFromSliceError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self {
            cliff_end_ts: i64::from_le_bytes(value[..8].try_into()?),
            vesting_end_ts: i64::from_le_bytes(value[8..16].try_into()?),
            unlock_period: i64::from_le_bytes(value[16..24].try_into()?)
        })
    }
}

impl LinearVestingStrategy {
    pub fn is_valid(&self, clock: &Clock) -> ProgramResult {
        let now_ts: i64 = clock.unix_timestamp;
        let LinearVestingStrategy { cliff_end_ts, vesting_end_ts, unlock_period } = *self;
        
        if cliff_end_ts < now_ts {
            return Err(IDOProgramError::CliffPeriodMustBeGreaterThanNow.into());
        }

        if vesting_end_ts <= now_ts {
            return Err(IDOProgramError::VestingPeriodMustBeGreaterThanNow.into());
        }

        if cliff_end_ts >= vesting_end_ts {
            return Err(IDOProgramError::VestingPeriodMustBeGreaterThanCliff.into());
        }

        let unlocks: i64 = (vesting_end_ts - cliff_end_ts) / unlock_period;
        
        if unlocks == 0 {
            return Err(IDOProgramError::UnlocksMustNotEqualZero.into());
        }

        if unlocks > MAX_UNLOCKS as i64 {
            return Err(IDOProgramError::MaxUnlocksOverflow.into());
        }
        
        Ok(())
    }

    /// If the cliff equals to 0 => which basically means there is no cliff, program will use the current timestamp as the end of the cliff
    /// so the vesting period starts.
    /// Otherwise the provided cliff will be set and validated later in `Self::is_valid()` method.
    pub fn reinit_with_checked_cliff(self, clock: &Clock) -> Self {
        Self {
            cliff_end_ts: if self.cliff_end_ts == 0 { clock.unix_timestamp } else { self.cliff_end_ts },
            ..self
        }
    }
}