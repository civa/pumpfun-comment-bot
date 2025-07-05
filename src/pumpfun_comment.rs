use std::{
    collections::{HashMap, hash_map},
    sync::Arc,
};

use reqwest::{Client, StatusCode, Url, cookie::CookieStore, header::HeaderValue};
use serde_json::json;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::generate_wallet::LocalSolanaWallet;
use base58::ToBase58;

#[derive(thiserror::Error, Debug)]
pub enum PumpCommentErr {
    #[error("Error sending network req")]
    ReqwestError(#[from] reqwest::Error),
    #[error("parse url error")]
    ParseUrlError(#[from] url::ParseError),
}

pub async fn run_comments() -> Result<(), PumpCommentErr> {
    let mut hashmap_client_storage = HashMap::new();
    // let prepare the cookie jar
    let jar = Arc::new(reqwest::cookie::Jar::default());
    let client = reqwest::Client::builder()
        .cookie_provider(jar.clone())
        .build()
        .unwrap();
    let random = LocalSolanaWallet::load_wallets()
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    hashmap_client_storage.insert(random.address.clone(), client.clone());
    let login = login(random, &client).await?;
    get_profile(&client).await;
    Ok(())
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
