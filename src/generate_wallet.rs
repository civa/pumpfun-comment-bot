use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Error;
use solana_sdk::{signature::Keypair, signer::Signer};

#[derive(Deserialize, Serialize)]
pub struct LocalSolanaWallet {
    pub address: String,
    pub pk: String,
}

#[derive(clap::Args)]
pub struct GenerateWalletOpts {
    num: usize,
}

impl LocalSolanaWallet {
    pub fn generate_wallets(opts: GenerateWalletOpts) {
        let mut wallets = Vec::with_capacity(opts.num as usize);
        for _i in 0..opts.num {
            let keypair = Keypair::new();
            let wallet = LocalSolanaWallet {
                address: keypair.pubkey().to_string(),
                pk: keypair.to_base58_string(),
            };
            wallets.push(wallet);
        }

        Self::save_wallets(&wallets).unwrap() // #fix
    }

    pub fn save_wallets(wallets: &Vec<LocalSolanaWallet>) -> Result<(), Error> {
        let serialized = serde_json::to_string(wallets)?;
        fs::write("wallets.json", serialized).unwrap(); // fix error handling later
        Ok(())
    }
}
