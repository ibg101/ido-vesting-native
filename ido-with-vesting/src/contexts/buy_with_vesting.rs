use solana_program::{
    program_error::ProgramError,
    account_info::{next_account_info, AccountInfo}
};


pub struct IDOBuyWithVestingCtx<'a, 'b> {
    pub signer_info: &'a AccountInfo<'b>,
    pub vesting_info: &'a AccountInfo<'b>,
    pub treasury_info: &'a AccountInfo<'b>,
    pub config_info: &'a AccountInfo<'b>,
    pub mint_info: &'a AccountInfo<'b>,
    pub system_program_info: &'a AccountInfo<'b>
}

impl<'a, 'b> TryFrom<&'a [AccountInfo<'b>]> for IDOBuyWithVestingCtx<'a, 'b> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo<'b>]) -> Result<Self, Self::Error> {
        let accounts_iter = &mut accounts.iter();
        
        Ok(Self {
            signer_info: next_account_info(accounts_iter)?,
            vesting_info: next_account_info(accounts_iter)?,
            treasury_info: next_account_info(accounts_iter)?,
            config_info: next_account_info(accounts_iter)?,
            mint_info: next_account_info(accounts_iter)?,
            system_program_info: next_account_info(accounts_iter)?
        })
    }    
}