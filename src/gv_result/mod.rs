use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};

use crate::manager::AccountCrcAlgs;
use serde::{Deserialize, Serialize};

use crate::dialog::KnownReturncode;
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::swift::{get_multi_tag_value, get_one_block, get_tag_value, pack_multi};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciExecStatus {
    pub success: bool,
    pub job_results: Vec<HbciJobResult>,
    pub messages: Vec<String>,
    pub global_return_values: Vec<HbciReturnValue>,
    pub segment_return_values: Vec<HbciReturnValue>,
    pub dialog_statuses: BTreeMap<String, HbciDialogStatus>,
    pub exception_messages: BTreeMap<String, Vec<String>>,
}

impl HbciExecStatus {
    pub fn customer_ids(&self) -> Vec<String> {
        let mut ids = BTreeSet::new();
        ids.extend(self.dialog_statuses.keys().cloned());
        ids.extend(self.exception_messages.keys().cloned());
        ids.into_iter().collect()
    }

    pub fn add_dialog_status(
        &mut self,
        customer_id: impl Into<String>,
        status: Option<HbciDialogStatus>,
    ) {
        let customer_id = customer_id.into();
        if let Some(status) = status {
            self.dialog_statuses.insert(customer_id, status);
        } else {
            self.dialog_statuses.remove(&customer_id);
        }
    }

    pub fn dialog_status(&self, customer_id: &str) -> Option<&HbciDialogStatus> {
        self.dialog_statuses.get(customer_id)
    }

    pub fn dialog_status_list(&self) -> Vec<&HbciDialogStatus> {
        self.dialog_statuses.values().collect()
    }

    pub fn add_exception_message(
        &mut self,
        customer_id: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.exception_messages
            .entry(customer_id.into())
            .or_default()
            .push(message.into());
    }

    pub fn exception_messages(&self, customer_id: &str) -> Option<&[String]> {
        self.exception_messages.get(customer_id).map(Vec::as_slice)
    }

    pub fn is_ok(&self) -> bool {
        if self.has_dialog_data() {
            self.customer_ids()
                .iter()
                .all(|customer_id| self.is_ok_for_customer(customer_id))
        } else {
            self.success
        }
    }

    pub fn is_ok_for_customer(&self, customer_id: &str) -> bool {
        !self.exception_messages.contains_key(customer_id)
            && self
                .dialog_status(customer_id)
                .is_some_and(HbciDialogStatus::is_ok)
    }

    pub fn to_string_for_customer(&self, customer_id: &str) -> String {
        let exception_messages = self
            .exception_messages(customer_id)
            .into_iter()
            .flatten()
            .cloned();
        let status = self.dialog_status(customer_id).map(ToString::to_string);

        joined_status_strings(exception_messages.chain(status))
    }

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
        if self.has_dialog_data() {
            self.dialog_error_string()
        } else {
            self.message_status().error_string()
        }
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

    fn has_dialog_data(&self) -> bool {
        !self.dialog_statuses.is_empty() || !self.exception_messages.is_empty()
    }

