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
    pub result: Option<HbciJobResultData>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HbciJobResultData {
    SaldoReq(GvrSaldoReq),
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrSaldoReq {
    pub entries: Vec<GvrSaldoReqInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrSaldoReqInfo {
    pub konto: Konto,
    pub ready: Saldo,
    pub unready: Option<Saldo>,
    pub kredit: Option<Value>,
    pub available: Option<Value>,
    pub used: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Konto {
    pub country: Option<String>,
    pub blz: Option<String>,
    pub number: Option<String>,
    pub subnumber: Option<String>,
    pub bic: Option<String>,
    pub iban: Option<String>,
    pub customer_id: Option<String>,
    pub name: Option<String>,
    pub name2: Option<String>,
    pub acctype: Option<String>,
    #[serde(rename = "type")]
    pub account_type: Option<String>,
    pub curr: Option<String>,
    #[serde(default)]
    pub limit: Option<Limit>,
    #[serde(default)]
    pub allowed_gvs: Vec<String>,
}

impl PartialEq for Konto {
    fn eq(&self, other: &Self) -> bool {
        self.blz == other.blz
            && self.country == other.country
            && self.number == other.number
            && self.subnumber == other.subnumber
            && self.curr == other.curr
            && self.customer_id == other.customer_id
            && self.name == other.name
            && self.name2 == other.name2
            && self.account_type == other.account_type
            && self.bic == other.bic
            && self.iban == other.iban
    }
}

impl Eq for Konto {}

impl Konto {
    pub fn check_iban(&self) -> bool {
        self.iban.as_deref().is_some_and(check_iban_crc)
    }

    pub fn is_sepa_account(&self) -> bool {
        self.bic.as_deref().is_some_and(|value| !value.is_empty())
            && self.iban.as_deref().is_some_and(|value| !value.is_empty())
    }
}

fn check_iban_crc(iban: &str) -> bool {
    if iban.len() < 4 {
        return false;
    }

    let mut remainder = 0u32;
    for byte in iban.as_bytes()[4..].iter().chain(&iban.as_bytes()[..4]) {
        match byte {
            b'0'..=b'9' => {
                remainder = (remainder * 10 + u32::from(byte - b'0')) % 97;
            }
            b'A'..=b'Z' => {
                remainder = (remainder * 100 + u32::from(byte - b'A' + 10)) % 97;
            }
            _ => return false,
        }
    }

    remainder == 1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limit {
    #[serde(rename = "type")]
    pub limit_type: String,
    pub value: Option<Value>,
    pub days: Option<u32>,
}

impl Limit {
    pub const TYPE_SINGLE: &'static str = "E";
    pub const TYPE_DAILY: &'static str = "T";
    pub const TYPE_WEEKLY: &'static str = "W";
    pub const TYPE_MONTHLY: &'static str = "M";
    pub const TYPE_TIME: &'static str = "Z";
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saldo {
    pub value: Value,
    pub date: Option<String>,
    pub time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    pub value: String,
    pub curr: Option<String>,
}
