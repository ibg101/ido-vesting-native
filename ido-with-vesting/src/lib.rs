pub mod state;
pub mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod vesting;
pub mod constants;
pub mod contexts;
pub mod error;
pub mod utils;

use solana_program::{declare_id, pubkey::Pubkey};


declare_id!("BhMF5PU37Ssyjwjp4FmHufc1b1pYZXZrRmNP4kV3fFc5");

/// This module is used for external Program ID's declaration.
/// 
/// Some of these IDs are temporarily located here until the appropriate crates 
/// are added, from which the corresponding IDs can be imported.
///
/// For example, I decided to work with the ATA program without using any external crates for now, 
/// but in production, official crate implementations should be used instead.
pub mod external_ids {
    use super::Pubkey;

    /// if spl_associated_token_account crate is added => remove this
    pub const ATA_PROGRAM_ID: Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
}