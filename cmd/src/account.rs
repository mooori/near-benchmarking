use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use log::info;
use near_crypto::{InMemorySigner, KeyType, SecretKey};
use near_jsonrpc_client::JsonRpcClient;
use near_ops::rpc_response_handler::RpcResponseHandler;
use near_ops::{
    account::{new_create_subaccount_actions, Account},
    rpc::{get_block, new_request, view_access_key},
};
use near_primitives::{
    transaction::{Transaction, TransactionV0},
    types::{AccountId, BlockReference, Finality},
};
use tokio::sync::mpsc;
use tokio::time;

#[derive(Args, Debug)]
pub struct CreateSubAccountsArgs {
    /// TODO try to have single arg for all commands
    #[arg(long)]
    pub rpc_url: String,
    #[arg(long)]
    pub signer_key_path: PathBuf,
    /// Starting nonce > current_nonce to send transactions to create sub accounts.
    // TODO remove this field and get nonce from rpc
    #[arg(long, default_value_t = 1)]
    pub nonce: u64,
    /// Number of sub accounts to create.
    #[arg(long)]
    pub num_sub_accounts: u64,
    /// Amount to deposit with each sub-account.
    #[arg(long)]
    pub deposit: u128,
    /// After each tick (in microseconds) a transaction is sent. If the hardware cannot keep up with
    /// that or if the NEAR node is congested, transactions are sent at a slower rate.
    #[arg(long)]
    pub interval_duration_micros: u64,
    /// Directory where created user account data (incl. key and nonce) is stored.
    #[arg(long)]
    pub user_data_dir: PathBuf,
}

pub async fn create_sub_accounts(args: &CreateSubAccountsArgs) -> anyhow::Result<()> {
    let signer = InMemorySigner::from_file(&args.signer_key_path)?;

    let client = JsonRpcClient::connect(&args.rpc_url);
    info!("{:#?}", client.headers());
    // The block hash included in a transaction affects the duration for which it is valid.
    // Benchmarks are expected to run ~30-60 minutes. Hence using any recent hash should be
    // sufficient to create valid transactions.
    let latest_block_hash = get_block(&client, BlockReference::Finality(Finality::Final))
        .await?
        .header
        .hash;

    let mut interval = time::interval(Duration::from_micros(args.interval_duration_micros));
    let timer = Instant::now();

    let mut sub_accounts: Vec<Account> =
        Vec::with_capacity(args.num_sub_accounts.try_into().unwrap());

    // Before a request is made, a permit to send into the channel is awaited. Hence buffer size
    // limits the number of outstanding requests. This helps to avoid congestion.
    // TODO find reasonable buffer size.
    let (channel_tx, channel_rx) = mpsc::channel(1200);

    let num_expected_responses = args.num_sub_accounts;
    let response_handler_task = tokio::task::spawn(async move {
        let mut rpc_response_handler = RpcResponseHandler::new(channel_rx, num_expected_responses);
        rpc_response_handler.handle_all_responses().await;
    });

    for i in 0..args.num_sub_accounts {
        let sub_account_key = SecretKey::from_random(KeyType::ED25519);
        let sub_account_id: AccountId = format!("user_{i}_o.{}", signer.account_id).parse()?;
        let tx = Transaction::V0(TransactionV0 {
            signer_id: signer.account_id.clone(),
            public_key: signer.public_key().clone(),
            nonce: args.nonce + i,
            receiver_id: sub_account_id.clone(),
            block_hash: latest_block_hash.clone(),
            actions: new_create_subaccount_actions(
                sub_account_key.public_key().clone(),
                args.deposit,
            ),
        });
        let request = new_request(tx, signer.clone());

        interval.tick().await;
        let client = client.clone();
        let channel_tx = channel_tx.clone();
        // The spawned task starts running immediately. Assume with interval between spanning them
        // this leads to transaction nonces hitting the node in order.
        tokio::spawn(async move {
            // Await permit before sending the request to make channel buffer size a limit for the
            // number of outstanding requests.
            let permit = channel_tx.reserve().await.unwrap();
            let res = client.call(request).await;
            permit.send(res);
        });

        sub_accounts.push(Account::new(sub_account_id, sub_account_key, 0));
    }

    info!(
        "Sent {} txs in {:.2} seconds",
        args.num_sub_accounts,
        timer.elapsed().as_secs_f64()
    );

    // Ensure all rpc responses are handled.
    response_handler_task
        .await
        .expect("response handler tasks should succeed");

    // Nonces of new access keys are set by nearcore: https://github.com/near/nearcore/pull/4064
    // Query them from the rpc to write `Accounts` with valid nonces to disk.
    // TODO use `JoinSet`, e.g. by storing accounts in map instead of vec.
    let mut get_access_key_tasks = Vec::with_capacity(sub_accounts.len());
    for account in sub_accounts.clone().into_iter() {
        let client = client.clone();
        get_access_key_tasks.push(tokio::spawn(async move {
            view_access_key(&client, account.id.clone(), account.public_key.clone()).await
        }))
    }

    for (i, task) in get_access_key_tasks.into_iter().enumerate() {
        let response = task.await.expect("join should succeed");
        let nonce = response?.nonce;
        let account = sub_accounts.get_mut(i).unwrap();
        account.nonce = nonce;
        account.write_to_dir(&args.user_data_dir)?;
    }

    Ok(())
}
