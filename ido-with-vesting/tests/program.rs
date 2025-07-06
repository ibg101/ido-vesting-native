use mint_fixture::{
    MintFixture,
    MintFixtureClient,
    MintFixtureError,
};
use ido_with_vesting::{
    ID as IDO_PROGRAM_ID,
    external_ids::ATA_PROGRAM_ID,
    entrypoint,
    instruction,
    utils::derive_program_pda,
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
    rent::Rent,
    pubkey::Pubkey,
    instruction::Instruction,
};
use solana_sdk::{
    message::Message,
    transaction::Transaction,
    signer::Signer
};


#[tokio::test]
async fn test_all_instructions() -> Result<(), BanksClientError> {
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
    let mint_fixture: MintFixture = MintFixture::new(
        MintFixtureClient::Banks(&banks_client),
        &payer,
        &payer_pkey,
        &latest_blockhash,
        &rent
    );
    let mint_decimals: u8 = 9;
    let mint_amount: u64 = 1_000_000_000;
    let (mint_pkey, ata_pda) = mint_fixture.create_mint_and_funded_ata(mint_decimals, mint_amount)
        .await
        .map_err(|e| match e {
            MintFixtureError::Banks(e) => e,
            _ => panic!("Expected BanksClient, got RpcClient!")
        })?;

    // // 1. Craft InitializeIDOWithVesting instruction
    let transfer_amount: u64 = mint_amount;  // so we transfer the whole supply to the IDO
    let lamports_per_token: u32 = 1_000;
    let vesting_strategy: LinearVestingStrategy = LinearVestingStrategy::new_without_cliff(
        60 * 5,  // 5 minutes vesting
        60          // 1 minute every new unlock
    );

    let treasury_pda: Pubkey = derive_program_pda(&[
        IDO_TREASURY_ACCOUNT_SEED,
        mint_pkey.as_ref()
    ]).0;
    let config_pda: Pubkey = derive_program_pda(&[
        IDO_CONFIG_ACCOUNT_SEED,
        treasury_pda.as_ref()
    ]).0;

    let initialize_ido_ix: Instruction = instruction::create_initialize_with_vesting(
        transfer_amount, 
        lamports_per_token, 
        &vesting_strategy, 
        &payer_pkey, 
        &ata_pda, 
        &treasury_pda, 
        &config_pda, 
        &mint_pkey
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

    let buy_ix: Instruction = instruction::create_buy_with_vesting(
        buy_amount, 
        &payer_pkey, 
        &vesting_account, 
        &treasury_pda, 
        &config_pda, 
        &mint_pkey
    );

    // 6. Craft BuyWithVesting transaction
    let message: Message = Message::new(&[buy_ix], Some(&payer_pkey));
    let mut buy_tx: Transaction = Transaction::new_unsigned(message);

    // 7. Sign BuyWithVesting tx and send it
    buy_tx.sign(&[&payer], latest_blockhash);
    banks_client.process_transaction(buy_tx).await?;
    
    // 8. Derive recipient ATA and Craft Claim instruction.
    // I decided not to force the instruction to always interpriate `signer` as the `recipient`,
    // so the caller can pass any valid `recipient` and `recipient_ata` beside `signer` and `signer_ata`.
    // 8.1 Recipient is a new wallet
    let new_wallet: Pubkey = Pubkey::new_unique();
    let recipient_ata: Pubkey = Pubkey::find_program_address(
        &[
            new_wallet.as_ref(),
            SPL_TOKEN_2022_ID.as_ref(),
            mint_pkey.as_ref()
        ], 
        &ATA_PROGRAM_ID
    ).0;
    let claim_ix: Instruction = instruction::create_claim(
        &payer_pkey, 
        &new_wallet, 
        &recipient_ata, 
        &vesting_account, 
        &treasury_pda, 
        &config_pda, 
        &mint_pkey
    );

    // 8.2 Recipient is a signer
    // let recipient_ata: Pubkey = Pubkey::find_program_address(
    //     &[
    //         payer_pkey.as_ref(),
    //         SPL_TOKEN_2022_ID.as_ref(),
    //         mint_pkey.as_ref()
    //     ], 
    //     &ATA_PROGRAM_ID
    // ).0;
    // let claim_ix: Instruction = instruction::create_claim(
    //     &payer_pkey, 
    //     &payer_pkey, 
    //     &recipient_ata, 
    //     &vesting_account, 
    //     &treasury_pda, 
    //     &config_pda, 
    //     &mint_pkey
    // );

    // 9. Craft Claim transaction
    let message: Message = Message::new(&[claim_ix], Some(&payer_pkey));
    let mut claim_tx: Transaction = Transaction::new_unsigned(message);

    // 10. Sign Claim tx and send it 
    claim_tx.sign(&[&payer], latest_blockhash);
    banks_client.process_transaction(claim_tx).await?;

    Ok(())
}