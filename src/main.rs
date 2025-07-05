#[macro_use]
extern crate tracing;
use clap::Parser;
use generate_wallet::GenerateWalletOpts;
use pumpfun_comment::{RunCommentsArgs, RunCommentsOnNewArgs};
use std::env::Args;
pub mod generate_wallet;
pub mod pumpfun_comment;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(clap::Parser)]
enum Opts {
    GenerateWallets(GenerateWalletOpts),
    GenerateComments(RunCommentsArgs),
    GenerateCommentsOnNew(RunCommentsOnNewArgs),
}
#[tokio::main]
async fn main() {
    let filter = EnvFilter::builder()
        .with_default_directive(tracing::level_filters::LevelFilter::INFO.into())
        .from_env()
        .unwrap();
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .init();
    let opts = Opts::parse();
    match opts {
        Opts::GenerateWallets(generate_wallet_opts) => {
            generate_wallet::LocalSolanaWallet::generate_wallets(generate_wallet_opts)
        }
        Opts::GenerateComments(args) => pumpfun_comment::run_comments(args).await.unwrap(),
        Opts::GenerateCommentsOnNew(args) => {
            pumpfun_comment::run_comments_on_new(args).await.unwrap()
        }
    }
}
