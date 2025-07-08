use ido_with_vesting::{
    ID as IDO_PROGRAM_ID,
    external_ids::ATA_PROGRAM_ID,
    utils::derive_program_pda,
    instruction,
    vesting::LinearVestingStrategy,
    constants::{
        IDO_CONFIG_ACCOUNT_SEED,
        IDO_TREASURY_ACCOUNT_SEED,
        IDO_VESTING_ACCOUNT_SEED
    }
};
use mint_fixture::{
    MintFixture,
    MintFixtureClient
};
use spl_token_2022::ID as SPL_TOKEN_2022_ID;
use solana_sdk::{
    rent::Rent,
    hash::Hash,
    pubkey::Pubkey,
    message::Message,
    transaction::{Transaction, TransactionError},
    instruction::{Instruction, InstructionError},
    native_token::LAMPORTS_PER_SOL,
    signer::{keypair::Keypair, Signer},
};
use solana_client::{
    rpc_request::{RpcError, RpcResponseErrorData},
    rpc_response::RpcSimulateTransactionResult,
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient
};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::from_path(std::path::Path::new("./ido-with-vesting/.env"))?;
    env_logger::init();

    // 0. Define RpcClient and init payer
    let url: String = std::env::var("RPC_HTTP_URL")?;
    let rpc_client: RpcClient = RpcClient::new(url);
    let (payer_pkey, payer) = init_payer(&rpc_client).await?;

    // 1. Create & Initialize Mint Account; Create & Initialize ATA; Mint tokens to ATA
    let rent: Rent = Rent::default();
    let mint_fixture: MintFixture = MintFixture::new(
        MintFixtureClient::Rpc(&rpc_client),
        &payer,
        &payer_pkey,
        &rent
    );
    let mint_decimals: u8 = 9;
    let mint_raw_amount: u64 = 1_000_000_000;
    let mint_amount: u64 = mint_raw_amount * 10u64.pow(mint_decimals as u32);

    let latest_blockhash: Hash = rpc_client.get_latest_blockhash().await?;
    let mint_pkey: Pubkey = mint_fixture.create_and_intiialize_mint(mint_decimals, &latest_blockhash).await?;
    log::info!("prelude: create mint account - success: {}", mint_pkey);
    let ata_pda: Pubkey = mint_fixture.create_and_intiialize_ata(&mint_pkey, &latest_blockhash).await?;
    log::info!("prelude: create signers ata - success: {}", ata_pda);
    let latest_blockhash: Hash = rpc_client.get_latest_blockhash().await?;  // we get new blockhash, because prev hash expires at this point
    mint_fixture.mint_to_ata(&mint_pkey, &ata_pda, mint_amount, &latest_blockhash).await?;
    log::info!("prelude: mint tokens to signers ata - success: {}", ata_pda);

    // 2. Derive all required PDA
    let treasury_pda: Pubkey = derive_program_pda(&[
        IDO_TREASURY_ACCOUNT_SEED,
        mint_pkey.as_ref()
    ]).0;
    let config_pda: Pubkey = derive_program_pda(&[
        IDO_CONFIG_ACCOUNT_SEED,
        treasury_pda.as_ref()
    ]).0;
    let (vesting_account, _vesting_bump) = Pubkey::find_program_address(
        &[
            IDO_VESTING_ACCOUNT_SEED,
            payer_pkey.as_ref(),
            mint_pkey.as_ref()
        ], 
        &IDO_PROGRAM_ID
    );

    // 3. Init ProgramClient
    let recipient_pkey: Pubkey = if std::env::var("RECIPIENT_IS_SIGNER")?.parse()? {
        payer_pkey
    } else {
        Pubkey::new_unique()
    };
    let recipient_ata: Pubkey = Pubkey::find_program_address(
        &[
            recipient_pkey.as_ref(),
            SPL_TOKEN_2022_ID.as_ref(),
            mint_pkey.as_ref()
        ], 
        &ATA_PROGRAM_ID
    ).0;
    let required_accounts: RequiredAccounts = RequiredAccounts { 
        payer_pkey: &payer_pkey, 
        recipient_pkey: &recipient_pkey,
        recipient_ata: &recipient_ata,
        mint_pkey: &mint_pkey, 
        ata_pda: &ata_pda, 
        treasury_pda: &treasury_pda, 
        config_pda: &config_pda, 
        vesting_account: &vesting_account 
    };
    let program_client: ProgramClient = ProgramClient::new(&rpc_client, &payer, required_accounts);

    // 4. initialize IDO with vesting
    let transfer_amount: u64 = mint_amount;  // so we transfer the whole supply to the IDO
    let lamports_per_token: u32 = 1_000;
    let vesting_duration_secs: i64 = 60 * 5;  // 5 minutes vesting
    let unlock_period_secs: i64 = 60;         // 1 minute every new unlock
    let vesting_strategy: LinearVestingStrategy = LinearVestingStrategy::new_without_cliff(
        vesting_duration_secs,
        unlock_period_secs
    );
    program_client.initialize_ido_with_vesting(transfer_amount, lamports_per_token, vesting_strategy).await?;

    // 5. buy tokens with vesting
    // Currently i airdrop only 5 SOL to the newly created payer account
    // and set lamports_per_token = 1_000 in LinearVestingStrategy. 
    // That being said if you try to set buy amount greater/equal than/to 5 SOL => it will fail.
    // If you still want to do so => go to the .env and set CREATE_NEW_PAYER=false & define PAYER_SEED_PHRASE=...
    let buy_amount: u64 = 1_000_000;
    program_client.buy_with_vesting(buy_amount).await?;

    // 6. try to claim tokens (1st claim must immeditately pass, since in current vesting strategy cliff period does not exist)
    program_client.claim().await?;

    log::info!("Simulate claim before next unlock! This must fail!");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;  // add some delay, but not greater than `unlock_period`
    program_client.claim().await?;  // this must fail!!!

    // 7. now let's try to claim tokens for next 2 unlock period.
    let delay: i64 = unlock_period_secs * 2;
    log::info!("Simulate claim after 2 unlock periods! Waiting: {} seconds..", delay);
    tokio::time::sleep(tokio::time::Duration::from_secs(delay as u64)).await;  
    program_client.claim().await?;

    // 8. let's buy MORE tokens, the more the better yeah?
    log::info!("Simulate additional buy during Vesting Period and after some claims!");
    let buy_amount: u64 = 2_000_000;
    program_client.buy_with_vesting(buy_amount).await?;

    // 9. finally let's try to claim the rest tokens.
    let delay: i64 = vesting_duration_secs - unlock_period_secs * 2; 
    log::info!("Simulate claim the rest tokens! Waiting: {} seconds..", delay);
    tokio::time::sleep(tokio::time::Duration::from_secs(delay as u64)).await;  
    program_client.claim().await?;

    // 10. try to buy after vesting period is over
    log::info!("Simulate additional buy after Vesting Period! This must fail!");
    let buy_amount: u64 = 500_000;
    program_client.buy_with_vesting(buy_amount).await?;  // this must fail!!!

    Ok(())
}


