mod utils;
use utils::spl_token_manipulations::Prelude;

use ido_with_vesting::{
    ID as IDO_PROGRAM_ID,
    entrypoint,
    vesting::LinearVestingStrategy
};

use spl_token_2022::ID as SPL_TOKEN_2022_ID;
use solana_program_test::{
    ProgramTest,
    BanksClientError,
    processor
};
use solana_program::{
    system_program::ID as SYSTEM_PROGRAM_ID,
    rent::Rent,
    hash::Hash,
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
};
use solana_sdk::{
    sysvar::SysvarId,
    signer::{keypair::Keypair, Signer}
};


#[tokio::test]
async fn test_initialize_ido_with_vesting_ix() -> Result<(), BanksClientError> {
    // spl token 2022 is preloaded automatically, so there is no need to explicitly add_program with spl-token-2022 binary
    let program: ProgramTest = ProgramTest::new(
        "ido_with_vesting", 
        IDO_PROGRAM_ID,
        processor!(entrypoint::process_instruction)
    );

    let (banks_client, payer, latest_blockhash) = program.start().await;
    let payer_pkey: Pubkey = payer.pubkey();
    let rent: Rent = banks_client.get_sysvar::<Rent>().await?;

    // 0. Create & Initialize Mint Account; Create & Initialize ATA; Mint tokens to ATA
    let prelude: Prelude = Prelude::init(&banks_client, &payer, &payer_pkey, &latest_blockhash, &rent);
    let (mint_pkey, ata_pda) = prelude.create_mint_and_funded_ata().await?;

    // // 1. Craft instruction
    let transfer_amount: u64 = 1_000_000_000;
    let lamports_per_token: u32 = 1_000;
    let vesting_strategy: LinearVestingStrategy = LinearVestingStrategy::new_without_cliff(
        60 * 5,  // 5 minutes vesting
        60          // 1 minute every new unlock
    );

    let mut ix_payload: Vec<u8> = Vec::with_capacity(37);         
    ix_payload.push(0);  // IDOInstruction::InitializeWithVesting
    ix_payload.extend_from_slice(&transfer_amount.to_le_bytes());
    ix_payload.extend_from_slice(&lamports_per_token.to_le_bytes());
    ix_payload.extend_from_slice(vesting_strategy.as_ref());

    let initialize_ido_ix: Instruction = Instruction::new_with_bytes(
        IDO_PROGRAM_ID, 
        &ix_payload, 
        vec![
            AccountMeta::new(payer_pkey, true),
            AccountMeta::new(ata_pda, false),
            // AccountMeta::new(treasury_pda, false),
            // AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(mint_pkey, false),
            AccountMeta::new_readonly(Rent::id(), false),
            AccountMeta::new_readonly(SPL_TOKEN_2022_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ]
    );

    // 2. Craft transaction

    // 3. Sign tx and send it

    Ok(())
}