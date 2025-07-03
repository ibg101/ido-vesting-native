use solana_program_test::{
    BanksClient, 
    BanksClientError, 
};
use spl_token_2022::{
    state::Mint,
    ID as SPL_TOKEN_2022_ID
};
use solana_program::{
    hash::Hash,
    rent::Rent,
    system_program::{ID as SYSTEM_PROGRAM_ID},
    pubkey::Pubkey,
    program_pack::Pack,
    instruction::{AccountMeta, Instruction}
};
#[allow(deprecated)]
use solana_sdk::{
    message::Message,
    transaction::Transaction,
    system_transaction,
    signer::{Signer, keypair::Keypair}
};


const ATA_PROGRAM_ID: Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub trait BanksClientExt {    
    /// Creates & sends `system_transaction::create_account()` Transaction.
    fn send_tx_create_account(
        &self,
        from_keypair: &Keypair, 
        to_keypair: Option<Keypair>,
        recent_blockhash: &Hash,
        lamports: u64, 
        space: u64, 
        owner: &Pubkey
    ) -> impl std::future::Future<Output = Result<Keypair, BanksClientError>>;
}

impl BanksClientExt for BanksClient {
    fn send_tx_create_account(
        &self,
        from_keypair: &Keypair, 
        to_keypair: Option<Keypair>,
        recent_blockhash: &Hash,
        lamports: u64, 
        space: u64, 
        owner: &Pubkey
    ) -> impl std::future::Future<Output = Result<Keypair, BanksClientError>> {
        async move {
            let new_keypair: Keypair = to_keypair.unwrap_or(Keypair::new());

            let tx: Transaction = system_transaction::create_account(
                from_keypair, 
                &new_keypair, 
                *recent_blockhash, 
                lamports, 
                space, 
                owner
            );

            self.process_transaction(tx).await?;

            Ok(new_keypair)
        }
    }
}

pub fn derive_ata(wallet_pkey: &Pubkey, mint_pkey: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            wallet_pkey.as_ref(),
            SPL_TOKEN_2022_ID.as_ref(),
            mint_pkey.as_ref()
        ], 
        &ATA_PROGRAM_ID
    )
}

pub mod spl_token_manipulations {
    use super::*;
    
    /// This struct containts methods used to perform all necessary operations, that some of this program's instruction rely on.
    /// 
    /// You can't invoke `initialize_ido_with_vesting` instruction without providing mint account as well as signer's ATA with some SPL tokens
    /// on it's balance.
    pub struct Prelude<'a> {
        banks_client: &'a BanksClient,
        payer: &'a Keypair,
        payer_pkey: &'a Pubkey,
        latest_blockhash: &'a Hash,
        rent: &'a Rent
    }

    impl<'a> Prelude<'a> {
        pub fn init(
            banks_client: &'a BanksClient, 
            payer: &'a Keypair,
            payer_pkey: &'a Pubkey, 
            latest_blockhash: &'a Hash, 
            rent: &'a Rent
        ) -> Self {
            Self { banks_client, payer, payer_pkey, latest_blockhash, rent }
        }

        pub async fn create_mint_and_funded_ata(&self) -> Result<(Pubkey, Pubkey), BanksClientError> {
            let mint_pkey: Pubkey = self.create_and_intiialize_mint().await?;
            let ata_pda: Pubkey = self.create_and_intiialize_ata(&mint_pkey).await?;
            self.mint_to_ata(&mint_pkey, &ata_pda).await?;

            Ok((mint_pkey, ata_pda))
        }

        pub async fn create_and_intiialize_mint(&self) -> Result<Pubkey, BanksClientError> {
            // 1. create account using system program
            let mint_keypair: Keypair = self.banks_client.send_tx_create_account(
                self.payer, 
                None, 
                self.latest_blockhash, 
                self.rent.minimum_balance(Mint::LEN), 
                Mint::LEN as u64, 
                &SPL_TOKEN_2022_ID
            ).await?;

            // 2. initialize Mint account using SPL program
            // 2.1 craft initialize mint ix & tx
            let initialize_mint_ix: Instruction = spl_token_2022::instruction::initialize_mint(
                &SPL_TOKEN_2022_ID, 
                &mint_keypair.pubkey(), 
                self.payer_pkey, 
                None, 
                9
            ).map_err(|_| BanksClientError::ClientError("Failed to craft InitializeMint ix!"))?;
            let message: Message = Message::new(&[initialize_mint_ix], Some(self.payer_pkey));
            let mut initialize_mint_tx: Transaction = Transaction::new_unsigned(message);
            
            // 2.2 sign & send tx
            initialize_mint_tx.sign(&[self.payer], *self.latest_blockhash);
            self.banks_client.process_transaction(initialize_mint_tx).await?;

            Ok(mint_keypair.pubkey())
        }

        pub async fn create_and_intiialize_ata(&self, mint_pkey: &Pubkey) -> Result<Pubkey, BanksClientError> {
            let (ata_pda, _bump) = derive_ata(self.payer_pkey, mint_pkey);

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
            self.banks_client.process_transaction(create_ata_tx).await?;

            Ok(ata_pda)
        }

        pub async fn mint_to_ata(&self, mint_pkey: &Pubkey, ata_pda: &Pubkey) -> Result<(), BanksClientError> {
            let mut mint_to_ix_payload: Vec<u8> = Vec::with_capacity(9);
            mint_to_ix_payload.push(7);
            mint_to_ix_payload.extend_from_slice(&u64::to_le_bytes(1_000_000_000)); 
            
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
            self.banks_client.process_transaction(mint_to_tx).await?;

            Ok(())
        }
    }
}