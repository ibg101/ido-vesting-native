#[allow(deprecated)]
use solana_sdk::{
    rent::Rent,
    hash::Hash,
    pubkey::Pubkey,
    program_pack::Pack,
    message::Message,
    system_program::ID as SYSTEM_PROGRAM_ID,
    system_transaction,
    transaction::Transaction,
    instruction::{Instruction, AccountMeta},
    signer::keypair::Keypair,
    signature::Signer,
};
use spl_token_2022::{
    state::Mint,
    ID as SPL_TOKEN_2022_ID
};
use solana_program_test::{
    BanksClient, 
    BanksClientError
};
use solana_client::{
    client_error::ClientError,
    nonblocking::rpc_client::RpcClient
};


// since i've implemented ix crafting & ATA derivation manually, importing `spl_associated_token_account_client` only for ID is an overkill
const ATA_PROGRAM_ID: Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

#[derive(Debug)]
pub enum MintFixtureError {
    Client(ClientError),
    Banks(BanksClientError)
}

impl From<ClientError> for MintFixtureError {
    fn from(value: ClientError) -> Self {
        Self::Client(value)
    }
}

impl From<BanksClientError> for MintFixtureError {
    fn from(value: BanksClientError) -> Self {
        Self::Banks(value)
    }
}

pub enum MintFixtureClient<'a> {
    Rpc(&'a RpcClient),
    Banks(&'a BanksClient),
}

pub struct MintFixture<'a> {
    client: MintFixtureClient<'a>,
    payer: &'a Keypair,
    payer_pkey: &'a Pubkey,
    latest_blockhash: &'a Hash,
    rent: &'a Rent
}

impl<'a> MintFixture<'a> {
    pub fn new(
        client: MintFixtureClient<'a>,
        payer: &'a Keypair,
        payer_pkey: &'a Pubkey,
        latest_blockhash: &'a Hash,
        rent: &'a Rent 
    ) -> Self {
        Self { client, payer, payer_pkey, latest_blockhash, rent }
    }

    /// `mint_amount` - for example: 1_000_000_000
    pub async fn create_mint_and_funded_ata(&self, mint_decimals: u8, mint_amount: u64) -> Result<(Pubkey, Pubkey), MintFixtureError> {
        let mint_pkey: Pubkey = self.create_and_intiialize_mint(mint_decimals).await?;
        let ata_pda: Pubkey = self.create_and_intiialize_ata(&mint_pkey).await?;
        self.mint_to_ata(&mint_pkey, &ata_pda, mint_amount).await?;

        Ok((mint_pkey, ata_pda))
    }

    pub async fn create_and_intiialize_mint(&self, mint_decimals: u8) -> Result<Pubkey, MintFixtureError> {
        // 1. create account using system program
        let mint_keypair: Keypair = Keypair::new();

        let create_tx: Transaction = system_transaction::create_account(
            self.payer, 
            &mint_keypair, 
            *self.latest_blockhash, 
            self.rent.minimum_balance(Mint::LEN), 
            Mint::LEN as u64, 
            &SPL_TOKEN_2022_ID
        );

        self.process_transaction(create_tx).await?;

        // 2. initialize Mint account using SPL program
        // 2.1 craft initialize mint ix & tx
        let initialize_mint_ix: Instruction = spl_token_2022::instruction::initialize_mint(
            &SPL_TOKEN_2022_ID, 
            &mint_keypair.pubkey(), 
            self.payer_pkey, 
            None, 
            mint_decimals
        ).map_err(|_| BanksClientError::ClientError("Failed to craft InitializeMint ix!"))?;
        let message: Message = Message::new(&[initialize_mint_ix], Some(self.payer_pkey));
        let mut initialize_mint_tx: Transaction = Transaction::new_unsigned(message);
        
        // 2.2 sign & send tx
        initialize_mint_tx.sign(&[self.payer], *self.latest_blockhash);
        self.process_transaction(initialize_mint_tx).await?;

        Ok(mint_keypair.pubkey())
    }

    pub async fn create_and_intiialize_ata(&self, mint_pkey: &Pubkey) -> Result<Pubkey, MintFixtureError> {
        let ata_pda: Pubkey = Pubkey::find_program_address(
            &[
                self.payer_pkey.as_ref(),
                SPL_TOKEN_2022_ID.as_ref(),
                mint_pkey.as_ref()
            ], 
            &ATA_PROGRAM_ID
        ).0;

        let create_ata_ix: Instruction = Instruction::new_with_bytes(
            ATA_PROGRAM_ID, 
            &[0],  // AssociatedTokenAccount::Create ix
            vec![
                AccountMeta::new(*self.payer_pkey, true),
                AccountMeta::new(ata_pda, false),
                AccountMeta::new_readonly(*self.payer_pkey, false),
                AccountMeta::new_readonly(*mint_pkey, false),
                AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
                AccountMeta::new_readonly(SPL_TOKEN_2022_ID, false)
            ]
        );

        let message: Message = Message::new(&[create_ata_ix], Some(self.payer_pkey));
        let mut create_ata_tx: Transaction = Transaction::new_unsigned(message); 

        create_ata_tx.sign(&[self.payer], *self.latest_blockhash);
        self.process_transaction(create_ata_tx).await?;

        Ok(ata_pda)
    }

    pub async fn mint_to_ata(&self, mint_pkey: &Pubkey, ata_pda: &Pubkey, mint_amount: u64) -> Result<(), MintFixtureError> {
        let mut mint_to_ix_payload: Vec<u8> = Vec::with_capacity(9);
        mint_to_ix_payload.push(7);
        mint_to_ix_payload.extend_from_slice(&u64::to_le_bytes(mint_amount)); 
        
        let mint_to_ix: Instruction = Instruction::new_with_bytes(
            SPL_TOKEN_2022_ID, 
            &mint_to_ix_payload, 
            vec![
                AccountMeta::new(*mint_pkey, false),
                AccountMeta::new(*ata_pda, false),
                AccountMeta::new_readonly(*self.payer_pkey, true)
            ]
        );
        let message: Message = Message::new(&[mint_to_ix], Some(self.payer_pkey));
        let mut mint_to_tx: Transaction = Transaction::new_unsigned(message);

        mint_to_tx.sign(&[self.payer], *self.latest_blockhash);
        self.process_transaction(mint_to_tx).await?;

        Ok(())
    }

    async fn process_transaction(&self, tx: Transaction) -> Result<(), MintFixtureError> {
        match self.client {
            MintFixtureClient::Rpc(client) => {
                client.send_and_confirm_transaction(&tx).await?;
            },
            MintFixtureClient::Banks(client) => {
                client.process_transaction(tx).await?;
            },
        }

        Ok(())
    }
}

