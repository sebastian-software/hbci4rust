use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciExecStatus {
    pub success: bool,
    pub job_results: Vec<HbciJobResult>,
    pub messages: Vec<String>,
    pub global_return_values: Vec<HbciReturnValue>,
    pub segment_return_values: Vec<HbciReturnValue>,
}

impl HbciExecStatus {
    pub fn global_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.global_return_values.clone())
    }

    pub fn segment_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.segment_return_values.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HbciStatusCode {
    Ok,
    Unknown,
    Error,
}

impl HbciStatusCode {
    pub const STATUS_OK: i32 = 0;
    pub const STATUS_UNKNOWN: i32 = 1;
    pub const STATUS_ERR: i32 = 2;

    pub fn original_code(&self) -> i32 {
        match self {
            Self::Ok => Self::STATUS_OK,
            Self::Unknown => Self::STATUS_UNKNOWN,
            Self::Error => Self::STATUS_ERR,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciStatus {
    pub return_values: Vec<HbciReturnValue>,
    pub exception_messages: Vec<String>,
}

impl HbciStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_return_values<I>(values: I) -> Self
    where
        I: IntoIterator<Item = HbciReturnValue>,
    {
        Self {
            return_values: values.into_iter().collect(),
            exception_messages: Vec::new(),
        }
    }

    pub fn add_return_value(&mut self, value: HbciReturnValue) {
        self.return_values.push(value);
    }

    pub fn add_exception_message(&mut self, message: impl Into<String>) {
        self.exception_messages.push(message.into());
    }

    pub fn has_exceptions(&self) -> bool {
        !self.exception_messages.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.return_values.iter().any(HbciReturnValue::is_error)
    }

    pub fn has_warnings(&self) -> bool {
        self.return_values.iter().any(HbciReturnValue::is_warning)
    }

    pub fn has_success(&self) -> bool {
        self.return_values.iter().any(HbciReturnValue::is_success)
    }

    pub fn errors(&self) -> Vec<&HbciReturnValue> {
        self.return_values
            .iter()
            .filter(|value| value.is_error())
            .collect()
    }

    pub fn warnings(&self) -> Vec<&HbciReturnValue> {
        self.return_values
            .iter()
            .filter(|value| value.is_warning())
            .collect()
    }

    pub fn successes(&self) -> Vec<&HbciReturnValue> {
        self.return_values
            .iter()
            .filter(|value| value.is_success())
            .collect()
    }

    pub fn status_code(&self) -> HbciStatusCode {
        if self.has_exceptions() || self.has_errors() {
            HbciStatusCode::Error
        } else if self.has_success() || self.has_warnings() {
            HbciStatusCode::Ok
        } else {
            HbciStatusCode::Unknown
        }
    }

    pub fn is_ok(&self) -> bool {
        self.status_code() == HbciStatusCode::Ok
    }

    pub fn error_string(&self) -> String {
        let mut lines = self.exception_messages.clone();
        lines.extend(self.errors().into_iter().map(ToString::to_string));
        lines.join("\n")
    }
}

impl Display for HbciStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut first = true;

        for message in &self.exception_messages {
            write_status_line(formatter, &mut first, message)?;
        }
        for value in self.errors() {
            write_status_line(formatter, &mut first, &value.to_string())?;
        }
        for value in self.warnings() {
            write_status_line(formatter, &mut first, &value.to_string())?;
        }
        for value in self.successes() {
            write_status_line(formatter, &mut first, &value.to_string())?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciJobResult {
    pub job_name: String,
    pub success: bool,
    pub raw_response: Option<String>,
    pub return_values: Vec<HbciReturnValue>,
    pub result: Option<HbciJobResultData>,
}

impl HbciJobResult {
    pub fn job_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.return_values.clone())
    }

    pub fn is_ok_with_global_status(&self, global_status: &HbciStatus) -> bool {
        let job_status = self.job_status();
        global_status.status_code() != HbciStatusCode::Error
            && job_status.status_code() != HbciStatusCode::Error
            && (global_status.status_code() != HbciStatusCode::Unknown
                || job_status.status_code() != HbciStatusCode::Unknown)
    }
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
        self.to_string()
    }
}

impl Display for HbciReturnValue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.code, self.text)?;
        for param in &self.params {
            write!(formatter, " p:{param}")?;
        }

        if let Some(segment_ref) = &self.segment_ref {
            write!(formatter, " ({segment_ref}")?;
            if let Some(data_ref) = &self.data_ref {
                write!(formatter, ":{data_ref}")?;
            }
            formatter.write_str(")")?;
        }

        Ok(())
    }
}

