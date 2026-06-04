use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciExecStatus {
    pub success: bool,
    pub job_results: Vec<HbciJobResult>,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciJobResult {
    pub job_name: String,
    pub success: bool,
    pub raw_response: Option<String>,
}
