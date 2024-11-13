use std::time::Instant;

use log::{info, warn};
use near_jsonrpc_client::{
    errors::JsonRpcError,
    methods::tx::{RpcTransactionError, RpcTransactionResponse},
};
use near_primitives::views::TxExecutionStatus;
use tokio::sync::mpsc::Receiver;

use crate::rpc::check_tx_response;

pub type RpcCallResult = Result<RpcTransactionResponse, JsonRpcError<RpcTransactionError>>;

pub struct RpcResponseHandler {
    receiver: Receiver<RpcCallResult>,
    /// The `wait_until` value passed to transactions.
    wait_until: TxExecutionStatus,
    response_check_severity: ResponseCheckSeverity,
    num_expected_responses: u64,
}

#[derive(Copy, Clone, Debug)]
pub enum ResponseCheckSeverity {
    /// An unexpected response or transaction failure will be logged as warning.
    Log,
    /// An unexpected response or transaction failure will raise a panic.
    Assert,
}

impl RpcResponseHandler {
    pub fn new(
        receiver: Receiver<RpcCallResult>,
        wait_until: TxExecutionStatus,
        response_check_severity: ResponseCheckSeverity,
        num_expected_responses: u64,
    ) -> Self {
        Self {
            receiver,
            wait_until,
            response_check_severity,
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
            check_tx_response(
                rpc_response,
                self.wait_until.clone(),
                self.response_check_severity,
            );
        }

        if let Some(timer) = timer {
            info!(
                "Received {num_received} tx responses in {:.2} seconds",
                timer.elapsed().as_secs_f64()
            );
        }
    }
}
