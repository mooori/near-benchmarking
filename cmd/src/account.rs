use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Args;
use log::info;
use near_crypto::{InMemorySigner, KeyType, SecretKey};
use near_jsonrpc_client::JsonRpcClient;
use near_ops::block_service::BlockService;
use near_ops::rpc_response_handler::{ResponseCheckSeverity, RpcResponseHandler};
use near_ops::{
    account::{new_create_subaccount_actions, Account},
    rpc::{new_request, view_access_key},
};
use near_primitives::views::TxExecutionStatus;
use near_primitives::{
    transaction::{Transaction, TransactionV0},
    types::AccountId,
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
    /// Optional prefix for sub account names to avoid generating accounts that already exist on
    /// subsequent invocations.
    ///
    /// # Example
    ///
    /// The name of the `i`-th sub account will be:
    ///
    /// - `user_<i>.<signer_account_id>` if `sub_account_prefix == None`
    /// - `a_user_<i>.<signer_account_id>` if `sub_account_prefix == Some("a")`
    #[arg(long)]
    pub sub_account_prefix: Option<String>,
    /// Number of sub accounts to create.
    #[arg(long)]
    pub num_sub_accounts: u64,
    /// Amount to deposit with each sub-account.
    #[arg(long)]
    pub deposit: u128,
    #[arg(long)]
    /// Acts as upper bound on the number of concurrently open RPC requests.
    pub channel_buffer_size: usize,
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
    let block_service = Arc::new(BlockService::new(client.clone()).await);
    block_service.clone().start().await;

    let mut interval = time::interval(Duration::from_micros(args.interval_duration_micros));
    let timer = Instant::now();

    let mut sub_accounts: Vec<Account> =
        Vec::with_capacity(args.num_sub_accounts.try_into().unwrap());

    // Before a request is made, a permit to send into the channel is awaited. Hence buffer size
    // limits the number of outstanding requests. This helps to avoid congestion.
    // TODO find reasonable buffer size.
    let (channel_tx, channel_rx) = mpsc::channel(args.channel_buffer_size);

    let wait_until = TxExecutionStatus::ExecutedOptimistic;
    let wait_until_channel = wait_until.clone();
    let num_expected_responses = args.num_sub_accounts;
    let response_handler_task = tokio::task::spawn(async move {
        let mut rpc_response_handler = RpcResponseHandler::new(
            channel_rx,
            wait_until_channel,
            ResponseCheckSeverity::Assert,
            num_expected_responses,
        );
        rpc_response_handler.handle_all_responses().await;
    });

    for i in 0..args.num_sub_accounts {
        let sub_account_key = SecretKey::from_random(KeyType::ED25519);
        let sub_account_id: AccountId = {
            let subname = if let Some(prefix) = &args.sub_account_prefix {
                format!("{prefix}_user_{i}")
            } else {
                format!("user_{i}")
            };
            format!("{subname}.{}", signer.account_id).parse()?
        };
        let tx = Transaction::V0(TransactionV0 {
            signer_id: signer.account_id.clone(),
            public_key: signer.public_key().clone(),
            nonce: args.nonce + i,
            receiver_id: sub_account_id.clone(),
            block_hash: block_service.get_block_hash(),
            actions: new_create_subaccount_actions(
                sub_account_key.public_key().clone(),
                args.deposit,
            ),
        });
        let request = new_request(tx, wait_until.clone(), signer.clone());

        interval.tick().await;
        let client = client.clone();
        // Await permit before sending the request to make channel buffer size a limit for the
        // number of outstanding requests.
        let permit = channel_tx.clone().reserve_owned().await.unwrap();
        // The spawned task starts running immediately. Assume with interval between spanning them
        // this leads to transaction nonces hitting the node in order.
        tokio::spawn(async move {
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

    info!("Querying nonces of newly created sub accounts.");

    // Nonces of new access keys are set by nearcore: https://github.com/near/nearcore/pull/4064
    // Query them from the rpc to write `Accounts` with valid nonces to disk.
    // TODO use `JoinSet`, e.g. by storing accounts in map instead of vec.
    let mut get_access_key_tasks = Vec::with_capacity(sub_accounts.len());
    // Use an interval to avoid overwhelming the node with requests.
    let mut interval = time::interval(Duration::from_micros(150));
    for account in sub_accounts.clone().into_iter() {
        interval.tick().await;
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
