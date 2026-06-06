use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::dialog::KnownReturncode;
use crate::error::{HbciError, HbciErrorKind, HbciResult};

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

    pub fn message_status(&self) -> HbciMsgStatus {
        HbciMsgStatus {
            global_status: self.global_status(),
            segment_status: self.segment_status(),
        }
    }

    pub fn error_string(&self) -> String {
        self.message_status().error_string()
    }

    pub fn is_invalid_pin(&self) -> bool {
        self.invalid_pin_code().is_some()
    }

    pub fn invalid_pin_code(&self) -> Option<&HbciReturnValue> {
        self.error_return_values_for_any_code(&KnownReturncode::LIST_AUTH_FAIL)
            .into_iter()
            .next()
    }

    pub fn return_values_for_code(&self, code: KnownReturncode) -> Vec<&HbciReturnValue> {
        self.all_return_values()
            .filter(|value| code.is(value.code.as_str()))
            .collect()
    }

    pub fn return_value_for_code(&self, code: KnownReturncode) -> Option<&HbciReturnValue> {
        self.all_return_values()
            .find(|value| code.is(value.code.as_str()))
    }

    fn all_return_values(&self) -> impl Iterator<Item = &HbciReturnValue> {
        self.global_return_values
            .iter()
            .chain(self.segment_return_values.iter())
    }

    fn error_return_values_for_any_code(&self, codes: &[KnownReturncode]) -> Vec<&HbciReturnValue> {
        self.global_return_values
            .iter()
            .filter(|value| value.is_error())
            .chain(
                self.segment_return_values
                    .iter()
                    .filter(|value| value.is_error()),
            )
            .filter(|value| KnownReturncode::contains(value.code.as_str(), codes))
            .collect()
    }
}

impl Display for HbciExecStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.message_status(), formatter)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciDialogStatus {
    pub message_statuses: Vec<HbciMsgStatus>,
    pub init_status: Option<HbciMsgStatus>,
    pub end_status: Option<HbciMsgStatus>,
}

impl HbciDialogStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_init_status(&mut self, status: HbciMsgStatus) {
        self.init_status = Some(status);
    }

    pub fn set_message_statuses<I>(&mut self, statuses: I)
    where
        I: IntoIterator<Item = HbciMsgStatus>,
    {
        self.message_statuses = statuses.into_iter().collect();
    }

    pub fn set_end_status(&mut self, status: HbciMsgStatus) {
        self.end_status = Some(status);
    }

    pub fn is_ok(&self) -> bool {
        self.init_status.as_ref().is_some_and(HbciMsgStatus::is_ok)
            && self.message_statuses.iter().all(HbciMsgStatus::is_ok)
            && self.end_status.as_ref().is_some_and(HbciMsgStatus::is_ok)
    }

    pub fn has_exceptions(&self) -> bool {
        self.init_status
            .as_ref()
            .is_some_and(HbciMsgStatus::has_exceptions)
            || self
                .message_statuses
                .iter()
                .any(HbciMsgStatus::has_exceptions)
            || self
                .end_status
                .as_ref()
                .is_some_and(HbciMsgStatus::has_exceptions)
    }

    pub fn error_string(&self) -> String {
        let parts = self
            .init_status
            .iter()
            .chain(self.message_statuses.iter())
            .chain(self.end_status.iter())
            .map(HbciMsgStatus::error_string);

        joined_status_strings(parts)
    }
}

impl Display for HbciDialogStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut sections = Vec::with_capacity(self.message_statuses.len() + 2);

        sections.push(format!(
            "{STAT_INIT_LABEL}:\n{}",
            dialog_status_display(self.init_status.as_ref())
        ));

        sections.extend(
            self.message_statuses
                .iter()
                .enumerate()
                .map(|(index, status)| format!("{STAT_MSG_LABEL} #{}:\n{status}", index + 1)),
        );

        sections.push(format!(
            "{STAT_END_LABEL}:\n{}",
            dialog_status_display(self.end_status.as_ref())
        ));

        formatter.write_str(&sections.join("\n"))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciMsgStatus {
    pub global_status: HbciStatus,
    pub segment_status: HbciStatus,
}