    fn dialog_error_string(&self) -> String {
        let customer_ids = self.customer_ids();
        let has_multiple_customer_ids = customer_ids.len() > 1;
        let mut lines = Vec::new();

        for customer_id in customer_ids {
            let mut customer_lines = Vec::new();
            if let Some(exception_messages) = self.exception_messages(&customer_id) {
                customer_lines.extend(exception_messages.iter().cloned());
            }
            if let Some(status) = self.dialog_status(&customer_id) {
                let error_string = status.error_string();
                if !error_string.is_empty() {
                    customer_lines.push(error_string);
                }
            }

            if !customer_lines.is_empty() {
                if has_multiple_customer_ids {
                    lines.push(format!("Dialog for '{customer_id}':"));
                }
                lines.extend(customer_lines);
            }
        }

        lines.join("\n")
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
        if !self.has_dialog_data() {
            return Display::fmt(&self.message_status(), formatter);
        }

        formatter.write_str(
            &self
                .customer_ids()
                .into_iter()
                .map(|customer_id| {
                    format!(
                        "Dialog for '{customer_id}':\n{}",
                        self.to_string_for_customer(&customer_id)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
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
    #[serde(default)]
    pub result_data: BTreeMap<String, String>,
    #[serde(default)]
    pub global_return_values: Vec<HbciReturnValue>,
    pub return_values: Vec<HbciReturnValue>,
    pub result: Option<HbciJobResultData>,
}

impl HbciJobResult {
    pub fn store_result(&mut self, key: impl Into<String>, value: Option<impl Into<String>>) {
        if let Some(value) = value {
            self.result_data.insert(key.into(), value.into());
        }
    }

    pub fn dialog_id(&self) -> Option<&str> {
        self.result_data.get("basic.dialogid").map(String::as_str)
    }

    pub fn msg_num(&self) -> Option<&str> {
        self.result_data.get("basic.msgnum").map(String::as_str)
    }

    pub fn seg_num(&self) -> Option<&str> {
        self.result_data.get("basic.segnum").map(String::as_str)
    }

    pub fn job_id_for_date(&self, yyyymmdd: &str) -> String {
        format!(
            "{}/{}/{}/{}",
            yyyymmdd,
            self.dialog_id().unwrap_or("null"),
            self.msg_num().unwrap_or("null"),
            self.seg_num().unwrap_or("null")
        )
    }

    pub fn ret_number(&self) -> usize {
        self.return_values.len()
    }

    pub fn ret_value(&self, index: usize) -> Option<&HbciReturnValue> {
        self.return_values.get(index)
    }

    pub fn global_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.global_return_values.clone())
    }

    pub fn job_status(&self) -> HbciStatus {
        HbciStatus::from_return_values(self.return_values.clone())
    }

    pub fn is_ok(&self) -> bool {
        self.is_ok_with_global_status(&self.global_status())
    }

    pub fn is_ok_with_global_status(&self, global_status: &HbciStatus) -> bool {
        let job_status = self.job_status();
        global_status.status_code() != HbciStatusCode::Error
            && job_status.status_code() != HbciStatusCode::Error
            && (global_status.status_code() != HbciStatusCode::Unknown
                || job_status.status_code() != HbciStatusCode::Unknown)
    }
}

impl Display for HbciJobResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for (key, value) in &self.result_data {
            write_status_line(formatter, &mut first, &format!("{key} = {value}"))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HbciJobResultData {
    AccInfo(GvrAccInfo),
    DauerEdit(GvrDauerEdit),
    DauerList(GvrDauerList),
    DauerNew(GvrDauerNew),
    SaldoReq(GvrSaldoReq),
    KUms(GvrKUms),
    TanMediaList(GvrTanMediaList),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrAccInfo {
    pub entries: Vec<GvrAccInfoEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrAccInfoEntry {
    pub account: Konto,
    pub account_kind: Option<i32>,
    pub created: Option<String>,
    pub sollzins: Option<String>,
    pub habenzins: Option<String>,
    pub ueberzins: Option<String>,
    pub kredit: Option<Value>,
    pub ref_account: Option<Konto>,
    pub versandart: Option<i32>,
    pub turnus: Option<i32>,
    pub comment: Option<String>,
    pub address: Option<GvrAccInfoAddress>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrAccInfoAddress {
    pub name1: Option<String>,
    pub name2: Option<String>,
    pub street_pf: Option<String>,
    pub plz_ort: Option<String>,
    pub plz: Option<String>,
    pub ort: Option<String>,
    pub country: Option<String>,
    pub tel: Option<String>,
    pub fax: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrDauerList {
    pub entries: Vec<GvrDauerListEntry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrDauerListEntry {
    pub my: Konto,
    pub other: Konto,
    pub value: Option<Value>,
    pub key: Option<String>,
    pub addkey: Option<String>,
    pub usage: Vec<String>,
    pub nextdate: Option<String>,
    pub orderid: Option<String>,
    pub firstdate: Option<String>,
    pub timeunit: Option<String>,
    pub turnus: Option<i32>,
    pub execday: Option<i32>,
    pub exectime: Option<String>,
    pub lastdate: Option<String>,
    pub aussetzung: Option<GvrDauerListAussetzung>,
    pub can_change: bool,
    pub can_skip: bool,
    pub can_delete: bool,
    pub pmtinfid: Option<String>,
    pub purposecode: Option<String>,
    pub sepadescr: Option<String>,
    pub sepapain_raw: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrDauerListAussetzung {
    pub annual: bool,
    pub startdate: Option<String>,
    pub enddate: Option<String>,
    pub number: Option<String>,
    pub newvalue: Option<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrDauerNew {
    pub order_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrDauerEdit {
    pub order_id: Option<String>,
    pub order_id_old: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrTanMediaList {
    pub tan_option: Option<i32>,
    pub media: Vec<GvrTanMediaInfo>,
}

impl GvrTanMediaList {
    pub fn active_media_names(&self) -> Vec<String> {
        self.media
            .iter()
            .filter(|info| info.status.as_deref() == Some("1"))
            .filter_map(|info| info.media_name.as_deref())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrTanMediaInfo {
    pub media_category: Option<String>,
    pub status: Option<String>,
    pub card_number: Option<String>,
    pub card_seq_number: Option<String>,
    pub card_type: Option<i32>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub tan_list_number: Option<String>,
    pub media_name: Option<String>,
    pub mobile_number: Option<String>,
    pub mobile_number_secure: Option<String>,
    pub free_tans: Option<i32>,
    pub last_use: Option<String>,
    pub activated_on: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrKUms {
    buffer_mt940: String,
    buffer_mt942: String,
    tage_mt940: Vec<GvrKUmsBTag>,
    tage_mt942: Vec<GvrKUmsBTag>,
    pub camt_booked: Vec<String>,
    pub camt_not_booked: Vec<String>,
    parsed: bool,
    pub rest_mt940: String,
    pub rest_mt942: String,
}

impl GvrKUms {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn append_mt940_data(&mut self, data: impl AsRef<str>) {
        self.buffer_mt940.push_str(data.as_ref());
    }

    pub fn append_mt942_data(&mut self, data: impl AsRef<str>) {
        self.buffer_mt942.push_str(data.as_ref());
    }

    pub fn get_data_per_day(&mut self) -> &[GvrKUmsBTag] {
        self.verify_mt94x_parsing();
        &self.tage_mt940
    }

    pub fn get_data_per_day_unbooked(&mut self) -> &[GvrKUmsBTag] {
        self.verify_mt94x_parsing();
        &self.tage_mt942
    }

    pub fn get_flat_data(&mut self) -> Vec<&GvrKUmsLine> {
        self.verify_mt94x_parsing();
        self.tage_mt940
            .iter()
            .flat_map(|tag| tag.lines.iter())
            .collect()
    }

    pub fn get_flat_data_unbooked(&mut self) -> Vec<&GvrKUmsLine> {
        self.verify_mt94x_parsing();
        self.tage_mt942
            .iter()
            .flat_map(|tag| tag.lines.iter())
            .collect()
    }

    fn verify_mt94x_parsing(&mut self) {
        if self.parsed {
            return;
        }

        self.parsed = true;
        parse_mt94x(
            &mut self.buffer_mt940,
            &mut self.tage_mt940,
            &mut self.rest_mt940,
        );
        parse_mt94x(
            &mut self.buffer_mt942,
            &mut self.tage_mt942,
            &mut self.rest_mt942,
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrKUmsBTag {
    pub my: Konto,
    pub counter: Option<String>,
    pub start: Option<Saldo>,
    pub start_type: char,
    pub lines: Vec<GvrKUmsLine>,
    pub end: Option<Saldo>,
    pub end_type: char,
}

impl Default for GvrKUmsBTag {
    fn default() -> Self {
        Self {
            my: Konto::default(),
            counter: None,
            start: None,
            start_type: '\0',
            lines: Vec::new(),
            end: None,
            end_type: '\0',
        }
    }
}

impl GvrKUmsBTag {
    pub fn add_line(&mut self, line: GvrKUmsLine) {
        self.lines.push(line);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GvrKUmsLine {
    pub valuta: Option<String>,
    pub bdate: Option<String>,
    pub value: Option<Value>,
    pub is_storno: bool,
    pub saldo: Option<Saldo>,
    pub customerref: Option<String>,
    pub instref: Option<String>,
    pub orig_value: Option<Value>,
    pub charge_value: Option<Value>,
    pub gvcode: Option<String>,
    pub additional: Option<String>,
    pub text: Option<String>,
    pub primanota: Option<String>,
    pub usage: Vec<String>,
    pub other: Option<Konto>,
    pub addkey: Option<String>,
    pub is_sepa: bool,
    pub is_camt: bool,
    pub id: Option<String>,
    pub end_to_end_id: Option<String>,
    pub purposecode: Option<String>,
    pub mandate_id: Option<String>,
}

impl GvrKUmsLine {
    pub fn add_usage(&mut self, usage: Option<String>) {
        if let Some(usage) = usage {
            self.usage.push(usage);
        }
    }
}

fn parse_mt94x(buffer: &mut String, tags: &mut Vec<GvrKUmsBTag>, rest: &mut String) {
    if buffer.is_empty() {
        return;
    }

    while !buffer.is_empty() {
        let Some(block) = get_one_block(buffer) else {
            break;
        };
        tags.push(kums_btag_from_block(&block));
        buffer.drain(..block.len());
    }

    rest.clear();
    rest.push_str(buffer);
}

fn kums_btag_from_block(block: &str) -> GvrKUmsBTag {
    let mut btag = GvrKUmsBTag {
        my: kums_account_from_info(get_tag_value(block, "25", 0).as_deref()),
        counter: get_tag_value(block, "28C", 0),
        ..GvrKUmsBTag::default()
    };

    if let Some(start) = get_tag_value(block, "60F", 0) {
        btag.start = kums_saldo_from_mt940(&start);
        btag.start_type = 'F';
    } else if let Some(start) = get_tag_value(block, "60M", 0) {
        btag.start = kums_saldo_from_mt940(&start);
        btag.start_type = 'M';
    }

    let mut saldo = btag
        .start
        .as_ref()
        .and_then(|saldo| decimal_to_cents(&saldo.value.value))
        .unwrap_or_default();
    let mut ums_counter = 0;
    while let Some(st_ums) = get_tag_value(block, "61", ums_counter) {
        if let Some(mut line) = kums_line_from_mt940(&st_ums, btag.start.as_ref(), &mut saldo) {
            if let Some(st_multi) = get_tag_value(block, "86", ums_counter) {
                kums_apply_mt940_multitag(&mut line, &st_multi);
            }
            btag.add_line(line);
        }
        ums_counter += 1;
    }

    btag.end_type = 'F';
    if let Some(end) = get_tag_value(block, "62F", 0) {
        btag.end = kums_saldo_from_mt940(&end);
        btag.end_type = 'F';
    } else if let Some(end) = get_tag_value(block, "62M", 0) {
        btag.end = kums_saldo_from_mt940(&end);
        btag.end_type = 'M';
    }

    kums_correct_line_balances(&mut btag);

    btag
}

fn kums_line_from_mt940(
    input: &str,
    start: Option<&Saldo>,
    saldo: &mut i64,
) -> Option<GvrKUmsLine> {
    let valuta = input.get(0..6)?.to_owned();
    let bytes = input.as_bytes();
    let mut next;
    let bdate;

    if *bytes.get(6)? > b'9' {
        bdate = start
            .and_then(|saldo| saldo.date.clone())
            .unwrap_or_else(|| valuta.clone());
        next = 6;
    } else {
        bdate = corrected_mt940_booking_date(&valuta, input.get(6..10)?);
        next = 10;
    }

    let is_storno;
    let cd;
    match bytes.get(next).copied()? {
        b'C' | b'D' => {
            is_storno = false;
            cd = bytes[next] as char;
            next += 1;
        }
        _ => {
            is_storno = true;
            cd = *bytes.get(next + 1)? as char;
            next += 2;
        }
    }

    if *bytes.get(next)? > b'9' {
        next += 1;
    }

    let npos = find_from_absolute(input, "N", next)?;
    let raw_amount = input.get(next..npos)?.replace(',', ".");
    let neg_value_indicator = if is_storno { 'C' } else { 'D' };
    let amount = if cd == neg_value_indicator {
        format!("-{raw_amount}")
    } else {
        raw_amount
    };
    let line_value = decimal_to_cents(&amount)?;
    let curr = start
        .and_then(|saldo| saldo.value.curr.clone())
        .unwrap_or_else(|| "EUR".to_owned());
    let value = Value {
        value: cents_to_decimal(line_value),
        curr: Some(curr.clone()),
    };
    next = npos + 4;

    *saldo += line_value;
    let line_saldo = Saldo {
        value: Value {
            value: cents_to_decimal(*saldo),
            curr: Some(curr),
        },
        date: Some(bdate.clone()),
        time: None,
    };

    let customer_end = find_from_absolute(input, "//", next)
        .or_else(|| find_from_absolute(input, "\r\n", next))
        .unwrap_or(input.len());
    let customerref = input.get(next..customer_end).unwrap_or_default().to_owned();
    next = customer_end;

    let mut instref = String::new();
    if input.get(next..next + 2) == Some("//") {
        next += 2;
        let inst_end = find_from_absolute(input, "\r\n", next).unwrap_or(input.len());
        instref = input.get(next..inst_end).unwrap_or_default().to_owned();
        next = inst_end + 2;
    }

    let mut line = GvrKUmsLine {
        valuta: Some(valuta),
        bdate: Some(bdate),
        value: Some(value),
        is_storno,
        saldo: Some(line_saldo),
        customerref: Some(customerref),
        instref: Some(instref),
        ..GvrKUmsLine::default()
    };

    if input.as_bytes().get(next) == Some(&b'\r') {
        next += 2;
        line.orig_value = kums_optional_tagged_value(input, "/OCMT/", next);
        line.charge_value = kums_optional_tagged_value(input, "/CHGS/", next);
    }

    Some(line)
}

fn kums_apply_mt940_multitag(line: &mut GvrKUmsLine, input: &str) {
    let Some(gvcode) = input.get(0..3) else {
        return;
    };
    line.gvcode = Some(gvcode.to_owned());
    let st_multi = pack_multi(input.get(3..).unwrap_or_default());

    if gvcode == "999" {
        line.additional = Some(st_multi);
        return;
    }

    line.is_sepa = gvcode.starts_with('1');
    line.text = get_multi_tag_value(&st_multi, "00");
    line.primanota = get_multi_tag_value(&st_multi, "10");
    for index in 20..30 {
        line.add_usage(get_multi_tag_value(&st_multi, &index.to_string()));
    }

    line.other = kums_other_account_from_multitag(&st_multi, line.is_sepa);
    line.addkey = get_multi_tag_value(&st_multi, "34");
    for index in 60..64 {
        line.add_usage(get_multi_tag_value(&st_multi, &index.to_string()));
    }
}

fn kums_other_account_from_multitag(input: &str, is_sepa: bool) -> Option<Konto> {
    let mut blz = get_multi_tag_value(input, "30").map(trim_after_first_space);
    let mut number = get_multi_tag_value(input, "31");
    let mut name = get_multi_tag_value(input, "32");
    let name2 = get_multi_tag_value(input, "33");

    let has_account_data = blz.is_some() || number.is_some() || name.is_some() || name2.is_some();
    if !has_account_data {
        return None;
    }

    if blz.is_none() {
        blz = Some(String::new());
    }
    if number.is_none() {
        number = Some(String::new());
    }
    if name.is_none() {
        name = Some(String::new());
    }

    let mut account = Konto {
        blz,
        number,
        name,
        name2,
        ..Konto::default()
    };

    if is_sepa {
        account.bic = account.blz.clone();
        account.iban = account.number.clone();
    }

    Some(account)
}

fn trim_after_first_space(input: String) -> String {
    input
        .split_once(' ')
        .map(|(prefix, _)| prefix.to_owned())
        .unwrap_or(input)
}

fn kums_correct_line_balances(btag: &mut GvrKUmsBTag) {
    let Some(end) = &btag.end else {
        return;
    };
    let Some(mut saldo) = decimal_to_cents(&end.value.value) else {
        return;
    };
    let Some(last_line_saldo) = btag
        .lines
        .last()
        .and_then(|line| line.saldo.as_ref())
        .and_then(|saldo| decimal_to_cents(&saldo.value.value))
    else {
        return;
    };

    if last_line_saldo == saldo {
        return;
    }

    let curr = end.value.curr.clone();
    for line in btag.lines.iter_mut().rev() {
        let Some(line_saldo) = line.saldo.as_mut() else {
            return;
        };
        let Some(line_value) = line
            .value
            .as_ref()
            .and_then(|value| decimal_to_cents(&value.value))
        else {
            return;
        };

        line_saldo.value = Value {
            value: cents_to_decimal(saldo),
            curr: curr.clone(),
        };
        saldo -= line_value;
    }
}

fn kums_account_from_info(konto_info: Option<&str>) -> Konto {
    let mut account = Konto {
        blz: Some(String::new()),
        number: Some(String::new()),
        iban: konto_info.map(str::to_owned),
        curr: Some(String::new()),
        ..Konto::default()
    };

    if let Some((blz, number)) = konto_info.and_then(|value| value.split_once('/')) {
        let (number, curr) = split_mt940_number_currency(number);
        account.blz = Some(blz.to_owned());
        account.number = Some(number);
        account.iban = Some(String::new());
        account.curr = Some(curr);
    }

    account
}

fn split_mt940_number_currency(number: &str) -> (String, String) {
    let mut split = number.len();
    while split > 0 {
        let character = number[..split]
            .chars()
            .next_back()
            .expect("split is at a character boundary");
        if character.is_ascii_digit() {
            break;
        }
        split -= character.len_utf8();
    }

    if split < number.len() {
        (number[..split].to_owned(), number[split..].to_owned())
    } else {
        (number.to_owned(), String::new())
    }
}

fn kums_saldo_from_mt940(input: &str) -> Option<Saldo> {
    let credit_debit = input.get(0..1)?;
    let date = input.get(1..7)?;
    let curr = input.get(7..10)?;
    let amount = input
        .get(10..)?
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .replace(',', ".");
    let value = if credit_debit == "D" {
        format!("-{amount}")
    } else {
        amount
    };

    Some(Saldo {
        value: Value {
            value,
            curr: Some(curr.to_owned()),
        },
        date: Some(date.to_owned()),
        time: None,
    })
}

fn kums_optional_tagged_value(input: &str, marker: &str, start: usize) -> Option<Value> {
    let pos = find_from_absolute(input, marker, start)?;
    let curr = input.get(pos + 6..pos + 9)?;
    let amount_start = pos + 9;
    let slashpos = find_from_absolute(input, "/", amount_start).unwrap_or(input.len());
    let amount = input.get(amount_start..slashpos)?.replace(',', ".");
    if amount.is_empty() || decimal_to_cents(&amount).is_none() {
        return None;
    }

    Some(Value {
        value: amount,
        curr: Some(curr.to_owned()),
    })
}

fn find_from_absolute(input: &str, pattern: &str, start: usize) -> Option<usize> {
    input
        .get(start..)?
        .find(pattern)
        .map(|relative| start + relative)
}

fn corrected_mt940_booking_date(valuta: &str, booking_month_day: &str) -> String {
    let Some(valuta_year) = valuta.get(0..2).and_then(|value| value.parse::<i32>().ok()) else {
        return format!(
            "{}{}",
            valuta.get(0..2).unwrap_or_default(),
            booking_month_day
        );
    };
    let Some(valuta_month) = valuta.get(2..4).and_then(|value| value.parse::<u32>().ok()) else {
        return format!("{valuta_year:02}{booking_month_day}");
    };
    let Some(valuta_day) = valuta.get(4..6).and_then(|value| value.parse::<u32>().ok()) else {
        return format!("{valuta_year:02}{booking_month_day}");
    };
    let Some(booking_month) = booking_month_day
        .get(0..2)
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return format!("{valuta_year:02}{booking_month_day}");
    };
    let Some(booking_day) = booking_month_day
        .get(2..4)
        .and_then(|value| value.parse::<u32>().ok())
    else {
        return format!("{valuta_year:02}{booking_month_day}");
    };

    let valuta_days = days_from_civil(2000 + valuta_year, valuta_month, valuta_day);
    let mut booking_year = valuta_year;
    let mut booking_days = days_from_civil(2000 + booking_year, booking_month, booking_day);
    if (booking_days - valuta_days).abs() > 180 {
        if booking_days < valuta_days {
            booking_year += 1;
        } else {
            booking_year -= 1;
        }
        booking_days = days_from_civil(2000 + booking_year, booking_month, booking_day);
    }

    if (booking_days - valuta_days).abs() > 366 {
        return format!("{valuta_year:02}{booking_month_day}");
    }

    format!(
        "{:02}{booking_month:02}{booking_day:02}",
        booking_year.rem_euclid(100)
    )
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = year - i32::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn decimal_to_cents(input: &str) -> Option<i64> {
    let input = input.trim();
    let negative = input.starts_with('-');
    let unsigned = input.strip_prefix(['-', '+']).unwrap_or(input);
    let (integer, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if integer.is_empty() || !integer.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    if fraction.len() > 2 || !fraction.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }

    let integer = integer.parse::<i64>().ok()?;
    let mut cents = integer * 100;
    if !fraction.is_empty() {
        let mut padded = fraction.to_owned();
        while padded.len() < 2 {
            padded.push('0');
        }
        cents += padded.parse::<i64>().ok()?;
    }

    Some(if negative { -cents } else { cents })
}

fn cents_to_decimal(cents: i64) -> String {
    let negative = cents < 0;
    let unsigned = cents.abs();
    let prefix = if negative { "-" } else { "" };

    format!("{prefix}{}.{:02}", unsigned / 100, unsigned % 100)
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
    pub creditorid: Option<String>,
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
            creditorid: None,
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
    AccountCrcAlgs::check_iban(iban)
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