type ProgramClientResult = Result<(), ClientError>;

struct ProgramClient<'a> {
    client: &'a RpcClient,
    payer: &'a Keypair,
    required_accounts: RequiredAccounts<'a>,
}

struct RequiredAccounts<'a> {
    payer_pkey: &'a Pubkey,
    /// this account is used as a recipient address when invoking claim instruction
    recipient_pkey: &'a Pubkey,
    recipient_ata: &'a Pubkey,
    mint_pkey: &'a Pubkey,
    ata_pda: &'a Pubkey, 
    treasury_pda: &'a Pubkey,
    config_pda: &'a Pubkey,
    vesting_account: &'a Pubkey,
}

impl<'a> ProgramClient<'a> {
    fn new(
        client: &'a RpcClient,
        payer: &'a Keypair, 
        required_accounts: RequiredAccounts<'a>
    ) -> Self {        
        Self {
            client,
            payer,
            required_accounts
        }
    }

    async fn initialize_ido_with_vesting(
        &self, 
        transfer_amount: u64,
        lamports_per_token: u32,
        vesting_strategy: LinearVestingStrategy
    ) -> ProgramClientResult {
        let RequiredAccounts { 
            payer_pkey, 
            mint_pkey, 
            ata_pda, 
            treasury_pda, 
            config_pda,
            .. 
        } = self.required_accounts;

        let initialize_ido_ix: Instruction = instruction::create_initialize_with_vesting(
            transfer_amount, 
            lamports_per_token, 
            &vesting_strategy, 
            payer_pkey, 
            ata_pda, 
            treasury_pda, 
            config_pda, 
            mint_pkey
        );

        Self::craft_tx_and_process(&self, &[initialize_ido_ix], "initialize ido with vesting").await?;

        Ok(())
    }

