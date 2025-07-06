use solana_program::{sysvar::clock::Clock, entrypoint::ProgramResult};
use super::{
    state::IDOVestingAccount,
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
        unsafe { 
            std::slice::from_raw_parts(
                self as *const Self as *const u8, 
                std::mem::size_of::<Self>()
            )
        }
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
    /// 
    /// Otherwise the provided cliff will be set and validated later in `Self::is_valid()` method.
    pub fn reinit_with_checked_cliff(self, clock: &Clock) -> Self {
        Self {
            cliff_end_ts: if self.cliff_end_ts == 0 { clock.unix_timestamp } else { self.cliff_end_ts },
            ..self
        }
    }

    #[cfg(feature = "ergonomic-init")]
    /// All arguments must be represented as seconds.
    pub fn new(
        cliff_duration: Option<i64>, 
        vesting_duration: i64,
        unlock_period: i64
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now_ts: i64 = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

        let cliff_end_ts: i64 = if let Some(cliff_duration) = cliff_duration {
            now_ts + cliff_duration
        } else {
            0
        };

        Self {
            cliff_end_ts,
            vesting_end_ts: vesting_duration + now_ts,
            unlock_period
        }
    }

    #[cfg(feature = "ergonomic-init")]
    pub fn new_without_cliff(
        vesting_duration: i64,
        unlock_period: i64
    ) -> Self {
        Self::new(None, vesting_duration, unlock_period)
    }
}

/// TODO: i'd like to refactor this fn later 

/// This function handles most of the business logic.
/// However i didn't decide to delegate transfering tokens to this fn, so it must be implemented externally.
/// 
/// This fn has the following flow. Checks for:
/// 1. Cliff Period is active => returns `IDOProgramError::CliffIsActive`.
/// 2. Vesting Period is over => returns the `left_transfer_portion`.
/// 3. First OR next Unlock is reached => returns the `transfer_portion` that's calculated based on `unlocked_times`.
/// 
/// Otherwise Vesting is still considered active and the appropriated error is returned.
pub fn allow_claim_and_define_portion(
    clock: &Clock,
    vesting_strategy: &LinearVestingStrategy,
    vesting_account: &mut IDOVestingAccount
) -> Result<u64, IDOProgramError> {
    let now_ts: i64 = clock.unix_timestamp;
    let LinearVestingStrategy { cliff_end_ts, vesting_end_ts, unlock_period } = *vesting_strategy;

    if now_ts < cliff_end_ts {
        return Err(IDOProgramError::CliffIsActive);
    }

    let bought_amount: u64 = vesting_account.bought_amount;

    // vesting period is over
    if now_ts >= vesting_end_ts {
        let left_transfer_portion: u64 = bought_amount - vesting_account.claimed_amount;
        vesting_account.claimed_amount += left_transfer_portion;
        return Ok(left_transfer_portion);
    }
    
    let last_claim_ts: i64 = vesting_account.last_claim_ts;
    let never_claimed: bool = last_claim_ts == 0;

    // first claim OR new portion is available to be claimed
    if never_claimed
    || now_ts >= last_claim_ts + unlock_period {
        let vesting_ends_in: i64 = vesting_end_ts - now_ts;
        let vesting_duration: i64 = vesting_end_ts - cliff_end_ts;
        let time_passed: i64 = vesting_duration - vesting_ends_in;
        let unlocked_times: i64 = if never_claimed {
            // if cliff has ended and user tries to immediately claim the tokens and next unlock period is not reached yet,
            // omitting max(1) will cause multiplying by 0 bug
            (time_passed / unlock_period).max(1)
        } else {
            time_passed / unlock_period
        };
        let transfer_portion: u64 = vesting_account.amount_per_unlock * unlocked_times as u64;

        vesting_account.last_claim_ts = now_ts;
        vesting_account.claimed_amount += transfer_portion;
        return Ok(transfer_portion);
    }

    Err(IDOProgramError::VestingIsActive)    
}