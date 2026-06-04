use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::comm::{CommClient, CommRequest, CommResponse};
use crate::error::{HbciError, HbciErrorKind, HbciResult};

#[derive(Clone, Default)]
pub struct ReplayCommClient {
    responses: Arc<Mutex<VecDeque<HbciResult<CommResponse>>>>,
    requests: Arc<Mutex<Vec<CommRequest>>>,
}

impl ReplayCommClient {
    pub fn new(responses: impl IntoIterator<Item = HbciResult<CommResponse>>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn push_response(&self, response: HbciResult<CommResponse>) -> HbciResult<()> {
        self.responses
            .lock()
            .map_err(|_| HbciError::new(HbciErrorKind::Network, "replay response lock poisoned"))?
            .push_back(response);
        Ok(())
    }

    pub fn requests(&self) -> HbciResult<Vec<CommRequest>> {
        self.requests
            .lock()
            .map(|requests| requests.clone())
            .map_err(|_| HbciError::new(HbciErrorKind::Network, "replay request lock poisoned"))
    }
}

#[async_trait]
impl CommClient for ReplayCommClient {
    async fn send(&self, request: CommRequest) -> HbciResult<CommResponse> {
        self.requests
            .lock()
            .map_err(|_| HbciError::new(HbciErrorKind::Network, "replay request lock poisoned"))?
            .push(request);

        self.responses
            .lock()
            .map_err(|_| HbciError::new(HbciErrorKind::Network, "replay response lock poisoned"))?
            .pop_front()
            .unwrap_or_else(|| {
                Err(HbciError::new(
                    HbciErrorKind::Network,
                    "replay client has no response for request",
                ))
            })
    }
}
