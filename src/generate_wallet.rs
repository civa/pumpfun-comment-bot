use std::fs;

use rand::seq::IndexedRandom;
use serde::{Deserialize, Serialize};
use serde_json::Error;
use solana_sdk::{signature::Keypair, signer::Signer};
use thiserror::Error;

#[derive(Deserialize, Serialize, Clone)]
pub struct LocalSolanaWallet {
    pub address: String,
    pub pk: String,
}

#[derive(clap::Args)]
pub struct GenerateWalletOpts {
    num: usize,
}

#[derive(Error, Debug)]
pub enum LocalSolanaWalletError {
    #[error("IO Error")]
    Io(#[from] std::io::Error),
    #[error("Serialization Error")]
    Serde(#[from] serde_json::Error),
}
impl LocalSolanaWallet {
    pub fn generate_wallets(opts: GenerateWalletOpts) {
        let wallets = Self::generate_wallets_no_save(opts.num as usize);
        Self::save_wallets(&wallets).unwrap() // #fix
    }
    pub fn generate_wallets_no_save(num: usize) -> Vec<Self> {
        let mut wallets = Vec::with_capacity(num);
        for _i in 0..num {
            let keypair = Keypair::new();
            let wallet = LocalSolanaWallet {
                address: keypair.pubkey().to_string(),
                pk: keypair.to_base58_string(),
            };
            wallets.push(wallet);
        }

        wallets
    }

    pub fn save_wallets(wallets: &Vec<LocalSolanaWallet>) -> Result<(), LocalSolanaWalletError> {
        let serialized = serde_json::to_string(wallets)?;
        fs::write("wallets.json", serialized)?;
        Ok(())
    }
    pub fn load_wallets() -> Result<Vec<LocalSolanaWallet>, LocalSolanaWalletError> {
        let data = fs::read_to_string("wallets.json")?;
        let wallets: Vec<LocalSolanaWallet> = serde_json::from_str(&data)?;
        Ok(wallets)
    }

    pub fn get_random() -> Result<Option<Self>, LocalSolanaWalletError> {
        let wallets = Self::load_wallets()?;
        Ok(wallets.choose(&mut rand::rng()).cloned())
    }
}
