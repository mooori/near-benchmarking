use std::time::Instant;

use log::{info, warn};
use near_jsonrpc_client::{
    errors::JsonRpcError,
    methods::tx::{RpcTransactionError, RpcTransactionResponse},
};
use near_primitives::views::TxExecutionStatus;
use tokio::sync::mpsc::Receiver;

use crate::rpc::assert_transaction_and_receipts_success;

pub type RpcCallResult = Result<RpcTransactionResponse, JsonRpcError<RpcTransactionError>>;

pub struct RpcResponseHandler {
    receiver: Receiver<RpcCallResult>,
    /// The `wait_until` value of the transactions whose responses are awaited.
    wait_until: TxExecutionStatus,
    num_expected_responses: u64,
}

impl RpcResponseHandler {
    pub fn new(
        receiver: Receiver<RpcCallResult>,
        wait_until: TxExecutionStatus,
        num_expected_responses: u64,
    ) -> Self {
        Self {
            receiver,
            wait_until,
            num_expected_responses,
        }
    }

    pub async fn handle_all_responses(&mut self) {
        // Start timer after receiving the first response.
        let mut timer: Option<Instant> = None;

        let mut num_received = 0;
        while num_received < self.num_expected_responses {
            let response = match self.receiver.recv().await {
                Some(res) => res,
                None => {
                    warn!(
                        "Expectet {} responses but channel closed after {num_received}",
                        self.num_expected_responses
                    );
                    break;
                }
            };

            num_received += 1;
            if timer.is_none() {
                timer = Some(Instant::now());
            }

            let rpc_response = response.expect("rpc call should succeed");
            assert_transaction_and_receipts_success(&rpc_response);
        }

        if let Some(timer) = timer {
            info!(
                "Received {num_received} tx responses in {:.2} seconds",
                timer.elapsed().as_secs_f64()
            );
        }
    }
}