fn write_status_line(formatter: &mut Formatter<'_>, first: &mut bool, line: &str) -> fmt::Result {
    if !*first {
        formatter.write_str("\n")?;
    }
    formatter.write_str(line)?;
    *first = false;
    Ok(())
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrSaldoReq {
    pub entries: Vec<GvrSaldoReqInfo>,
}

impl Display for GvrSaldoReq {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        for (index, entry) in self.entries.iter().enumerate() {
            if index != 0 {
                formatter.write_str("\n")?;
            }
            write!(formatter, "{entry}")?;
        }

        Ok(())
    }
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

impl Display for GvrSaldoReqInfo {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        writeln!(formatter, "Konto: {}", self.konto)?;
        write!(formatter, "  Gebucht: {}", self.ready)?;

        if let Some(unready) = &self.unready {
            write!(formatter, "\n  Pending: {unready}")?;
        }
        if let Some(kredit) = &self.kredit {
            write!(formatter, "\n  Kredit: {kredit}")?;
        }
        if let Some(available) = &self.available {
            write!(formatter, "\n  Verfügbar: {available}")?;
        }
        if let Some(used) = &self.used {
            write!(formatter, "\n  Benutzt: {used}")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for Konto {
    fn default() -> Self {
        Self {
            country: None,
            blz: None,
            number: None,
            subnumber: None,
            bic: None,
            iban: None,
            customer_id: None,
            name: None,
            name2: None,
            acctype: None,
            account_type: None,
            curr: Some("EUR".to_owned()),
            limit: None,
            allowed_gvs: Vec::new(),
        }
    }
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

impl Display for Konto {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if let Some(account_type) = &self.account_type {
            write!(formatter, "{account_type} ")?;
        }
        if let Some(name) = &self.name {
            write!(formatter, "{name} ")?;
        }
        if let Some(name2) = &self.name2 {
            write!(formatter, "{name2} ")?;
        }
        if let Some(number) = &self.number {
            write!(formatter, "{number}")?;
        }
        if let Some(subnumber) = &self.subnumber {
            write!(formatter, "/{subnumber}")?;
        }

        write!(formatter, " ")?;

        if let Some(blz) = &self.blz {
            write!(formatter, "BLZ {blz} () ")?;
        }
        if let Some(bic) = &self.bic {
            write!(formatter, "BIC {bic} ")?;
        }
        if let Some(iban) = &self.iban {
            write!(formatter, "IBAN {iban} ")?;
        }
        if let Some(country) = &self.country {
            write!(formatter, "[{country}] ")?;
        }
        if let Some(curr) = &self.curr {
            write!(formatter, "({curr})")?;
        }

        Ok(())
    }
}

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

impl Display for Limit {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self.limit_type.as_str() {
            Self::TYPE_SINGLE => write!(formatter, "Einzellimit")?,
            Self::TYPE_DAILY => write!(formatter, "Tageslimit")?,
            Self::TYPE_WEEKLY => write!(formatter, "Wochenlimit")?,
            Self::TYPE_MONTHLY => write!(formatter, "Monatslimit")?,
            Self::TYPE_TIME => write!(
                formatter,
                "Zeitliches Limit ({} Tage)",
                self.days.unwrap_or_default()
            )?,
            _ => {}
        }

        write!(
            formatter,
            ": {}",
            self.value
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "null".to_owned())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saldo {
    pub value: Value,
    pub date: Option<String>,
    pub time: Option<String>,
}

impl Display for Saldo {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match (self.date.as_deref(), self.time.as_deref()) {
            (Some(date), Some(time)) => write!(formatter, "{date} {time}")?,
            (Some(date), None) => write!(formatter, "{date}")?,
            (None, Some(time)) => write!(formatter, "{time}")?,
            (None, None) => write!(formatter, "null")?,
        }

        write!(formatter, " {}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value {
    pub value: String,
    pub curr: Option<String>,
}

impl Display for Value {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} {}",
            format_value_amount(&self.value),
            self.curr.as_deref().unwrap_or("null")
        )
    }
}

fn format_value_amount(value: &str) -> String {
    let compact = value.chars().filter(|ch| *ch != ' ').collect::<String>();
    let (negative, unsigned) = strip_sign(&compact);
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));

    if !integer.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
        || fraction.len() > 2
        || (integer.is_empty() && fraction.is_empty())
    {
        return compact;
    }

    let integer = trimmed_integer_part(integer);
    let mut formatted = String::new();
    if negative {
        formatted.push('-');
    }
    formatted.push_str(&integer);
    formatted.push('.');
    formatted.push_str(fraction);
    for _ in fraction.len()..2 {
        formatted.push('0');
    }

    formatted
}

fn strip_sign(value: &str) -> (bool, &str) {
    if let Some(unsigned) = value.strip_prefix('-') {
        (true, unsigned)
    } else if let Some(unsigned) = value.strip_prefix('+') {
        (false, unsigned)
    } else {
        (false, value)
    }
}

fn trimmed_integer_part(value: &str) -> String {
    let trimmed = value.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}
