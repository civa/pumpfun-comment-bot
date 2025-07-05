use clap::Parser;
use generate_wallet::GenerateWalletOpts;

pub mod generate_wallet;

#[derive(clap::Parser)]
enum Opts {
    GenerateWallets(GenerateWalletOpts),
}

fn main() {
    let opts = Opts::parse();

    match opts {
        Opts::GenerateWallets(generate_wallet_opts) => {
            generate_wallet::LocalSolanaWallet::generate_wallets(generate_wallet_opts)
        }
    }
}
