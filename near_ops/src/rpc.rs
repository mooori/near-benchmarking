use near_crypto::{InMemorySigner, Signer};
use near_jsonrpc_client::{
    methods::{
        block::RpcBlockRequest, send_tx::RpcSendTransactionRequest, tx::RpcTransactionResponse,
    },
    JsonRpcClient,
};
use near_primitives::{
    transaction::Transaction,
    types::{BlockReference, Finality},
    views::{BlockView, ExecutionStatusView, FinalExecutionStatus, TxExecutionStatus},
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
