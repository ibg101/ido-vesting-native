use std::error::Error;
use solana_program::program_error::ProgramError;


#[repr(u32)]
#[derive(Debug)]
pub enum IDOProgramError {
    MaxUnlocksOverflow,
    UnlocksMustNotEqualZero,
    CliffIsActive,
    VestingIsActive,
    VestingPeriodMustBeGreaterThanNow,
    CliffPeriodMustBeGreaterThanNow,
    VestingPeriodMustBeGreaterThanCliff,
    AlreadyClaimed,
    VestingPeriodEnded
}

impl Error for IDOProgramError {}
impl std::fmt::Display for IDOProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::MaxUnlocksOverflow => "Max Unlocks must not be greater than 100! Visit docs for more info on setting up Vesting Strategy.",
            Self::UnlocksMustNotEqualZero => "Unlocks must not equal zero! Visit docs to see how unlocks are calculated based on the Vesting Strategy.",
            Self::CliffIsActive => "Cliff Period is still active.",
            Self::VestingIsActive => "Vesting Period is still active, please wait until it's possible to claim the next portion of tokens.",
            Self::VestingPeriodMustBeGreaterThanNow => "Vesting Period must be greater than Current Timestamp.",
            Self::CliffPeriodMustBeGreaterThanNow => "Cliff Period must be greater than Current Timestamp.",
            Self::VestingPeriodMustBeGreaterThanCliff => "Vesting Period must be greater than Cliff Period.",
            Self::AlreadyClaimed => "Already claimed! No tokens to claim.",
            Self::VestingPeriodEnded => "Vesting Period has ended!"
        };

        f.write_str(msg)
    }
}

impl From<IDOProgramError> for ProgramError {
    fn from(value: IDOProgramError) -> Self {
        Self::Custom(value as u32)
    }
}