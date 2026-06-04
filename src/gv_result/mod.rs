use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciExecStatus {
    pub success: bool,
    pub job_results: Vec<HbciJobResult>,
    pub messages: Vec<String>,
    pub global_return_values: Vec<HbciReturnValue>,
    pub segment_return_values: Vec<HbciReturnValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciJobResult {
    pub job_name: String,
    pub success: bool,
    pub raw_response: Option<String>,
    pub return_values: Vec<HbciReturnValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciReturnValue {
    pub code: String,
    pub segment_ref: Option<String>,
    pub data_ref: Option<String>,
    pub text: String,
    pub params: Vec<String>,
}

impl HbciReturnValue {
    pub fn new(code: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            segment_ref: None,
            data_ref: None,
            text: text.into(),
            params: Vec::new(),
        }
    }

    pub fn is_error(&self) -> bool {
        self.code.starts_with('9')
    }

    pub fn is_warning(&self) -> bool {
        self.code.starts_with('3')
    }

    pub fn is_success(&self) -> bool {
        self.code.starts_with('0')
    }

    pub fn is_known_status(&self) -> bool {
        self.is_success() || self.is_warning() || self.is_error()
    }

    pub fn message(&self) -> String {
        let mut message = format!("{}:{}", self.code, self.text);
        for param in &self.params {
            message.push_str(" p:");
            message.push_str(param);
        }

        if let Some(segment_ref) = &self.segment_ref {
            message.push_str(" (");
            message.push_str(segment_ref);
            if let Some(data_ref) = &self.data_ref {
                message.push(':');
                message.push_str(data_ref);
            }
            message.push(')');
        }

        message
    }
}
