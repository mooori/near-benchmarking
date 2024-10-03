use std::path::PathBuf;

use clap::Args;
use near_crypto::{InMemorySigner, KeyType, SecretKey, Signer};
use near_jsonrpc_client::{methods::send_tx::RpcSendTransactionRequest, JsonRpcClient};
use near_primitives::{
    transaction::SignedTransaction,
    types::{AccountId, BlockReference, Finality, FunctionArgs},
    views::TxExecutionStatus,
};

use near_ops::{
    account::Account,
    contract::read_wasm_bytes,
    rpc::{assert_transaction_and_receipts_success, get_block},
};

#[derive(Args, Debug)]
pub struct CreateContractArgs {
    /// TODO try to have single arg for all commands
    #[arg(long)]
    pub rpc_url: String,
    #[arg(long)]
    pub signer_key_path: PathBuf,
    /// Starting nonce > current_nonce to send transactions to create sub accounts.
    // TODO remove this field and get nonce from rpc
    #[arg(long, default_value_t = 1)]
    pub nonce: u64,
    /// Must be a subaccount of the signer.
    #[arg(long)]
    pub new_account_id: AccountId,
    #[arg(long)]
    pub deposit: u128,
    /// Directory where data of the new account is stored. The contract is deployed to that account.
    #[arg(long)]
    pub user_data_dir: PathBuf,
    #[arg(long)]
    pub wasm_path: PathBuf,
}

pub async fn create_contract(args: &CreateContractArgs) -> anyhow::Result<()> {
    let signer = InMemorySigner::from_file(&args.signer_key_path)?;

    let client = JsonRpcClient::connect(&args.rpc_url);
    // The block hash included in a transaction affects the duration for which it is valid.
    // Benchmarks are expected to run ~30-60 minutes. Hence using any recent hash should be
    // sufficient to create valid transactions.
    let latest_block_hash = get_block(&client, BlockReference::Finality(Finality::Final))
        .await?
        .header
        .hash;

    let sub_account_key = SecretKey::from_random(KeyType::ED25519);

    let transaction = SignedTransaction::create_contract(
        args.nonce,
        signer.account_id.clone(),
        args.new_account_id.clone(),
        read_wasm_bytes(&args.wasm_path)?,
        args.deposit,
        sub_account_key.public_key().clone(),
        &Signer::from(signer),
        latest_block_hash,
    );

    let request = RpcSendTransactionRequest {
        signed_transaction: transaction,
        wait_until: TxExecutionStatus::ExecutedOptimistic,
    };
    let response = client.call(request).await?;
    assert_transaction_and_receipts_success(&response);

    let account = Account::new(args.new_account_id.clone(), sub_account_key, 0);
    account.write_to_dir(&args.user_data_dir)?;

    Ok(())
}

#[derive(Args, Debug)]
pub struct CallContractArgs {
    /// TODO try to have single arg for all commands
    #[arg(long)]
    pub rpc_url: String,
    #[arg(long)]
    pub signer_key_path: PathBuf,
    /// Starting nonce > current_nonce to send transactions to create sub accounts.
    // TODO remove this field and get nonce from rpc
    #[arg(long, default_value_t = 1)]
    pub nonce: u64,
    #[arg(long)]
    pub receiver_id: AccountId,
    #[arg(long)]
    pub method_name: String,
    /// A that represents a json object.
    #[arg(long)]
    pub args: String,
    #[arg(long)]
    pub gas: u64,
    #[arg(long)]
    pub deposit: u128,
}

pub async fn call_contract(args: &CallContractArgs) -> anyhow::Result<()> {
    let signer = InMemorySigner::from_file(&args.signer_key_path)?;

    let client = JsonRpcClient::connect(&args.rpc_url);
    // The block hash included in a transaction affects the duration for which it is valid.
    // Benchmarks are expected to run ~30-60 minutes. Hence using any recent hash should be
    // sufficient to create valid transactions.
    let latest_block_hash = get_block(&client, BlockReference::Finality(Finality::Final))
        .await?
        .header
        .hash;

    // TODO read+write nonce to signer's account file

    // Validate args.args is string that represents a json object.
    let args_json: serde_json::Value = serde_json::from_str(&args.args)?;
    matches!(args_json, serde_json::Value::Object(_));
    let function_args = args.args.clone().into_bytes();

    let transaction = SignedTransaction::call(
        args.nonce,
        signer.account_id.clone(),
        args.receiver_id.clone(),
        &Signer::from(signer),
        args.deposit,
        args.method_name.to_string(),
        function_args,
        args.gas,
        latest_block_hash,
    );

    let request = RpcSendTransactionRequest {
        signed_transaction: transaction,
        wait_until: TxExecutionStatus::ExecutedOptimistic,
    };
    let response = client.call(request).await?;
    assert_transaction_and_receipts_success(&response);

    Ok(())
}
