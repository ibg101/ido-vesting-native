mod utils;
use utils::spl_token_manipulations::Prelude;

use ido_with_vesting::{
    ID as IDO_PROGRAM_ID,
    entrypoint,
    vesting::LinearVestingStrategy,
    constants::{
        IDO_CONFIG_ACCOUNT_SEED, 
        IDO_TREASURY_ACCOUNT_SEED,
        IDO_VESTING_ACCOUNT_SEED
    }
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
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
};
use solana_sdk::{
    message::Message,
    transaction::Transaction,
    sysvar::SysvarId,
    signer::Signer
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

    // // 1. Craft InitializeIDOWithVesting instruction
    let transfer_amount: u64 = 1_000_000_000;
    let lamports_per_token: u32 = 1_000;
    let vesting_strategy: LinearVestingStrategy = LinearVestingStrategy::new_without_cliff(
        60 * 5,  // 5 minutes vesting
        60          // 1 minute every new unlock
    );

    let treasury_pda: Pubkey = Pubkey::find_program_address(
        &[
            IDO_TREASURY_ACCOUNT_SEED,
            mint_pkey.as_ref()
        ],
        &IDO_PROGRAM_ID
    ).0;
    let config_pda: Pubkey = Pubkey::find_program_address(
        &[
            IDO_CONFIG_ACCOUNT_SEED,
            treasury_pda.as_ref()
        ], 
        &IDO_PROGRAM_ID
    ).0;

    let mut init_ix_payload: Vec<u8> = Vec::with_capacity(37);         
    init_ix_payload.push(0);  // IDOInstruction::InitializeWithVesting
    init_ix_payload.extend_from_slice(&transfer_amount.to_le_bytes());
    init_ix_payload.extend_from_slice(&lamports_per_token.to_le_bytes());
    init_ix_payload.extend_from_slice(vesting_strategy.as_ref());

    let initialize_ido_ix: Instruction = Instruction::new_with_bytes(
        IDO_PROGRAM_ID, 
        &init_ix_payload, 
        vec![
            AccountMeta::new(payer_pkey, true),
            AccountMeta::new(ata_pda, false),
            AccountMeta::new(treasury_pda, false),
            AccountMeta::new(config_pda, false),
            AccountMeta::new_readonly(mint_pkey, false),
            AccountMeta::new_readonly(Rent::id(), false),
            AccountMeta::new_readonly(SPL_TOKEN_2022_ID, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ]
    );

    // 2. Craft InitializeIDOWithVesting transaction
    let message: Message = Message::new(&[initialize_ido_ix], Some(&payer_pkey));
    let mut initialize_ido_tx: Transaction = Transaction::new_unsigned(message);

    // 3. Sign InitializeIDOWithVesting tx and send it
    initialize_ido_tx.sign(&[&payer], latest_blockhash);
    banks_client.process_transaction(initialize_ido_tx).await?;

    // 4. Derive VestingAccount PDA
    let (vesting_account, _vesting_bump) = Pubkey::find_program_address(
        &[
            IDO_VESTING_ACCOUNT_SEED,
            payer_pkey.as_ref(),
            mint_pkey.as_ref()
        ], 
        &IDO_PROGRAM_ID
    );

    // 5. Craft BuyWithVesting instruction
    let buy_amount: u64 = 17_000_000;

    let mut buy_ix_payload: Vec<u8> = Vec::with_capacity(9);
    buy_ix_payload.push(1); 
    buy_ix_payload.extend_from_slice(&buy_amount.to_le_bytes());

    let buy_ix: Instruction = Instruction::new_with_bytes(
        IDO_PROGRAM_ID, 
        &buy_ix_payload, 
        vec![
            AccountMeta::new(payer_pkey, true),
            AccountMeta::new(vesting_account, false),
            AccountMeta::new(treasury_pda, false),
            AccountMeta::new_readonly(config_pda, false),
            AccountMeta::new_readonly(mint_pkey, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false)
        ]
    );

    // 6. Craft BuyWithVesting transaction
    let message: Message = Message::new(&[buy_ix], Some(&payer_pkey));
    let mut buy_tx: Transaction = Transaction::new_unsigned(message);

    // 7. Sign BuyWithVesting tx and send it
    buy_tx.sign(&[&payer], latest_blockhash);
    banks_client.process_transaction(buy_tx).await?;
    
    Ok(())
}