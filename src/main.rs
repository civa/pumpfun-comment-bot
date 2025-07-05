use clap::Parser;
use generate_wallet::GenerateWalletOpts;
use pumpfun_comment::RunCommentsArgs;
use std::env::Args;
pub mod generate_wallet;
pub mod pumpfun_comment;

#[derive(clap::Parser)]
enum Opts {
    GenerateWallets(GenerateWalletOpts),
    GenerateComments(RunCommentsArgs),
}
#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    match opts {
        Opts::GenerateWallets(generate_wallet_opts) => {
            generate_wallet::LocalSolanaWallet::generate_wallets(generate_wallet_opts)
        }
        Opts::GenerateComments(args) => pumpfun_comment::run_comments(args).await.unwrap(),
    }
}
