use std::array::TryFromSliceError;


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