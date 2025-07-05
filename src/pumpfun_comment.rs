use std::{
    collections::{hash_map, HashMap},
    fs,
    sync::Arc, thread, time::{Duration, Instant},
};

use futures::StreamExt;
use rand::seq::IndexedRandom;
use reqwest::{cookie::{CookieStore, Jar}, header::HeaderValue, Client, Proxy, StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::{loader_upgradeable_instruction, signature::Keypair, signer::Signer};
use tokio_tungstenite::{connect_async, tungstenite::client::IntoClientRequest};
use crate::generate_wallet::{GenerateWalletOpts, LocalSolanaWallet};
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
    #[error("")]
    ErrorLoadingWebsocket,
}

#[derive(clap::Args, Clone)]
pub struct RunCommentsArgs {
    #[arg(long)]
    num: Option<usize>,
    #[arg(long, short)]
    random: bool,
    #[arg(short)]
    sleep: Option<u64>,
    #[arg(long, short)]
    mint: String
}
#[derive(clap::Args, Clone)]
pub struct RunCommentsOnNewArgs {
    #[arg(long, short)]
    random: bool,
    #[arg(short)]
    sleep: Option<u64>,
}


#[derive(Deserialize, Serialize, Debug, Clone)]
struct SocketEvent {
    params: Params
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Params {
    pair: Pair
}
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Pair {
    #[serde(alias = "baseToken")]
    base_token: Token 
}
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Token {
    account : String,
}

pub async fn run_comments_on_new(opts: RunCommentsOnNewArgs) -> Result<(), PumpCommentErr> {
    let wss_endpoint = "wss://bot-api.zarp.ai/stream-new-pairs".into_client_request().unwrap();
    info!("Entering stream");
    let (ws_stream, _) = connect_async(wss_endpoint).await.unwrap();
    info!("splitting socket");
    let (_w, mut r) =ws_stream.split();
    info!("Listening to messages");

    let mut tasks = vec![];
    let mut last_comment = Instant::now() - Duration::from_secs(opts.sleep.unwrap_or(60));
    while let Some(Ok((msg))) = r.next().await {
        match msg {
            tokio_tungstenite::tungstenite::Message::Binary(bytes) =>{
                if last_comment.elapsed() < Duration::from_secs(opts.sleep.unwrap_or(60)) {
                    continue;
                }
                info!("Deserializing bytes");
                let deserialized =match  serde_json::from_slice::<SocketEvent>(&bytes) {
                    Ok(x) => x,
                    Err(e) => {
                        println!("{}", e);
                        continue;
                    }
                };

                let pair_to_comment = deserialized.params.pair.base_token.account;
                println!("Checking if to comment on {}", pair_to_comment);

                let child_opts = RunCommentsArgs { num: Some(1), random: false, sleep: Some(0), mint: pair_to_comment };
                let handle = tokio::spawn(run_comments(child_opts));
                tasks.push(handle);
                last_comment = Instant::now();
            },
            _ => todo!()
        }
        
    }

    

    todo!()
}

pub async fn run_comments(opts: RunCommentsArgs) -> Result<(), PumpCommentErr> {
    let mut hashmap_client_storage = HashMap::new();
    let wallets = match opts.random {
        true => {
            LocalSolanaWallet::generate_wallets_no_save(opts.num.unwrap_or(300) as usize)
        },
        false => {
            
    let loaded_wallets =
        LocalSolanaWallet::load_wallets().map_err(|_x| PumpCommentErr::ErrorLoadingWallets)?;
    let loaded_wallets_len = loaded_wallets.len();
    info!("Loaded {} wallets from wallets.json", loaded_wallets_len);
    let wallets: Vec<LocalSolanaWallet> = loaded_wallets
        .into_iter()
        .take(opts.num.unwrap_or(loaded_wallets_len))
        .collect();
    wallets
        }
    };
    for wallet in wallets {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        let username = std::env::var("PROXY_USER").unwrap();
        let password = std::env::var("PROXY_PASS").unwrap();
        let proxy_url = format!("http://user-{}:{}@dc.oxylabs.io:8000", username, password);
        let proxy = Proxy::http(proxy_url).unwrap();
        let client = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .proxy(proxy)
            .build()
            .unwrap();
        let login = login(wallet.clone(), &client).await?;
        if login.is_success() {
            info!("{} authenticated successfully to pump.fun", &wallet.address);
            hashmap_client_storage.insert(wallet.address.clone(), client.clone());
        } else {
            error!("{} failed to authenticate to pump.fun", &wallet.address);
        }
        add_extra_cookies(jar).await;
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

        let mint = opts.mint.clone();
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
            Err(x) => {
                println!("{}", x);
                continue;
            }
        }

        thread::sleep(Duration::from_secs(opts.sleep.unwrap_or(30)));
    }

    // get_profile(&client).await;
    Ok(())
}

async fn add_extra_cookies(jar: Arc<Jar>) {
        jar.add_cookie_str("x-aws-waf-token=f62cdc38-7abd-4220-b902-8b7c3afeb68b:IAoAZO9GAnstAAAA:diUjPoCVcgOlHzVo3xeRUcj5vmJfQLgUG62G7r0ELBDMqyUfFEPc5oIOIJVBesh0UbQWsURVOzwb38x5YFdXEq4Kax4Rprh73Y5fbd2Dqs0wecH20ATR3JxiZFf/7bLkuCxPOrwa4XjOpoDZ5RHVl/NOd9LKGFx9gWQI0EZiqtfRnR9XGLMveV57MJzvTAkrbGRY/Ugfn8igefz4Mys5nwo11c7qimFOllqnV85ks3C+0xwpTJ42mOBK", &Url::parse("https://pump.fun").unwrap()); // todo
}
async fn comment(client: &Client, mint: &str, text: &str) -> Result<StatusCode, PumpCommentErr> {
    let endpoint = "https://frontend-api-v3.pump.fun/replies".to_owned();
    let json = json!({
        "mint": mint,
        "text": text
    });
    let comment_res =  match client
        .post(endpoint)
        // .header("x-aws-waf-token", "")
        .header("user-agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/137.0.0.0 Safari/537.36")
        .header("content-type", "application/json")
        //demostration purpose only 😁
        .header("origin", "https://pump.fun")
        .json(&json)
        .send()
        .await {
            Ok(x) => x,
            Err(x) => {
                println!("{:?}", x);
                return Err(PumpCommentErr::ReqwestError(x))
                
            },
        };

    

    let status = comment_res.status();
    let text = comment_res.text().await;
    println!("{:?}", text);
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
