use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use near_jsonrpc_client::methods::send_tx::RpcSendTransactionRequest;
use near_jsonrpc_client::JsonRpcClient;
use near_ops::account::accounts_from_dir;
use near_ops::rpc::{assert_transaction_and_receipts_success, get_block};
use near_primitives::transaction::SignedTransaction;
use near_primitives::types::{BlockReference, Finality};
use near_primitives::views::TxExecutionStatus;
use rand::distributions::{Distribution, Uniform};
use tokio::task::JoinSet;
use tokio::time;

#[derive(Args, Debug)]
pub struct BenchmarkNativeTransferArgs {
    /// TODO try to have single arg for all commands
    #[arg(long)]
    pub rpc_url: String,
    #[arg(long)]
    pub user_data_dir: PathBuf,
    #[arg(long)]
    pub num_transfers: u64,
    /// After each tick (in milliseconds) a transaction is sent. If the hardware cannot keep up
    /// with that, transactions are sent at a slower rate.
    #[arg(long)]
    pub interval_duration_ms: u64,
    #[arg(long)]
    pub amount: u128,
}

pub async fn benchmark_native_transfers(args: &BenchmarkNativeTransferArgs) -> anyhow::Result<()> {
    let mut accounts = accounts_from_dir(&args.user_data_dir)?;
    assert!(accounts.len() >= 2);

    let mut join_set = JoinSet::new();
    let mut interval = time::interval(Duration::from_millis(args.interval_duration_ms));
    let timer = Instant::now();

    let between = Uniform::from(0..accounts.len());
    let mut rng = rand::thread_rng();

    let client = JsonRpcClient::connect(&args.rpc_url);
    // The block hash included in a transaction affects the duration for which it is valid.
    // Benchmarks are expected to run ~30-60 minutes. Hence using any recent hash should be
    // sufficient to create valid transactions.
    let latest_block_hash = get_block(&client, BlockReference::Finality(Finality::Final))
        .await?
        .header
        .hash;

    // TODO create a channel to send tx responses into.

    for i in 0..args.num_transfers {
        let idx_sender = usize::try_from(i % u64::try_from(accounts.len()).unwrap()).unwrap();
        let idx_receiver = {
            let mut idx = between.sample(&mut rng);
            if idx == idx_sender {
                // Avoid creating a transaction where an account sends NEAR to itself.
                // Relies on accounts.len() > 2 (asserted above).
                if idx < accounts.len() - 1 {
                    idx += 1;
                } else {
                    idx = 0
                }
            }
            idx
        };

        let sender = &accounts[idx_sender];
        let receiver = &accounts[idx_receiver];
        let transaction = SignedTransaction::send_money(
            sender.nonce + 1,
            sender.id.clone(),
            receiver.id.clone(),
            &sender.as_signer(),
            args.amount,
            latest_block_hash,
        );
        let request = RpcSendTransactionRequest {
            signed_transaction: transaction,
            wait_until: TxExecutionStatus::ExecutedOptimistic,
        };

        interval.tick().await;
        let client = client.clone();
        join_set.spawn(async move { client.call(request).await });
        if i % 1000 == 0 {
            println!("num txs sent: {}", i);
        }

        let sender = accounts.get_mut(idx_sender).unwrap();
        sender.nonce += 1;
    }

    println!(
        "Sent {} txs in {:.2} seconds",
        args.num_transfers,
        timer.elapsed().as_secs_f64()
    );

    for account in accounts.iter() {
        account.write_to_dir(&args.user_data_dir)?;
    }

    let mut num_joined = 0;
    while let Some(res) = join_set.join_next().await {
        let response = res.expect("join should succeed");
        let rpc_response = response.expect("rpc request should succeed");
        assert_transaction_and_receipts_success(&rpc_response);
        num_joined += 1;
        if num_joined % 1000 == 0 {
            println!("num txs executed optimistically: {num_joined}");
        }
    }

    println!(
        "Optimistically executed {} txs in {:.2} seconds",
        args.num_transfers,
        timer.elapsed().as_secs_f64()
    );

    Ok(())
}
