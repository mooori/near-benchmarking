use std::{collections::HashMap, sync::LazyLock};

use log::warn;
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
        AccessKeyView, BlockView, ExecutionStatusView, FinalExecutionOutcomeView,
        FinalExecutionStatus, QueryRequest, TxExecutionStatus,
    },
};

use crate::rpc_response_handler::ResponseCheckSeverity;

pub fn new_request(
    transaction: Transaction,
    wait_until: TxExecutionStatus,
    signer: InMemorySigner,
) -> RpcSendTransactionRequest {
    RpcSendTransactionRequest {
        signed_transaction: transaction.sign(&Signer::from(signer)),
        wait_until,
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

/// Maps `TxExecutionStatus` to integers s.t. higher numbers represent a higher finality.
fn tx_execution_level(status: &TxExecutionStatus) -> u8 {
    match status {
        TxExecutionStatus::None => 0,
        TxExecutionStatus::Included => 1,
        TxExecutionStatus::ExecutedOptimistic => 2,
        TxExecutionStatus::IncludedFinal => 3,
        TxExecutionStatus::Executed => 4,
        TxExecutionStatus::Final => 5,
    }
}

/// Checks the rpc request to send a transaction succeeded. Depending on `wait_until`, the status
/// of receipts might be checked too. Logs warnings on request failures.
///
/// For now, only handling empty transaction success values and not inspecting success values of
/// receipts.
///
/// # Panics
pub fn check_tx_response(
    response: RpcTransactionResponse,
    wait_until: TxExecutionStatus,
    response_check_severity: ResponseCheckSeverity,
) {
    if tx_execution_level(&response.final_execution_status) < tx_execution_level(&wait_until) {
        let msg = format!(
            "got final execution status {:#?}, expected at least {:#?}",
            response.final_execution_status, wait_until
        );
        warn_or_panic(&msg, response_check_severity);
    }

    // Check the outcome, if applicable.
    match response.final_execution_status {
        TxExecutionStatus::None => {
            // The response to a transaction with `wait_until: None` contains no outcome.
            // If that ever changes, the outcome must be checked, hence the assert.
            assert!(response.final_execution_outcome.is_none());
        }
        TxExecutionStatus::Included => {
            unimplemented!("given how transactions are sent, this status is not yet returned")
        }
        TxExecutionStatus::ExecutedOptimistic
        | TxExecutionStatus::IncludedFinal
        | TxExecutionStatus::Executed
        | TxExecutionStatus::Final => {
            // For now, only sending transactions that expect an empty success value.
            check_outcome(
                response,
                FinalExecutionStatus::SuccessValue(vec![]),
                response_check_severity,
            );
        }
    }
}

/// For now not inspecting success values or receipt ids.
fn check_outcome(
    response: RpcTransactionResponse,
    expected_status: FinalExecutionStatus,
    response_check_severity: ResponseCheckSeverity,
) {
    let outcome = response
        .final_execution_outcome
        .expect("response should have an outcome")
        .into_outcome();
    if outcome.status != expected_status {
        let msg = format!(
            "got outcome.status {:#?}, expected {:#?}",
            outcome.status, expected_status
        );
        warn_or_panic(&msg, response_check_severity);
    }

    for receipt_outcome in outcome.receipts_outcome.iter() {
        match &receipt_outcome.outcome.status {
            ExecutionStatusView::Unknown => {
                warn_or_panic("unknown receipt outcome", response_check_severity)
            }
            ExecutionStatusView::Failure(err) => {
                warn_or_panic(&format!("receipt failed: {err}"), response_check_severity)
            }
            ExecutionStatusView::SuccessValue(_) => {}
            ExecutionStatusView::SuccessReceiptId(_) => {}
        }
    }
}

fn warn_or_panic(msg: &str, response_check_severity: ResponseCheckSeverity) {
    match response_check_severity {
        ResponseCheckSeverity::Log => warn!("{msg}"),
        ResponseCheckSeverity::Assert => panic!("{msg}"),
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
        Ok(())
    }
}
