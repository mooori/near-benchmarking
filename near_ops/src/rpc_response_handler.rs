use near_jsonrpc_client::{
    errors::JsonRpcError,
    methods::tx::{RpcTransactionError, RpcTransactionResponse},
};
use tokio::sync::mpsc::Receiver;

use crate::rpc::assert_transaction_and_receipts_success;

pub type RpcCallResult = Result<RpcTransactionResponse, JsonRpcError<RpcTransactionError>>;

pub struct RpcResponseHandler {
    receiver: Receiver<RpcCallResult>,
    num_expected_responses: u64,
}

impl RpcResponseHandler {
    pub fn new(receiver: Receiver<RpcCallResult>, num_expected_responses: u64) -> Self {
        Self {
            receiver,
            num_expected_responses,
        }
    }

    pub async fn handle_all_responses(&mut self) {
        for _ in 0..self.num_expected_responses {
            let response = match self.receiver.recv().await {
                Some(res) => res,
                None => {
                    println!("Handling fewer responses than expected");
                    break;
                }
            };
            let rpc_response = response.expect("rpc call should succeed");
            assert_transaction_and_receipts_success(&rpc_response);
        }
    }
}
