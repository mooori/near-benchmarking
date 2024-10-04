use std::path::PathBuf;

use clap::Args;
use near_crypto::{InMemorySigner, Signer};
use near_jsonrpc_client::methods::send_tx::RpcSendTransactionRequest;
use near_jsonrpc_client::JsonRpcClient;
use near_ops::account::accounts_from_dir;
use near_ops::rpc::{assert_transaction_and_receipts_success, get_block};
use near_primitives::transaction::SignedTransaction;
use near_primitives::types::{BlockReference, Finality};
use near_primitives::views::TxExecutionStatus;

#[derive(Args, Debug)]
pub struct BenchmarkNativeTransferArgs {
    /// TODO try to have single arg for all commands
    #[arg(long)]
    pub rpc_url: String,
    #[arg(long)]
    pub user_data_dir: PathBuf,
}

pub async fn benchmark_native_transfers(args: &BenchmarkNativeTransferArgs) -> anyhow::Result<()> {
    let idx_sender = 0;
    let mut accounts = accounts_from_dir(&args.user_data_dir)?;
    let sender = &accounts[idx_sender];
    let receiver = &accounts[1];
    println!("sending {} -> {}", sender.id, receiver.id);

    let client = JsonRpcClient::connect(&args.rpc_url);
    // The block hash included in a transaction affects the duration for which it is valid.
    // Benchmarks are expected to run ~30-60 minutes. Hence using any recent hash should be
    // sufficient to create valid transactions.
    let latest_block_hash = get_block(&client, BlockReference::Finality(Finality::Final))
        .await?
        .header
        .hash;

    let tx_nonce = sender.nonce + 1;
    let transaction = SignedTransaction::send_money(
        tx_nonce,
        sender.id.clone(),
        receiver.id.clone(),
        &Signer::from(InMemorySigner::from_secret_key(
            sender.id.clone(),
            sender.secret_key.clone(),
        )),
        1,
        latest_block_hash,
    );

    let request = RpcSendTransactionRequest {
        signed_transaction: transaction,
        wait_until: TxExecutionStatus::ExecutedOptimistic,
    };
    let response = client.call(request).await?;
    assert_transaction_and_receipts_success(&response);

    let sender = accounts.get_mut(idx_sender).unwrap();
    sender.nonce = tx_nonce;
    sender.write_to_dir(&args.user_data_dir)?;

    Ok(())
}
