use benchmark::{benchmark_native_transfers, BenchmarkNativeTransferArgs};
use clap::{Parser, Subcommand};

use near_ops::account::CreateAccountArgs;

mod account;
use account::{create_sub_accounts, CreateSubAccountsArgs};
mod benchmark;
mod contract;
use contract::{call_contract, create_contract, CallContractArgs, CreateContractArgs};

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
    /// Creates a sub account of the signer and deploys a contract to it.
    CreateContract(CreateContractArgs),
    CallContract(CallContractArgs),
    BenchmarkNativeTransfers(BenchmarkNativeTransferArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    match &cli.command {
        Commands::CreateAccount(CreateAccountArgs { account_id }) => {
            unimplemented!();
        }
        Commands::CreateSubAccounts(args) => {
            create_sub_accounts(args).await?;
        }
        Commands::CreateContract(args) => {
            create_contract(args).await?;
        }
        Commands::CallContract(args) => {
            call_contract(args).await?;
        }
        Commands::BenchmarkNativeTransfers(args) => {
            benchmark_native_transfers(args).await?;
        }
    }
    Ok(())
}
