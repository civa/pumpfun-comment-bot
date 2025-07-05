use std::{
    collections::{HashMap, hash_map},
    fs,
    sync::Arc,
};

use rand::seq::IndexedRandom;
use reqwest::{Client, StatusCode, Url, cookie::CookieStore, header::HeaderValue};
use serde_json::json;
use solana_sdk::{loader_upgradeable_instruction, signature::Keypair, signer::Signer};

use crate::generate_wallet::LocalSolanaWallet;
use base58::ToBase58;

#[derive(thiserror::Error, Debug)]
pub enum PumpCommentErr {
    #[error("Error sending network req")]
    ReqwestError(#[from] reqwest::Error),
    #[error("parse url error")]
    ParseUrlError(#[from] url::ParseError),
    #[error("error loading wallets")]
    ErrorLoadingWallets,
    #[error("error loading comments")]
    ErrorLoadingComments,
    #[error("error deserializing")]
    DeserializationErr(#[from] serde_json::Error),
}

#[derive(clap::Args)]
pub struct RunCommentsArgs {
    num: Option<usize>,
}
pub async fn run_comments(opts: RunCommentsArgs) -> Result<(), PumpCommentErr> {
    let mut hashmap_client_storage = HashMap::new();
    let loaded_wallets =
        LocalSolanaWallet::load_wallets().map_err(|_x| PumpCommentErr::ErrorLoadingWallets)?;
    let loaded_wallets_len = loaded_wallets.len();
    info!("Loaded {} wallets from wallets.json", loaded_wallets_len);
    let wallets: Vec<LocalSolanaWallet> = loaded_wallets
        .into_iter()
        .take(opts.num.unwrap_or(loaded_wallets_len))
        .collect();

    for wallet in wallets {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .build()
            .unwrap();
        let login = login(wallet.clone(), &client).await?;
        if login.is_success() {
            info!("{} authenticated successfully to pump.fun", &wallet.address);
            hashmap_client_storage.insert(wallet.address.clone(), client.clone());
        } else {
            error!("{} failed to authenticate to pump.fun", &wallet.address);
        }
    }

    info!("Authentication complete");
    let comments = fs::read_to_string("comments.json");
    let comments = match comments {
        Ok(x) => x,
        Err(x) => {
            error!("{:?}", x);
            return Err(PumpCommentErr::ErrorLoadingComments);
        }
    };
    let comments_vec: Vec<String> = serde_json::from_str(&comments)?;

    info!("Fetched {} comments from comments.json", comments_vec.len());

    info!(
        "Starting comments with {} authenticated clients",
        hashmap_client_storage.len()
    );

    for (address, client) in hashmap_client_storage {
        info!("Commenting with {}", address);
        let random_comment = match comments_vec.choose(&mut rand::rng()) {
            Some(x) => x.clone(),
            None => {
                error!("No comment to send");
                continue;
            }
        };

        let profile = get_profile(&client).await.unwrap();
        if profile.is_success() {
            info!("fetched profile successfully");
        } else {
            error!("could not fetch profile");
            continue;
        }

        let mint = std::env::var("MINT").unwrap();
        let res = comment(&client, &mint, &random_comment).await;
        match res {
            Ok(x) => {
                if x.is_success() {
                    info!("{} commented {} on {}", address, random_comment, mint);
                } else {
                    error!("{} failed to comment on {}", address, mint);
                    error!("error code : {}", x);
                }
            }
            Err(_) => todo!(),
        }
    }

    // get_profile(&client).await;
    Ok(())
}

async fn comment(client: &Client, mint: &str, text: &str) -> Result<StatusCode, PumpCommentErr> {
    let endpoint = "https://frontend-api-v3.pump.fun/replies";
    let json = json!({
        "mint": mint,
        "text": text
    });
    let comment_res = client
        .post(endpoint)
        .header("content-type", "application/json")
        //demostration purpose only 😁
        .header("origin", "https://pump.fun")
        .json(&json)
        .send()
        .await?;

    let status = comment_res.status();
    Ok(status)
}
async fn get_profile(client: &Client) -> Result<StatusCode, PumpCommentErr> {
    let profile_endpoint = "https://frontend-api-v3.pump.fun/auth/my-profile";
    let res = client.get(profile_endpoint).send().await?;
    Ok(res.status())
}
async fn login(wallet: LocalSolanaWallet, client: &Client) -> Result<StatusCode, PumpCommentErr> {
    let now = chrono::Utc::now().timestamp_millis();
    let keypair = Keypair::from_base58_string(&wallet.pk);

    // lets do the big guys signing
    let pump_msg = format!("Sign in to pump.fun: {}", now);
    let signature = keypair.sign_message(pump_msg.as_bytes());
    let base58_signature = bs58::encode(signature.as_array()).into_string();
    let frontend_api = "https://frontend-api-v3.pump.fun/auth/login";
    let login_payload = json!({
        "address": keypair.pubkey().to_string(),
        "signature": base58_signature,
        "timestamp": now
    });

    let response = client
        .post(frontend_api)
        .header("content-type", "application/json")
        //demostration purpose only 😁
        .header("origin", "https://pump.fun")
        .json(&login_payload)
        .send()
        .await?;

    Ok(response.status())
}
