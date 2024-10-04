use near_crypto::{InMemorySigner, PublicKey, Signer};
use near_jsonrpc_client::{
    methods::{
        block::RpcBlockRequest, query::RpcQueryRequest, send_tx::RpcSendTransactionRequest,
        tx::RpcTransactionResponse,
    },
    JsonRpcClient,
};
use near_jsonrpc_primitives::types::query::QueryResponseKind;
use near_primitives::{
    transaction::Transaction,
    types::{AccountId, BlockReference, Finality},
    views::{
        AccessKeyView, BlockView, ExecutionStatusView, FinalExecutionStatus, QueryRequest,
        TxExecutionStatus,
    },
};

pub fn new_request(transaction: Transaction, signer: InMemorySigner) -> RpcSendTransactionRequest {
    RpcSendTransactionRequest {
        signed_transaction: transaction.sign(&Signer::from(signer)),
        wait_until: TxExecutionStatus::ExecutedOptimistic,
    }
}

pub async fn get_latest_block(client: &JsonRpcClient) -> anyhow::Result<BlockView> {
    get_block(client, BlockReference::Finality(Finality::Final)).await
}

pub async fn get_block(
    client: &JsonRpcClient,
    block_ref: BlockReference,
) -> anyhow::Result<BlockView> {
    let request = RpcBlockRequest {
        block_reference: block_ref,
    };
    let block_view = client.call(request).await?;
    Ok(block_view)
}

pub async fn view_access_key(
    client: &JsonRpcClient,
    account_id: AccountId,
    public_key: PublicKey,
) -> anyhow::Result<AccessKeyView> {
    let request = RpcQueryRequest {
        block_reference: BlockReference::latest(),
        request: QueryRequest::ViewAccessKey {
            account_id,
            public_key,
        },
    };
    let response = client.call(request).await?;
    match response.kind {
        QueryResponseKind::AccessKey(access_key_view) => Ok(access_key_view),
        _ => Err(anyhow::anyhow!("unexpected query response")),
    }
}

/// Asserts a transaction and all its receipts succeeded.
pub fn assert_transaction_and_receipts_success(response: &RpcTransactionResponse) {
    match response.final_execution_status {
        TxExecutionStatus::None
        | TxExecutionStatus::Included
        | TxExecutionStatus::IncludedFinal => panic!(
            "Transaction should have been executed. Instead status is: {:?}",
            response.final_execution_status
        ),
        TxExecutionStatus::ExecutedOptimistic
        | TxExecutionStatus::Executed
        | TxExecutionStatus::Final => {}
    }

    let outcome = response
        .final_execution_outcome
        .clone()
        .expect("there should be an outcome")
        .into_outcome();
    matches!(outcome.status, FinalExecutionStatus::SuccessValue { .. });
    for receipt_outcome in outcome.receipts_outcome.iter() {
        match &receipt_outcome.outcome.status {
            ExecutionStatusView::Unknown => panic!("receipt should have outcome"),
            ExecutionStatusView::Failure(err) => panic!("receipt failed: {}", err),
            ExecutionStatusView::SuccessValue(_) => {}
            ExecutionStatusView::SuccessReceiptId(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::connect_rpc_client;

    #[tokio::test]
    async fn test_get_block() -> anyhow::Result<()> {
        let block_ref = BlockReference::Finality(Finality::Final);
        let client = connect_rpc_client();
        let block_view = get_block(&client, block_ref).await?;
        println!("block_hash: {}", block_view.header.hash);
        Ok(())
    }
}
