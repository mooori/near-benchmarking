use clap::{Parser, Subcommand};

use near_ops::account::CreateAccountArgs;

mod account;
use account::{create_sub_accounts, CreateSubAccountsArgs};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Creates an account.
    CreateAccount(CreateAccountArgs),
    /// Creates sub accounts for the signer.
    CreateSubAccounts(CreateSubAccountsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::CreateAccount(CreateAccountArgs { account_id }) => {
            println!("{account_id}")
        }
        Commands::CreateSubAccounts(args) => {
            create_sub_accounts(args).await?;
        }
    }
    Ok(())
}
