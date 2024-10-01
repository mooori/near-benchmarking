use anyhow::Context;
use near_jsonrpc_client::JsonRpcClient;
use near_workspaces::{
    network::{Sandbox, ValidatorKey},
    Worker,
};
use std::path::PathBuf;

// TODO make these constants parameters
const RPC_ADDR: &str = "http://localhost:3030";
const VALIDATOR_KEY_PATH: &str = "./near-sandbox-home";

pub fn connect_rpc_client() -> JsonRpcClient {
    JsonRpcClient::connect(RPC_ADDR)
}

// TODO use localnet, for consistency with how cli will be used
pub async fn connect_workspaces_to_sandbox(
    rpc_address: &str,
    validator_key: PathBuf,
) -> anyhow::Result<Worker<Sandbox>> {
    let worker = near_workspaces::sandbox()
        .rpc_addr(rpc_address)
        .validator_key(ValidatorKey::HomeDir(validator_key.into()))
        .await
        .context("Is there an rpc node listening on {rpc_address}")?;
    Ok(worker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_workspaces_to_sandbox() -> anyhow::Result<()> {
        let _worker = connect_workspaces_to_sandbox(RPC_ADDR, VALIDATOR_KEY_PATH.into()).await?;
        Ok(())
    }
}
