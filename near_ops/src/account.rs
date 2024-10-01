use std::{
    fs::{self, File},
    path::Path,
};

use clap::Args;
use near_crypto::{PublicKey, SecretKey};
use near_primitives::{
    account::{AccessKey, AccessKeyPermission},
    action::{Action, AddKeyAction, CreateAccountAction, TransferAction},
    types::AccountId,
};
use serde::{Deserialize, Serialize};

#[derive(Args, Debug)]
pub struct CreateAccountArgs {
    /// Name of the account to create.
    #[arg(long)]
    pub account_id: String,
}

pub fn new_create_subaccount_actions(public_key: PublicKey, deposit: u128) -> Vec<Action> {
    vec![
        Action::CreateAccount(CreateAccountAction {}),
        Action::AddKey(Box::new(AddKeyAction {
            access_key: AccessKey {
                nonce: 0,
                permission: AccessKeyPermission::FullAccess,
            },
            public_key,
        })),
        Action::Transfer(TransferAction { deposit }),
    ]
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Account {
    id: AccountId,
    public_key: PublicKey,
    secret_key: SecretKey,
    // New transaction must have a nonce bigger than this.
    nonce: u64,
}

impl Account {
    pub fn new(id: AccountId, secret_key: SecretKey, nonce: u64) -> Self {
        Self {
            id,
            public_key: secret_key.public_key(),
            secret_key,
            nonce,
        }
    }

    pub fn write_to_dir(&self, dir: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string(self)?;
        let file_name = dir.join(self.id.to_string());
        fs::write(file_name, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::str::FromStr;

    use near_crypto::{InMemorySigner, SecretKey, Signer};
    use near_jsonrpc_client::{methods::send_tx::RpcSendTransactionRequest, JsonRpcClient};
    use near_primitives::{
        account::{AccessKey, AccessKeyPermission},
        action::{Action, AddKeyAction, CreateAccountAction, TransferAction},
        hash::CryptoHash,
        transaction::{Transaction, TransactionV0},
    };

    use crate::test_utils::connect_workspaces_to_sandbox;

    // TODO make these constants parameters
    const RPC_ADDR: &str = "http://localhost:3030";
    const VALIDATOR_KEY_PATH: &str = ".near-sandbox-home/validator_key.json";

    #[tokio::test]
    async fn test_create_account() -> anyhow::Result<()> {
        let worker = connect_workspaces_to_sandbox(RPC_ADDR, VALIDATOR_KEY_PATH.into()).await?;
        let latest_block = worker.view_block().await?;
        let latest_block_hash = CryptoHash::from_str(&latest_block.hash().to_string()).unwrap();

        let signer = InMemorySigner::from_file(Path::new(VALIDATOR_KEY_PATH))?;
        let new_account_keypair = SecretKey::from_random(near_crypto::KeyType::ED25519);
        /*
        Attempt to create a top level account
        let tx = Transaction::V0(TransactionV0 {
            signer_id: signer.account_id.clone(),
            public_key: signer.public_key.clone(),
            nonce: 4,
            receiver_id: "test.near".parse()?,
            block_hash: latest_block_hash,
            actions: vec![Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: "create_account".to_string(),
                args: json!({
                    "new_account_id": "foobar.near",
                    "new_public_key": new_account_keypair.public_key(),
                })
                .to_string()
                .into_bytes(),
                gas: 300_000_000_000_000,
                deposit: 42_000,
            }))],
        });
        */
        // TODO use helper methods here
        let tx = Transaction::V0(TransactionV0 {
            signer_id: signer.account_id.clone(),
            public_key: signer.public_key.clone(),
            nonce: 7,
            receiver_id: "foo_1.test.near".parse()?,
            block_hash: latest_block_hash,
            actions: vec![
                Action::CreateAccount(CreateAccountAction {}),
                Action::AddKey(Box::new(AddKeyAction {
                    access_key: AccessKey {
                        nonce: 0,
                        permission: AccessKeyPermission::FullAccess,
                    },
                    public_key: new_account_keypair.public_key(),
                })),
                Action::Transfer(TransferAction { deposit: 42 }),
            ],
        });
        println!("tx_hash: {}", tx.get_hash_and_size().0);

        let request = RpcSendTransactionRequest {
            signed_transaction: tx.sign(&Signer::InMemory(signer)),
            wait_until: near_primitives::views::TxExecutionStatus::ExecutedOptimistic,
        };
        let client = JsonRpcClient::connect(RPC_ADDR);
        let response = client.call(request).await?;
        println!("response: {:#?}", response);
        Ok(())
    }
}