    async fn buy_with_vesting(&self, buy_amount: u64) -> ProgramClientResult {
        let RequiredAccounts { 
            payer_pkey, 
            mint_pkey, 
            treasury_pda, 
            config_pda, 
            vesting_account,
            .. 
        } = self.required_accounts;
        
        let buy_ix: Instruction = instruction::create_buy_with_vesting(
            buy_amount, 
            payer_pkey, 
            vesting_account, 
            treasury_pda, 
            config_pda, 
            mint_pkey
        );

        Self::craft_tx_and_process(&self, &[buy_ix], "buy with vesting").await?;
        
        Ok(())
    }

    async fn claim(&self) -> ProgramClientResult {
        let RequiredAccounts { 
            payer_pkey, 
            recipient_pkey, 
            recipient_ata, 
            mint_pkey, 
            treasury_pda, 
            config_pda, 
            vesting_account,
            .. 
        } = self.required_accounts;

        let claim_ix: Instruction = instruction::create_claim(
            payer_pkey, 
            recipient_pkey, 
            recipient_ata, 
            vesting_account, 
            treasury_pda, 
            config_pda, 
            mint_pkey
        );

        Self::craft_tx_and_process(&self, &[claim_ix], "claim").await?;
        
        Ok(())
    }

    async fn craft_tx_and_process(&self, ixs: &[Instruction], operation_tag: &str) -> ProgramClientResult {
        let message: Message = Message::new(ixs, Some(self.required_accounts.payer_pkey));
        let mut tx: Transaction = Transaction::new_unsigned(message);
        
        let latest_blockhash: Hash = self.client.get_latest_blockhash().await?;
        tx.sign(&[self.payer], latest_blockhash);

        match self.client.send_and_confirm_transaction(&tx).await {
            Ok(sig) => log::info!("{}: success | signature: {}", operation_tag, sig),
            Err(e) => {
                if is_contract_violation_from_error(&e) {
                    log::info!("contract denied operation | test passed");
                    return Ok(())  // that's expected behavior
                }       
   
                log::error!("{}", e);
                return Err(e);
            }  
        }

        Ok(())
    }
}

async fn init_payer(client: &RpcClient) -> Result<(Pubkey, Keypair), Box<dyn std::error::Error>> {
    Ok(if !std::env::var("CREATE_NEW_PAYER")?.parse::<bool>()? {
        log::warn!("CREATE_NEW_PAYER=true which means PAYER_SEED_PHRASE will be used. Make sure that:\n\
        1. Valid value is provided.\n\
        2. Account has enough funds.");

        let payer: Keypair = Keypair::from_base58_string(&std::env::var("PAYER_SEED_PHRASE")?);
        let payer_pkey: Pubkey = payer.pubkey();
        
        (payer_pkey, payer)
    } else {
        let payer: Keypair = Keypair::new();
        let payer_pkey: Pubkey = payer.pubkey();
    
        let request_amount: u64 = LAMPORTS_PER_SOL * 5;
        let airdrop_sig = client.request_airdrop(&payer_pkey, request_amount).await?;
        loop {
            if client.confirm_transaction(&airdrop_sig).await? {
                break;
            }
        }

        (payer_pkey, payer)
    })
}

fn is_contract_violation_from_error(e: &ClientError) -> bool {
    match &e.kind {
        ClientErrorKind::RpcError(RpcError::RpcResponseError { data, .. }) => {
            match data {
                RpcResponseErrorData::SendTransactionPreflightFailure(
                    RpcSimulateTransactionResult { err: Some(TransactionError::InstructionError(_, InstructionError::Custom(code))), .. }
                ) => is_contract_violation(*code),
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_contract_violation(code: u32) -> bool {
    log::info!("program error discriminator: {}", code);
    matches!(code, 0x2 | 0x3 | 0x7 | 0x8 | 0x9)
}