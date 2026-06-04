mod replay;

use std::collections::BTreeMap;

use async_trait::async_trait;

use crate::error::{HbciError, HbciErrorKind, HbciResult};

pub use replay::ReplayCommClient;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommRequest {
    pub endpoint: String,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl CommRequest {
    pub fn new(endpoint: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            endpoint: endpoint.into(),
            headers: BTreeMap::new(),
            body: body.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommResponse {
    pub status: u16,
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl CommResponse {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self {
        Self {
            status: 200,
            headers: BTreeMap::new(),
            body: body.into(),
        }
    }
}

#[async_trait]
pub trait CommClient: Clone + Send + Sync + 'static {
    async fn send(&self, request: CommRequest) -> HbciResult<CommResponse>;
}

#[derive(Clone)]
pub struct DefaultCommClient {
    client: reqwest::Client,
}

impl Default for DefaultCommClient {
    fn default() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CommClient for DefaultCommClient {
    async fn send(&self, request: CommRequest) -> HbciResult<CommResponse> {
        let mut builder = self.client.post(&request.endpoint).body(request.body);
        for (key, value) in request.headers {
            builder = builder.header(key, value);
        }

        let response = builder.send().await.map_err(|err| {
            HbciError::with_source(HbciErrorKind::Network, "failed to send FinTS request", err)
        })?;
        let status = response.status().as_u16();

        let mut headers = BTreeMap::new();
        for (key, value) in response.headers() {
            if let Ok(value) = value.to_str() {
                headers.insert(key.as_str().to_owned(), value.to_owned());
            }
        }

        let body = response.bytes().await.map_err(|err| {
            HbciError::with_source(HbciErrorKind::Network, "failed to read FinTS response", err)
        })?;

        Ok(CommResponse {
            status,
            headers,
            body: body.to_vec(),
        })
    }
}
