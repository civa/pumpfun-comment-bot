use clap::Parser;
use generate_wallet::GenerateWalletOpts;
pub mod generate_wallet;
pub mod pumpfun_comment;

#[derive(clap::Parser)]
enum Opts {
    GenerateWallets(GenerateWalletOpts),
    GenerateComments,
}
#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    match opts {
        Opts::GenerateWallets(generate_wallet_opts) => {
            generate_wallet::LocalSolanaWallet::generate_wallets(generate_wallet_opts)
        }
        Opts::GenerateComments => pumpfun_comment::run_comments().await,
    }
}
