use std::{
    fs::{self},
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
                nonce: 0, // This value will be ignored: https://github.com/near/nearcore/pull/4064
                permission: AccessKeyPermission::FullAccess,
            },
            public_key,
        })),
        Action::Transfer(TransferAction { deposit }),
    ]
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Account {
    #[serde(rename = "account_id")]
    pub id: AccountId,
    pub public_key: PublicKey,
    pub secret_key: SecretKey,
    // New transaction must have a nonce bigger than this.
    pub nonce: u64,
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

    pub fn from_file(path: &Path) -> anyhow::Result<Account> {
        let content = fs::read_to_string(path)?;
        let account = serde_json::from_str(&content)?;
        Ok(account)
    }

    pub fn write_to_dir(&self, dir: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string(self)?;
        let mut file_name = self.id.to_string();
        file_name.push_str(".json");
        let file_path = dir.join(file_name);
        fs::write(file_path, json)?;
        Ok(())
    }

    pub fn get_and_bump_nonce(&mut self) -> u64 {
        self.nonce += 1;
        self.nonce
    }
}

/// Tries to deserialize all json files in `dir` as [`Account`].
pub fn accounts_from_dir(dir: &Path) -> anyhow::Result<Vec<Account>> {
    if !dir.is_dir() {
        anyhow::bail!("{:?} is not a directory", dir);
    }

    let mut accounts = vec![];
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let file_extension = path.extension();
        if file_extension == None || file_extension.unwrap() != "json" {
            continue;
        }
        let account = Account::from_file(&path)?;
        accounts.push(account);
    }

    Ok(accounts)
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