impl HbciMsgStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_statuses(global_status: HbciStatus, segment_status: HbciStatus) -> Self {
        Self {
            global_status,
            segment_status,
        }
    }

    pub fn has_exceptions(&self) -> bool {
        self.global_status.has_exceptions()
    }

    pub fn is_ok(&self) -> bool {
        self.global_status.status_code() == HbciStatusCode::Ok
    }

    pub fn error_string(&self) -> String {
        joined_status_strings([
            self.global_status.error_string(),
            self.segment_status.error_string(),
        ])
    }

    pub fn is_invalid_pin(&self) -> bool {
        self.invalid_pin_code().is_some()
    }

    pub fn invalid_pin_code(&self) -> Option<&HbciReturnValue> {
        self.global_status
            .errors()
            .into_iter()
            .chain(self.segment_status.errors())
            .find(|value| {
                KnownReturncode::contains(value.code.as_str(), &KnownReturncode::LIST_AUTH_FAIL)
            })
    }

    pub fn return_values_for_code(&self, code: KnownReturncode) -> Vec<&HbciReturnValue> {
        self.global_status
            .return_values_for_code(code)
            .into_iter()
            .chain(self.segment_status.return_values_for_code(code))
            .collect()
    }

    pub fn return_value_for_code(&self, code: KnownReturncode) -> Option<&HbciReturnValue> {
        self.global_status
            .return_value_for_code(code)
            .or_else(|| self.segment_status.return_value_for_code(code))
    }
}

impl Display for HbciMsgStatus {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&joined_status_strings([
            self.global_status.to_string(),
            self.segment_status.to_string(),
        ]))
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

    pub fn return_values_for_code(&self, code: KnownReturncode) -> Vec<&HbciReturnValue> {
        self.return_values
            .iter()
            .filter(|value| code.is(value.code.as_str()))
            .collect()
    }

    pub fn return_value_for_code(&self, code: KnownReturncode) -> Option<&HbciReturnValue> {
        self.return_values
            .iter()
            .find(|value| code.is(value.code.as_str()))
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
pub struct HbciInstMessage {
    pub subject: String,
    pub text: Option<String>,
}

impl HbciInstMessage {
    pub fn new(subject: impl Into<String>, text: Option<String>) -> Self {
        Self {
            subject: subject.into(),
            text,
        }
    }

    pub fn from_values(values: &BTreeMap<String, String>, header: &str) -> HbciResult<Self> {
        let subject = values
            .get(&format!("{header}.betreff"))
            .cloned()
            .ok_or_else(|| {
                HbciError::new(
                    HbciErrorKind::Protocol,
                    format!("institute message {header} has no subject"),
                )
            })?;
        let text = values.get(&format!("{header}.text")).cloned();

        Ok(Self::new(subject, text))
    }

    pub fn collect_from_values(values: &BTreeMap<String, String>, base: &str) -> Vec<Self> {
        let mut messages = Vec::new();
        let mut index = 0;

        loop {
            let header = counted_header(base, index);
            let Some(subject) = values.get(&format!("{header}.betreff")).cloned() else {
                break;
            };
            let text = values.get(&format!("{header}.text")).cloned();

            messages.push(Self::new(subject, text));
            index += 1;
        }

        messages
    }
}

impl Display for HbciInstMessage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {}",
            self.subject,
            self.text.as_deref().unwrap_or("null")
        )
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HbciReturnValue {
    pub code: String,
    pub segment_ref: Option<String>,
    pub data_ref: Option<String>,
    pub text: String,
    pub params: Vec<String>,
    #[serde(default)]
    pub element: Option<String>,
}

impl HbciReturnValue {
    pub fn new(code: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            segment_ref: None,
            data_ref: None,
            text: text.into(),
            params: Vec::new(),
            element: None,
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

impl PartialEq for HbciReturnValue {
    fn eq(&self, other: &Self) -> bool {
        self.code == other.code
            && self.text == other.text
            && self.segment_ref == other.segment_ref
            && self.data_ref == other.data_ref
    }
}

impl Eq for HbciReturnValue {}

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
            if let Some(element) = &self.element {
                write!(formatter, ": {element}")?;
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

fn joined_status_strings<I>(parts: I) -> String
where
    I: IntoIterator<Item = String>,
{
    parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

const STAT_INIT_LABEL: &str = "DIALOG-INIT";
const STAT_MSG_LABEL: &str = "DIALOG-MSG";
const STAT_END_LABEL: &str = "DIALOG-END";
const STATUS_INFO_UNAVAILABLE: &str = "(not status information available)";

fn dialog_status_display(status: Option<&HbciMsgStatus>) -> String {
    status
        .map(ToString::to_string)
        .unwrap_or_else(|| STATUS_INFO_UNAVAILABLE.to_owned())
}

fn counted_header(base: &str, index: usize) -> String {
    if index == 0 {
        base.to_owned()
    } else {
        format!("{base}_{}", index + 1)
    }
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
