use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::callback::{CallbackDataType, CallbackEvent, CallbackReason, HbciCallback};
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv_result::{Konto, Value};
use crate::protocol::normalize_iso_date;
use crate::sepa::{
    CAMT_052_001_01_URN, PAIN_001_001_02_URN, PAIN_008_001_01_URN,
    generate_pain_001_001_02_transfer, generate_pain_008_001_01_direct_debit,
};
use crate::tools::Properties;

pub const PINTAN_JOB_NAMES: &[&str] = &[
    "AccInfo",
    "CardList",
    "DauerDel",
    "DauerEdit",
    "DauerLastSEPAList",
    "DauerLastSEPANew",
    "DauerList",
    "DauerNew",
    "DauerSEPADel",
    "DauerSEPAEdit",
    "DauerSEPAList",
    "DauerSEPANew",
    "FestCondList",
    "FestList",
    "InfoList",
    "InfoOrder",
    "InstUebSEPA",
    "KUmsAll",
    "KUmsAllCamt",
    "KUmsNew",
    "KUmsZeitSEPA",
    "Kontoauszug",
    "KontoauszugPdf",
    "LastB2BSEPA",
    "LastCOR1SEPA",
    "LastSEPA",
    "MultiLastB2BSEPA",
    "MultiLastCOR1SEPA",
    "MultiLastSEPA",
    "MultiUebSEPA",
    "Receipt",
    "SEPAInfo",
    "SaldoReq",
    "SaldoReqAll",
    "Status",
    "TAN2Step",
    "TANList",
    "TANMediaList",
    "TermMultiUebSEPA",
    "TermUeb",
    "TermUebDel",
    "TermUebEdit",
    "TermUebList",
    "TermUebSEPA",
    "TermUebSEPADel",
    "TermUebSEPAEdit",
    "TermUebSEPAList",
    "Ueb",
    "UebBZU",
    "UebEil",
    "UebForeign",
    "UebSEPA",
    "Umb",
    "UmbSEPA",
    "VoP",
    "VoPAuth",
    "WPDepotList",
    "WPDepotUms",
];

pub(crate) const CLASSIC_USAGE_LINE_COUNT: usize = 14;

#[derive(Debug, Clone, Default)]
pub struct JobRegistry {
    names: BTreeSet<&'static str>,
}

impl JobRegistry {
    pub fn pintan() -> Self {
        Self {
            names: PINTAN_JOB_NAMES.iter().copied().collect(),
        }
    }

    pub fn contains(&self, name: &str) -> bool {
        self.names.contains(name)
    }

    pub fn new_job(&self, name: &str) -> HbciResult<HbciJob> {
        if self.contains(name) {
            Ok(HbciJob::new(name))
        } else {
            Err(HbciError::new(
                HbciErrorKind::Unsupported,
                format!("unsupported or out-of-scope job: {name}"),
            ))
        }
    }

    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.names.iter().copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciJob {
    name: String,
    params: BTreeMap<String, String>,
    #[serde(default)]
    lowlevel_params: BTreeMap<String, String>,
    #[serde(default)]
    constraints: Vec<HbciJobConstraint>,
}

impl HbciJob {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            constraints: constraints_for_job(&name),
            lowlevel_params: BTreeMap::new(),
            name,
            params: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        let value = value.into();

        if self.name == "Status"
            && name == "jobid"
            && let Ok(date) = status_jobid_date(&value)
        {
            self.set_frontend_and_lowlevel_param("startdate", date.clone());
            self.set_frontend_and_lowlevel_param("enddate", date);
        }

        self.params.insert(name, value);
    }

    pub fn set_param_int(&mut self, name: impl Into<String>, value: i32) {
        self.set_param(name, value.to_string());
    }

    pub fn try_set_param(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> HbciResult<()> {
        let name = name.into();
        let value = value.into();

        if !self.accepts_param(&name) {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("job parameter {name} is not accepted by {}", self.name),
            ));
        }

        if value.is_empty() {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("job parameter {name} must not be empty for {}", self.name),
            ));
        }

        if self.name == "UebBZU" && name == "bzudata" {
            validate_bzu_data(&value)?;
        }

        if self.name == "Status" && name == "jobid" {
            self.set_status_jobid_param(value)?;
            return Ok(());
        }

        self.set_frontend_and_lowlevel_param(name, value);
        Ok(())
    }

    pub fn try_set_param_int(&mut self, name: impl Into<String>, value: i32) -> HbciResult<()> {
        self.try_set_param(name, value.to_string())
    }

    pub fn try_set_param_date(
        &mut self,
        name: impl Into<String>,
        date: impl AsRef<str>,
    ) -> HbciResult<()> {
        self.try_set_param(name, normalize_iso_date(date.as_ref())?)
    }

    pub fn try_set_indexed_param(
        &mut self,
        name: impl Into<String>,
        index: usize,
        value: impl Into<String>,
    ) -> HbciResult<()> {
        let name = name.into();
        let value = value.into();

        if !self.accepts_param(&name) {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("job parameter {name} is not accepted by {}", self.name),
            ));
        }

        if value.is_empty() {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("job parameter {name} must not be empty for {}", self.name),
            ));
        }

        if !self.accepts_indexed_param(&name) {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("job parameter {name} is not indexed by {}", self.name),
            ));
        }

        self.set_indexed_lowlevel_params_for_frontend(&name, index, &value);
        Ok(())
    }

    pub fn try_set_indexed_param_date(
        &mut self,
        name: impl Into<String>,
        index: usize,
        date: impl AsRef<str>,
    ) -> HbciResult<()> {
        self.try_set_indexed_param(name, index, normalize_iso_date(date.as_ref())?)
    }

    pub fn try_set_indexed_param_value(
        &mut self,
        name: &str,
        index: usize,
        value: &Value,
    ) -> HbciResult<()> {
        self.try_set_optional_indexed_structured_param(
            name,
            index,
            "value",
            Some(value.value.as_str()),
        )?;
        self.try_set_optional_indexed_structured_param(name, index, "curr", value.curr.as_deref())
    }

    pub fn try_set_indexed_param_account(
        &mut self,
        name: &str,
        index: usize,
        account: &Konto,
    ) -> HbciResult<()> {
        self.try_set_optional_indexed_structured_param(
            name,
            index,
            "country",
            account.country.as_deref(),
        )?;
        self.try_set_optional_indexed_structured_param(name, index, "blz", account.blz.as_deref())?;
        self.try_set_optional_indexed_structured_param(
            name,
            index,
            "number",
            account.number.as_deref(),
        )?;
        self.try_set_optional_indexed_structured_param(
            name,
            index,
            "subnumber",
            account.subnumber.as_deref(),
        )?;
        self.try_set_optional_indexed_structured_param(
            name,
            index,
            "name",
            account.name.as_deref(),
        )?;
        self.try_set_optional_indexed_structured_param(
            name,
            index,
            "curr",
            account.curr.as_deref(),
        )?;
        self.try_set_optional_indexed_structured_param(name, index, "bic", account.bic.as_deref())?;
        self.try_set_optional_indexed_structured_param(name, index, "iban", account.iban.as_deref())
    }

    pub fn set_param_account(&mut self, name: &str, account: &Konto) {
        self.set_optional_account_param(name, "country", account.country.as_deref());
        self.set_optional_account_param(name, "blz", account.blz.as_deref());
        self.set_optional_account_param(name, "number", account.number.as_deref());
        self.set_optional_account_param(name, "subnumber", account.subnumber.as_deref());
        self.set_optional_account_param(name, "name", account.name.as_deref());
        self.set_optional_account_param(name, "curr", account.curr.as_deref());
        self.set_optional_account_param(name, "bic", account.bic.as_deref());
        self.set_optional_account_param(name, "iban", account.iban.as_deref());
    }

    pub fn set_param_value(&mut self, name: &str, value: &Value) {
        self.set_optional_structured_param(name, "value", Some(value.value.as_str()));
        self.set_optional_structured_param(name, "curr", value.curr.as_deref());
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    pub fn params(&self) -> &BTreeMap<String, String> {
        &self.params
    }

    pub fn lowlevel_param(&self, name: &str) -> Option<&str> {
        self.lowlevel_params.get(name).map(String::as_str)
    }

    pub fn lowlevel_params(&self) -> &BTreeMap<String, String> {
        &self.lowlevel_params
    }

    pub(crate) fn set_lowlevel_param_if_absent(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) {
        self.lowlevel_params
            .entry(name.into())
            .or_insert_with(|| value.into());
    }

    pub fn constraints(&self) -> &[HbciJobConstraint] {
        &self.constraints
    }

    pub fn constraint(&self, frontend_name: &str) -> Option<&HbciJobConstraint> {
        self.constraints
            .iter()
            .find(|constraint| constraint.frontend_name == frontend_name)
    }

    pub fn accepts_param(&self, frontend_name: &str) -> bool {
        self.constraint(frontend_name).is_some()
    }

    pub fn accepts_indexed_param(&self, frontend_name: &str) -> bool {
        self.constraints
            .iter()
            .any(|constraint| constraint.frontend_name == frontend_name && constraint.indexed)
    }

    pub fn verify_constraints(&mut self) -> HbciResult<BTreeMap<String, String>> {
        self.generate_sepa_pain_if_needed()?;

        let mut lowlevel_params = BTreeMap::new();

        for constraint in self.constraints.clone() {
            if let Some(resolved) = self.resolved_constraint_value(&constraint)? {
                if resolved.persist_destination
                    && !self
                        .lowlevel_params
                        .contains_key(&constraint.destination_name)
                {
                    self.lowlevel_params
                        .insert(constraint.destination_name.clone(), resolved.value.clone());
                }
                lowlevel_params.insert(constraint.destination_name, resolved.value);
            }
        }

        Ok(lowlevel_params)
    }

    fn generate_sepa_pain_if_needed(&mut self) -> HbciResult<()> {
        let Some(lowlevel_segment) = self.sepa_pain_generation_lowlevel_segment() else {
            return Ok(());
        };
        if self.has_sepapain_value(lowlevel_segment) {
            return Ok(());
        }

        let params = self.sepa_generation_params(lowlevel_segment);
        let xml = match self.name.as_str() {
            "LastSEPA" => generate_pain_008_001_01_direct_debit(&params)?,
            _ => generate_pain_001_001_02_transfer(&params)?,
        };
        self.set_frontend_and_lowlevel_param("_sepapain", xml);
        Ok(())
    }

    fn sepa_pain_generation_lowlevel_segment(&self) -> Option<&'static str> {
        match self.name.as_str() {
            "DauerSEPADel" => Some("DauerSEPADel1"),
            "DauerSEPAEdit" => Some("DauerSEPAEdit1"),
            "DauerSEPANew" => Some("DauerSEPANew1"),
            "InstUebSEPA" => Some("InstUebSEPA1"),
            "LastSEPA" => Some("LastSEPA1"),
            "TermUebSEPA" => Some("TermUebSEPA1"),
            "TermUebSEPADel" => Some("TermUebSEPADel1"),
            "TermUebSEPAEdit" => Some("TermUebSEPAEdit1"),
            "UebSEPA" => Some("UebSEPA1"),
            "UmbSEPA" => Some("UmbSEPA1"),
            _ => None,
        }
    }

    fn has_sepapain_value(&self, lowlevel_segment: &str) -> bool {
        self.param("_sepapain")
            .filter(|value| !value.is_empty())
            .is_some()
            || self
                .lowlevel_param(&format!("{lowlevel_segment}.sepapain"))
                .filter(|value| !value.is_empty())
                .is_some()
    }

    fn sepa_generation_params(&self, lowlevel_segment: &str) -> Properties {
        let mut params = Properties::new();

        for (name, value) in &self.params {
            if !value.is_empty() && !name.starts_with('_') {
                params.insert(name.clone(), value.clone());
            }
        }

        let sepa_prefix = format!("{lowlevel_segment}.sepa.");
        let date_name = format!("{lowlevel_segment}.date");
        for (name, value) in &self.lowlevel_params {
            if value.is_empty() {
                continue;
            }

            if let Some(sepa_name) = name.strip_prefix(&sepa_prefix) {
                params.insert(single_transfer_sepa_name(sepa_name), value.clone());
            } else if name == &date_name {
                params
                    .entry("date".to_owned())
                    .or_insert_with(|| value.clone());
            }
        }

        params
    }

    pub(crate) async fn verify_account_checks(
        &mut self,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<()> {
        match self.name.as_str() {
            "CardList" => self.check_account_crc("my", callback).await,
            "DauerList" => self.check_account_crc("my", callback).await,
            "Kontoauszug" | "KontoauszugPdf" | "KUmsAll" | "KUmsAllCamt" | "KUmsNew"
            | "SaldoReq" | "SaldoReqAll" => self.check_account_crc("my", callback).await,
            "FestList" => self.check_account_crc("my", callback).await,
            "DauerEdit" | "DauerNew" | "TermUeb" | "TermUebEdit" | "Ueb" | "UebBZU" | "UebEil"
            | "Umb" => {
                self.check_account_crc("src", callback).await?;
                self.check_account_crc("dst", callback).await
            }
            "TermUebList" => self.check_account_crc("my", callback).await,
            "UebForeign" => self.check_account_crc("src", callback).await,
            _ => Ok(()),
        }
    }

    fn resolved_constraint_value(
        &self,
        constraint: &HbciJobConstraint,
    ) -> HbciResult<Option<ResolvedConstraintValue>> {
        if constraint.destination_name.is_empty() {
            return Ok(None);
        }

        if let Some(value) = self
            .lowlevel_param(&constraint.destination_name)
            .filter(|value| !value.is_empty())
        {
            return Ok(Some(ResolvedConstraintValue::existing(value)));
        }

        if constraint.indexed {
            let indexed_destination = indexed_destination_name(&constraint.destination_name, 0);
            if let Some(value) = self
                .lowlevel_param(&indexed_destination)
                .filter(|value| !value.is_empty())
            {
                return Ok(Some(ResolvedConstraintValue::existing(value)));
            }
        }

        if let Some(value) = self
            .param(&constraint.frontend_name)
            .filter(|value| !value.is_empty())
        {
            return Ok(Some(ResolvedConstraintValue::new(value.to_owned(), true)));
        }

        let content = constraint.default_value.clone().ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!(
                    "missing required job parameter: {}",
                    constraint.frontend_name
                ),
            )
        })?;

        Ok((!content.is_empty()).then(|| ResolvedConstraintValue::new(content, true)))
    }

    async fn check_account_crc(
        &mut self,
        frontend_base: &str,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<()> {
        self.check_iban_crc(frontend_base, callback).await
    }

    async fn check_iban_crc(
        &mut self,
        frontend_base: &str,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<()> {
        let frontend_name = format!("{frontend_base}.iban");
        let Some(mut iban) = self.resolved_frontend_value(&frontend_name) else {
            return Ok(());
        };
        if iban.is_empty() || iban_crc_ok(&iban) {
            return Ok(());
        }

        let original_iban = iban.clone();
        let Some(callback) = callback else {
            return Ok(());
        };

        loop {
            let old_iban = iban.clone();
            let response = callback
                .handle(CallbackEvent {
                    reason: CallbackReason::HaveIbanError,
                    message: "CALLB_HAVE_IBAN_ERROR".to_owned(),
                    data_type: CallbackDataType::Text,
                    current_value: Some(iban.clone()),
                })
                .await?;

            iban = response.value.unwrap_or_else(|| old_iban.clone());
            if iban == old_iban || iban_crc_ok(&iban) {
                break;
            }
        }

        if iban != original_iban {
            self.set_frontend_and_lowlevel_param(frontend_name, iban);
        }
        Ok(())
    }

    fn resolved_frontend_value(&self, frontend_name: &str) -> Option<String> {
        self.constraint(frontend_name).and_then(|constraint| {
            self.lowlevel_param(&constraint.destination_name)
                .or_else(|| self.param(frontend_name))
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
    }

    fn set_optional_account_param(&mut self, base: &str, field: &str, value: Option<&str>) {
        self.set_optional_structured_param(base, field, value);
    }

    fn set_optional_structured_param(&mut self, base: &str, field: &str, value: Option<&str>) {
        let name = format!("{base}.{field}");
        if self.accepts_param(&name)
            && let Some(value) = value.filter(|value| !value.is_empty())
        {
            self.set_frontend_and_lowlevel_param(name, value);
        }
    }

    fn try_set_optional_indexed_structured_param(
        &mut self,
        base: &str,
        index: usize,
        field: &str,
        value: Option<&str>,
    ) -> HbciResult<()> {
        let name = format!("{base}.{field}");
        if self.accepts_param(&name)
            && let Some(value) = value.filter(|value| !value.is_empty())
        {
            self.try_set_indexed_param(name, index, value)?;
        }
        Ok(())
    }

    fn set_frontend_and_lowlevel_param(
        &mut self,
        frontend_name: impl Into<String>,
        value: impl Into<String>,
    ) {
        let frontend_name = frontend_name.into();
        let value = value.into();

        self.set_lowlevel_params_for_frontend(&frontend_name, &value);
        self.set_param(frontend_name, value);
    }

    fn set_lowlevel_params_for_frontend(&mut self, frontend_name: &str, value: &str) {
        let destinations = self
            .constraints
            .iter()
            .filter(|constraint| constraint.frontend_name == frontend_name)
            .filter(|constraint| !constraint.destination_name.is_empty())
            .map(|constraint| constraint.destination_name.clone())
            .collect::<Vec<_>>();
        let lowlevel_value = self.lowlevel_value_for_frontend(frontend_name, value);

        for destination in destinations {
            self.lowlevel_params
                .insert(destination, lowlevel_value.clone());
        }
    }

    fn lowlevel_value_for_frontend(&self, frontend_name: &str, value: &str) -> String {
        if self.name == "TAN2Step" && frontend_name == "orderhash" {
            return format!("B{value}");
        }

        if self.name == "VoPAuth" && frontend_name == "vopid" {
            return binary_lowlevel_value(value);
        }

        value.to_owned()
    }

    fn set_status_jobid_param(&mut self, value: String) -> HbciResult<()> {
        let date = status_jobid_date(&value)?;
        self.set_frontend_and_lowlevel_param("startdate", date.clone());
        self.set_frontend_and_lowlevel_param("enddate", date);
        self.params.insert("jobid".to_owned(), value);
        Ok(())
    }

    fn set_indexed_lowlevel_params_for_frontend(
        &mut self,
        frontend_name: &str,
        index: usize,
        value: &str,
    ) {
        let destinations = self
            .constraints
            .iter()
            .filter(|constraint| constraint.frontend_name == frontend_name && constraint.indexed)
            .map(|constraint| indexed_destination_name(&constraint.destination_name, index))
            .collect::<Vec<_>>();

        for destination in destinations {
            self.lowlevel_params.insert(destination, value.to_owned());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedConstraintValue {
    value: String,
    persist_destination: bool,
}

impl ResolvedConstraintValue {
    fn existing(value: &str) -> Self {
        Self::new(value.to_owned(), false)
    }

    fn new(value: String, persist_destination: bool) -> Self {
        Self {
            value,
            persist_destination,
        }
    }
}

fn iban_crc_ok(iban: &str) -> bool {
    Konto {
        iban: Some(iban.to_owned()),
        ..Konto::default()
    }
    .check_iban()
}

fn status_jobid_date(value: &str) -> HbciResult<String> {
    let Some((date, _rest)) = value.split_once('/') else {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Status jobid must start with yyyyMMdd/: {value}"),
        ));
    };
    if date.len() != 8 || !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("Status jobid must start with yyyyMMdd/: {value}"),
        ));
    }

    normalize_iso_date(&format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]))
}

fn validate_bzu_data(value: &str) -> HbciResult<()> {
    let len = value.len();
    if len != 13 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("UebBZU bzudata must be exactly 13 characters, got {len}"),
        ));
    }

    let mut p: i32 = 10;
    let mut s: i32 = 0;
    for byte in value.bytes() {
        s = (p % 11) + (i32::from(byte) - 0x30);
        let mut modulo = s % 10;
        if modulo == 0 {
            modulo = 10;
        }
        p = modulo << 1;
    }

    if s % 10 != 1 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("invalid UebBZU bzudata check digit: {value}"),
        ));
    }

    Ok(())
}

fn binary_lowlevel_value(value: &str) -> String {
    if value.starts_with('B') || value.starts_with('N') {
        value.to_owned()
    } else {
        format!("B{value}")
    }
}

fn indexed_destination_name(destination: &str, index: usize) -> String {
    let parts = destination.split('.').collect::<Vec<_>>();
    if !matches!(parts.len(), 3..=5) || !parts.iter().all(|part| is_word_part(part)) {
        return destination.to_owned();
    }

    if parts.len() == 3 {
        format!("{}.{}.{}[{index}]", parts[0], parts[1], parts[2])
    } else {
        let base = parts[..parts.len() - 1].join(".");
        format!("{base}[{index}].{}", parts[parts.len() - 1])
    }
}

fn single_transfer_sepa_name(name: &str) -> String {
    name.replace("[0]", "")
}

fn is_word_part(part: &str) -> bool {
    !part.is_empty()
        && part
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HbciJobConstraint {
    pub frontend_name: String,
    pub destination_name: String,
    pub default_value: Option<String>,
    pub indexed: bool,
}

impl HbciJobConstraint {
    pub fn new(
        frontend_name: impl Into<String>,
        destination_name: impl Into<String>,
        default_value: Option<impl Into<String>>,
    ) -> Self {
        Self {
            frontend_name: frontend_name.into(),
            destination_name: destination_name.into(),
            default_value: default_value.map(Into::into),
            indexed: false,
        }
    }

    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }
}

fn constraints_for_job(name: &str) -> Vec<HbciJobConstraint> {
    match name {
        "AccInfo" => acc_info_constraints(),
        "CardList" => card_list_constraints(),
        "DauerDel" => dauer_del_constraints(),
        "DauerEdit" => dauer_edit_constraints(),
        "DauerList" => dauer_list_constraints(),
        "DauerNew" => dauer_new_constraints(),
        "DauerSEPAEdit" => dauer_sepa_edit_constraints(),
        "DauerSEPAList" => dauer_sepa_list_constraints(),
        "DauerSEPADel" => dauer_sepa_del_constraints(),
        "DauerSEPANew" => dauer_sepa_new_constraints(),
        "FestCondList" => fest_cond_list_constraints(),
        "FestList" => fest_list_constraints(),
        "InfoList" => info_list_constraints(),
        "InfoOrder" => info_order_constraints(),
        "LastSEPA" => last_sepa_constraints(),
        "Kontoauszug" => kontoauszug_constraints(),
        "KontoauszugPdf" => kontoauszug_pdf_constraints(),
        "TermUeb" => term_ueb_constraints(),
        "TermUebDel" => term_ueb_del_constraints(),
        "TermUebEdit" => term_ueb_edit_constraints(),
        "TermUebSEPA" => term_ueb_sepa_constraints(),
        "TermUebSEPADel" => term_ueb_sepa_del_constraints(),
        "TermUebSEPAEdit" => term_ueb_sepa_edit_constraints(),
        "TermUebSEPAList" => term_ueb_sepa_list_constraints(),
        "TermUebList" => term_ueb_list_constraints(),
        "InstUebSEPA" => inst_ueb_sepa_constraints(),
        "Ueb" => ueb_constraints(),
        "UebBZU" => ueb_bzu_constraints(),
        "UebEil" => ueb_eil_constraints(),
        "UebForeign" => ueb_foreign_constraints(),
        "UebSEPA" => ueb_sepa_constraints(),
        "Umb" => umb_constraints(),
        "UmbSEPA" => umb_sepa_constraints(),
        "KUmsAll" => kums_all_constraints(),
        "KUmsAllCamt" => kums_all_camt_constraints(),
        "KUmsNew" => kums_new_constraints(),
        "KUmsZeitSEPA" => kums_zeit_sepa_constraints(),
        "Receipt" => receipt_constraints(),
        "SaldoReq" => saldo_req_constraints(),
        "SaldoReqAll" => saldo_req_all_constraints(),
        "Status" => status_constraints(),
        "TANList" => Vec::new(),
        "TANMediaList" => tan_media_list_constraints(),
        "TAN2Step" => tan2step_constraints(),
        "VoPAuth" => vop_auth_constraints(),
        _ => Vec::new(),
    }
}

fn dauer_sepa_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "DauerSEPAList2.My.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "DauerSEPAList2.My.iban", None::<String>),
        HbciJobConstraint::new("src.bic", "DauerSEPAList2.My.bic", Some("")),
        HbciJobConstraint::new("src.iban", "DauerSEPAList2.My.iban", Some("")),
        HbciJobConstraint::new("my.country", "DauerSEPAList2.My.KIK.country", Some("")),
        HbciJobConstraint::new("my.blz", "DauerSEPAList2.My.KIK.blz", Some("")),
        HbciJobConstraint::new("my.number", "DauerSEPAList2.My.number", Some("")),
        HbciJobConstraint::new("my.subnumber", "DauerSEPAList2.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "DauerSEPAList2.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("orderid", "DauerSEPAList2.orderid", Some("")),
        HbciJobConstraint::new("maxentries", "DauerSEPAList2.maxentries", Some("")),
    ]
}

fn dauer_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.country", "DauerList5.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "DauerList5.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "DauerList5.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "DauerList5.KTV.subnumber", Some("")),
        HbciJobConstraint::new("orderid", "DauerList5.orderid", Some("")),
        HbciJobConstraint::new("maxentries", "DauerList5.maxentries", Some("")),
    ]
}

fn dauer_new_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("src.number", "DauerNew5.My.number", None::<String>),
        HbciJobConstraint::new("src.subnumber", "DauerNew5.My.subnumber", Some("")),
        HbciJobConstraint::new("dst.blz", "DauerNew5.Other.KIK.blz", None::<String>),
        HbciJobConstraint::new("dst.number", "DauerNew5.Other.number", None::<String>),
        HbciJobConstraint::new("dst.subnumber", "DauerNew5.Other.subnumber", Some("")),
        HbciJobConstraint::new("btg.value", "DauerNew5.BTG.value", None::<String>),
        HbciJobConstraint::new("btg.curr", "DauerNew5.BTG.curr", None::<String>),
        HbciJobConstraint::new("name", "DauerNew5.name", None::<String>),
        HbciJobConstraint::new(
            "firstdate",
            "DauerNew5.DauerDetails.firstdate",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "timeunit",
            "DauerNew5.DauerDetails.timeunit",
            None::<String>,
        ),
        HbciJobConstraint::new("turnus", "DauerNew5.DauerDetails.turnus", None::<String>),
        HbciJobConstraint::new("execday", "DauerNew5.DauerDetails.execday", None::<String>),
        HbciJobConstraint::new("src.blz", "DauerNew5.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.country", "DauerNew5.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("dst.country", "DauerNew5.Other.KIK.country", Some("DE")),
        HbciJobConstraint::new("name2", "DauerNew5.name2", Some("")),
        HbciJobConstraint::new("lastdate", "DauerNew5.DauerDetails.lastdate", Some("")),
        HbciJobConstraint::new("key", "DauerNew5.key", Some("52")),
    ];

    for index in 0..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("DauerNew5.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn dauer_edit_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("src.number", "DauerEdit5.My.number", None::<String>),
        HbciJobConstraint::new("src.subnumber", "DauerEdit5.My.subnumber", Some("")),
        HbciJobConstraint::new("dst.blz", "DauerEdit5.Other.KIK.blz", None::<String>),
        HbciJobConstraint::new("dst.number", "DauerEdit5.Other.number", None::<String>),
        HbciJobConstraint::new("dst.subnumber", "DauerEdit5.Other.subnumber", Some("")),
        HbciJobConstraint::new("btg.value", "DauerEdit5.BTG.value", None::<String>),
        HbciJobConstraint::new("btg.curr", "DauerEdit5.BTG.curr", None::<String>),
        HbciJobConstraint::new("name", "DauerEdit5.name", None::<String>),
        HbciJobConstraint::new(
            "firstdate",
            "DauerEdit5.DauerDetails.firstdate",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "timeunit",
            "DauerEdit5.DauerDetails.timeunit",
            None::<String>,
        ),
        HbciJobConstraint::new("turnus", "DauerEdit5.DauerDetails.turnus", None::<String>),
        HbciJobConstraint::new("execday", "DauerEdit5.DauerDetails.execday", None::<String>),
        HbciJobConstraint::new("orderid", "DauerEdit5.orderid", None::<String>),
        HbciJobConstraint::new("src.blz", "DauerEdit5.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.country", "DauerEdit5.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("dst.country", "DauerEdit5.Other.KIK.country", Some("DE")),
        HbciJobConstraint::new("name2", "DauerEdit5.name2", Some("")),
        HbciJobConstraint::new("key", "DauerEdit5.key", Some("52")),
        HbciJobConstraint::new("date", "DauerEdit5.date", Some("")),
        HbciJobConstraint::new("lastdate", "DauerEdit5.DauerDetails.lastdate", Some("")),
    ];

    for index in 0..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("DauerEdit5.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn dauer_del_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("src.number", "DauerDel4.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "DauerDel4.My.subnumber", Some("")),
        HbciJobConstraint::new("dst.blz", "DauerDel4.Other.KIK.blz", Some("")),
        HbciJobConstraint::new("dst.number", "DauerDel4.Other.number", Some("")),
        HbciJobConstraint::new("dst.subnumber", "DauerDel4.Other.subnumber", Some("")),
        HbciJobConstraint::new("btg.value", "DauerDel4.BTG.value", Some("")),
        HbciJobConstraint::new("btg.curr", "DauerDel4.BTG.curr", Some("")),
        HbciJobConstraint::new("name", "DauerDel4.name", Some("")),
        HbciJobConstraint::new("firstdate", "DauerDel4.DauerDetails.firstdate", Some("")),
        HbciJobConstraint::new("timeunit", "DauerDel4.DauerDetails.timeunit", Some("")),
        HbciJobConstraint::new("turnus", "DauerDel4.DauerDetails.turnus", Some("")),
        HbciJobConstraint::new("execday", "DauerDel4.DauerDetails.execday", Some("")),
        HbciJobConstraint::new("src.blz", "DauerDel4.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.country", "DauerDel4.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("dst.country", "DauerDel4.Other.KIK.country", Some("DE")),
        HbciJobConstraint::new("name2", "DauerDel4.name2", Some("")),
        HbciJobConstraint::new("key", "DauerDel4.key", Some("52")),
        HbciJobConstraint::new("date", "DauerDel4.date", Some("")),
        HbciJobConstraint::new("orderid", "DauerDel4.orderid", Some("")),
        HbciJobConstraint::new("lastdate", "DauerDel4.DauerDetails.lastdate", Some("")),
    ];

    for index in 0..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("DauerDel4.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn term_ueb_sepa_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "TermUebSEPAList1.My.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "TermUebSEPAList1.My.iban", None::<String>),
        HbciJobConstraint::new("src.bic", "TermUebSEPAList1.My.bic", Some("")),
        HbciJobConstraint::new("src.iban", "TermUebSEPAList1.My.iban", Some("")),
        HbciJobConstraint::new("my.country", "TermUebSEPAList1.My.KIK.country", Some("")),
        HbciJobConstraint::new("my.blz", "TermUebSEPAList1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("my.number", "TermUebSEPAList1.My.number", Some("")),
        HbciJobConstraint::new("my.subnumber", "TermUebSEPAList1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "TermUebSEPAList1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("startdate", "TermUebSEPAList1.startdate", Some("")),
        HbciJobConstraint::new("enddate", "TermUebSEPAList1.enddate", Some("")),
        HbciJobConstraint::new("maxentries", "TermUebSEPAList1.maxentries", Some("")),
    ]
}

fn term_ueb_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.country", "TermUebList3.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "TermUebList3.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "TermUebList3.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "TermUebList3.KTV.subnumber", Some("")),
        HbciJobConstraint::new("startdate", "TermUebList3.startdate", Some("")),
        HbciJobConstraint::new("enddate", "TermUebList3.enddate", Some("")),
        HbciJobConstraint::new("maxentries", "TermUebList3.maxentries", Some("")),
    ]
}

fn term_ueb_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("src.country", "TermUeb4.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("src.blz", "TermUeb4.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.number", "TermUeb4.My.number", None::<String>),
        HbciJobConstraint::new("src.subnumber", "TermUeb4.My.subnumber", Some("")),
        HbciJobConstraint::new("dst.country", "TermUeb4.Other.KIK.country", Some("DE")),
        HbciJobConstraint::new("dst.blz", "TermUeb4.Other.KIK.blz", None::<String>),
        HbciJobConstraint::new("dst.number", "TermUeb4.Other.number", None::<String>),
        HbciJobConstraint::new("dst.subnumber", "TermUeb4.Other.subnumber", Some("")),
        HbciJobConstraint::new("btg.value", "TermUeb4.BTG.value", None::<String>),
        HbciJobConstraint::new("btg.curr", "TermUeb4.BTG.curr", None::<String>),
        HbciJobConstraint::new("name", "TermUeb4.name", None::<String>),
        HbciJobConstraint::new("date", "TermUeb4.date", None::<String>),
        HbciJobConstraint::new("name2", "TermUeb4.name2", Some("")),
        HbciJobConstraint::new("key", "TermUeb4.key", Some("51")),
    ];

    for index in 0..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("TermUeb4.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn term_ueb_del_constraints() -> Vec<HbciJobConstraint> {
    vec![HbciJobConstraint::new(
        "orderid",
        "TermUebDel3.id",
        None::<String>,
    )]
}

fn term_ueb_edit_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("src.country", "TermUebEdit4.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("src.blz", "TermUebEdit4.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.number", "TermUebEdit4.My.number", None::<String>),
        HbciJobConstraint::new("src.subnumber", "TermUebEdit4.My.subnumber", Some("")),
        HbciJobConstraint::new("dst.country", "TermUebEdit4.Other.KIK.country", Some("DE")),
        HbciJobConstraint::new("dst.blz", "TermUebEdit4.Other.KIK.blz", None::<String>),
        HbciJobConstraint::new("dst.number", "TermUebEdit4.Other.number", Some("")),
        HbciJobConstraint::new("dst.subnumber", "TermUebEdit4.Other.subnumber", Some("")),
        HbciJobConstraint::new("btg.value", "TermUebEdit4.BTG.value", None::<String>),
        HbciJobConstraint::new("btg.curr", "TermUebEdit4.BTG.curr", None::<String>),
        HbciJobConstraint::new("name", "TermUebEdit4.name", None::<String>),
        HbciJobConstraint::new("date", "TermUebEdit4.date", None::<String>),
        HbciJobConstraint::new("orderid", "TermUebEdit4.id", None::<String>),
        HbciJobConstraint::new("name2", "TermUebEdit4.name2", Some("")),
        HbciJobConstraint::new("key", "TermUebEdit4.key", Some("51")),
    ];

    for index in 0..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("TermUebEdit4.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn classic_usage_name(index: usize) -> String {
    if index == 0 {
        "usage".to_owned()
    } else {
        format!("usage_{}", index + 1)
    }
}

fn ueb_constraints() -> Vec<HbciJobConstraint> {
    classic_transfer_constraints("Ueb5")
}

fn ueb_eil_constraints() -> Vec<HbciJobConstraint> {
    classic_transfer_constraints("UebEil1")
}

fn ueb_foreign_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.country", "UebForeign2.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("src.blz", "UebForeign2.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.number", "UebForeign2.My.number", None::<String>),
        HbciJobConstraint::new("src.subnumber", "UebForeign2.My.subnumber", Some("")),
        HbciJobConstraint::new("src.name", "UebForeign2.myname", None::<String>),
        HbciJobConstraint::new("dst.country", "UebForeign2.Other.KIK.country", Some("")),
        HbciJobConstraint::new("dst.blz", "UebForeign2.Other.KIK.blz", Some("")),
        HbciJobConstraint::new("dst.number", "UebForeign2.Other.number", Some("")),
        HbciJobConstraint::new("dst.subnumber", "UebForeign2.Other.subnumber", Some("")),
        HbciJobConstraint::new("dst.iban", "UebForeign2.otheriban", Some("")),
        HbciJobConstraint::new("dst.name", "UebForeign2.othername", None::<String>),
        HbciJobConstraint::new("dst.kiname", "UebForeign2.otherkiname", None::<String>),
        HbciJobConstraint::new("btg.value", "UebForeign2.BTG.value", None::<String>),
        HbciJobConstraint::new("btg.curr", "UebForeign2.BTG.curr", None::<String>),
        HbciJobConstraint::new("kostentraeger", "UebForeign2.kostentraeger", Some("1")),
        HbciJobConstraint::new("usage", "UebForeign2.usage", Some("")),
    ]
}

fn umb_constraints() -> Vec<HbciJobConstraint> {
    classic_transfer_constraints("Umb2")
}

fn ueb_bzu_constraints() -> Vec<HbciJobConstraint> {
    let lowlevel_segment = "Ueb5";
    let mut constraints = vec![
        HbciJobConstraint::new("src.country", "Ueb5.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("src.blz", "Ueb5.My.KIK.blz", None::<String>),
        HbciJobConstraint::new("src.number", "Ueb5.My.number", None::<String>),
        HbciJobConstraint::new("src.subnumber", "Ueb5.My.subnumber", Some("")),
        HbciJobConstraint::new("dst.country", "Ueb5.Other.KIK.country", Some("DE")),
        HbciJobConstraint::new("dst.blz", "Ueb5.Other.KIK.blz", None::<String>),
        HbciJobConstraint::new("dst.number", "Ueb5.Other.number", None::<String>),
        HbciJobConstraint::new("dst.subnumber", "Ueb5.Other.subnumber", Some("")),
        HbciJobConstraint::new("btg.value", "Ueb5.BTG.value", None::<String>),
        HbciJobConstraint::new("btg.curr", "Ueb5.BTG.curr", None::<String>),
        HbciJobConstraint::new("name", "Ueb5.name", None::<String>),
        HbciJobConstraint::new("bzudata", "Ueb5.usage.usage", None::<String>),
        HbciJobConstraint::new("name2", "Ueb5.name2", Some("")),
        HbciJobConstraint::new("key", "Ueb5.key", Some("67")),
    ];

    for index in 1..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("{lowlevel_segment}.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn classic_transfer_constraints(lowlevel_segment: &str) -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new(
            "src.country",
            format!("{lowlevel_segment}.My.KIK.country"),
            Some("DE"),
        ),
        HbciJobConstraint::new(
            "src.blz",
            format!("{lowlevel_segment}.My.KIK.blz"),
            None::<String>,
        ),
        HbciJobConstraint::new(
            "src.number",
            format!("{lowlevel_segment}.My.number"),
            None::<String>,
        ),
        HbciJobConstraint::new(
            "src.subnumber",
            format!("{lowlevel_segment}.My.subnumber"),
            Some(""),
        ),
        HbciJobConstraint::new(
            "dst.country",
            format!("{lowlevel_segment}.Other.KIK.country"),
            Some("DE"),
        ),
        HbciJobConstraint::new(
            "dst.blz",
            format!("{lowlevel_segment}.Other.KIK.blz"),
            None::<String>,
        ),
        HbciJobConstraint::new(
            "dst.number",
            format!("{lowlevel_segment}.Other.number"),
            None::<String>,
        ),
        HbciJobConstraint::new(
            "dst.subnumber",
            format!("{lowlevel_segment}.Other.subnumber"),
            Some(""),
        ),
        HbciJobConstraint::new(
            "btg.value",
            format!("{lowlevel_segment}.BTG.value"),
            None::<String>,
        ),
        HbciJobConstraint::new(
            "btg.curr",
            format!("{lowlevel_segment}.BTG.curr"),
            None::<String>,
        ),
        HbciJobConstraint::new("name", format!("{lowlevel_segment}.name"), None::<String>),
        HbciJobConstraint::new("name2", format!("{lowlevel_segment}.name2"), Some("")),
        HbciJobConstraint::new("key", format!("{lowlevel_segment}.key"), Some("51")),
    ];

    for index in 0..CLASSIC_USAGE_LINE_COUNT {
        let frontend = classic_usage_name(index);
        let destination = format!("{lowlevel_segment}.usage.{frontend}");
        constraints.push(HbciJobConstraint::new(frontend, destination, Some("")));
    }

    constraints
}

fn term_ueb_sepa_del_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "TermUebSEPADel1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "TermUebSEPADel1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "TermUebSEPADel1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "TermUebSEPADel1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "TermUebSEPADel1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "TermUebSEPADel1.My.subnumber", Some("")),
        HbciJobConstraint::new("orderid", "TermUebSEPADel1.orderid", None::<String>),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "TermUebSEPADel1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "TermUebSEPADel1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "TermUebSEPADel1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "TermUebSEPADel1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "TermUebSEPADel1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "TermUebSEPADel1.sepa.dst.bic", Some("")),
        HbciJobConstraint::new("dst.iban", "TermUebSEPADel1.sepa.dst.iban", Some("")),
        HbciJobConstraint::new("dst.name", "TermUebSEPADel1.sepa.dst.name", Some("")),
        HbciJobConstraint::new("btg.value", "TermUebSEPADel1.sepa.btg.value", Some("")),
        HbciJobConstraint::new("btg.curr", "TermUebSEPADel1.sepa.btg.curr", Some("EUR")),
        HbciJobConstraint::new("usage", "TermUebSEPADel1.sepa.usage", Some("")),
        HbciJobConstraint::new("date", "TermUebSEPADel1.sepa.date", None::<String>),
        HbciJobConstraint::new("sepaid", "TermUebSEPADel1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "TermUebSEPADel1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "TermUebSEPADel1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        ),
        HbciJobConstraint::new("purposecode", "TermUebSEPADel1.sepa.purposecode", Some("")),
    ]
}

fn term_ueb_sepa_edit_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "TermUebSEPAEdit1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "TermUebSEPAEdit1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "TermUebSEPAEdit1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "TermUebSEPAEdit1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "TermUebSEPAEdit1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "TermUebSEPAEdit1.My.subnumber", Some("")),
        HbciJobConstraint::new("orderid", "TermUebSEPAEdit1.orderid", None::<String>),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "TermUebSEPAEdit1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "TermUebSEPAEdit1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "TermUebSEPAEdit1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "TermUebSEPAEdit1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "TermUebSEPAEdit1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "TermUebSEPAEdit1.sepa.dst.bic", Some("")),
        HbciJobConstraint::new("dst.iban", "TermUebSEPAEdit1.sepa.dst.iban", Some("")),
        HbciJobConstraint::new("dst.name", "TermUebSEPAEdit1.sepa.dst.name", Some("")),
        HbciJobConstraint::new("btg.value", "TermUebSEPAEdit1.sepa.btg.value", Some("")),
        HbciJobConstraint::new("btg.curr", "TermUebSEPAEdit1.sepa.btg.curr", Some("EUR")),
        HbciJobConstraint::new("usage", "TermUebSEPAEdit1.sepa.usage", Some("")),
        HbciJobConstraint::new("date", "TermUebSEPAEdit1.sepa.date", None::<String>),
        HbciJobConstraint::new("sepaid", "TermUebSEPAEdit1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "TermUebSEPAEdit1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "TermUebSEPAEdit1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        ),
        HbciJobConstraint::new("purposecode", "TermUebSEPAEdit1.sepa.purposecode", Some("")),
    ]
}

fn term_ueb_sepa_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "TermUebSEPA1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "TermUebSEPA1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "TermUebSEPA1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "TermUebSEPA1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "TermUebSEPA1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "TermUebSEPA1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "TermUebSEPA1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "TermUebSEPA1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "TermUebSEPA1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "TermUebSEPA1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "TermUebSEPA1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "TermUebSEPA1.sepa.dst.bic", Some("")),
        HbciJobConstraint::new("dst.iban", "TermUebSEPA1.sepa.dst.iban", Some("")),
        HbciJobConstraint::new("dst.name", "TermUebSEPA1.sepa.dst.name", Some("")),
        HbciJobConstraint::new("btg.value", "TermUebSEPA1.sepa.btg.value", Some("")),
        HbciJobConstraint::new("btg.curr", "TermUebSEPA1.sepa.btg.curr", Some("EUR")),
        HbciJobConstraint::new("usage", "TermUebSEPA1.sepa.usage", Some("")),
        HbciJobConstraint::new("date", "TermUebSEPA1.sepa.date", None::<String>),
        HbciJobConstraint::new("sepaid", "TermUebSEPA1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "TermUebSEPA1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "TermUebSEPA1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        ),
        HbciJobConstraint::new("purposecode", "TermUebSEPA1.sepa.purposecode", Some("")),
    ]
}

fn ueb_sepa_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "UebSEPA1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "UebSEPA1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "UebSEPA1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "UebSEPA1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "UebSEPA1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "UebSEPA1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "UebSEPA1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "UebSEPA1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "UebSEPA1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "UebSEPA1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "UebSEPA1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "UebSEPA1.sepa.dst.bic", Some("")).indexed(true),
        HbciJobConstraint::new("dst.iban", "UebSEPA1.sepa.dst.iban", Some("")).indexed(true),
        HbciJobConstraint::new("dst.name", "UebSEPA1.sepa.dst.name", Some("")).indexed(true),
        HbciJobConstraint::new("btg.value", "UebSEPA1.sepa.btg.value", Some("")).indexed(true),
        HbciJobConstraint::new("btg.curr", "UebSEPA1.sepa.btg.curr", Some("EUR")).indexed(true),
        HbciJobConstraint::new("usage", "UebSEPA1.sepa.usage", Some("")).indexed(true),
        HbciJobConstraint::new("batchbook", "UebSEPA1.sepa.batchbook", Some("0")),
        HbciJobConstraint::new("sepaid", "UebSEPA1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "UebSEPA1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "UebSEPA1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        )
        .indexed(true),
        HbciJobConstraint::new("purposecode", "UebSEPA1.sepa.purposecode", Some("")).indexed(true),
    ]
}

fn last_sepa_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "LastSEPA1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "LastSEPA1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "LastSEPA1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "LastSEPA1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "LastSEPA1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "LastSEPA1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "LastSEPA1.sepadescr",
            Some(PAIN_008_001_01_URN),
        ),
        HbciJobConstraint::new("_sepapain", "LastSEPA1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "LastSEPA1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "LastSEPA1.sepa.src.iban", None::<String>),
        HbciJobConstraint::new("src.name", "LastSEPA1.sepa.src.name", None::<String>),
        HbciJobConstraint::new("dst.bic", "LastSEPA1.sepa.dst.bic", Some("")).indexed(true),
        HbciJobConstraint::new("dst.iban", "LastSEPA1.sepa.dst.iban", None::<String>).indexed(true),
        HbciJobConstraint::new("dst.name", "LastSEPA1.sepa.dst.name", None::<String>).indexed(true),
        HbciJobConstraint::new(
            "dst.addr.country",
            "LastSEPA1.sepa.dst.addr.country",
            Some(""),
        )
        .indexed(true),
        HbciJobConstraint::new("dst.addr.line1", "LastSEPA1.sepa.dst.addr.line1", Some(""))
            .indexed(true),
        HbciJobConstraint::new("dst.addr.line2", "LastSEPA1.sepa.dst.addr.line2", Some(""))
            .indexed(true),
        HbciJobConstraint::new("btg.value", "LastSEPA1.sepa.btg.value", None::<String>)
            .indexed(true),
        HbciJobConstraint::new("btg.curr", "LastSEPA1.sepa.btg.curr", Some("EUR")).indexed(true),
        HbciJobConstraint::new("usage", "LastSEPA1.sepa.usage", Some("")).indexed(true),
        HbciJobConstraint::new("sepaid", "LastSEPA1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "LastSEPA1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "LastSEPA1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        )
        .indexed(true),
        HbciJobConstraint::new("creditorid", "LastSEPA1.sepa.creditorid", None::<String>)
            .indexed(true),
        HbciJobConstraint::new("mandateid", "LastSEPA1.sepa.mandateid", None::<String>)
            .indexed(true),
        HbciJobConstraint::new("purposecode", "LastSEPA1.sepa.purposecode", Some("")).indexed(true),
        HbciJobConstraint::new(
            "manddateofsig",
            "LastSEPA1.sepa.manddateofsig",
            None::<String>,
        )
        .indexed(true),
        HbciJobConstraint::new(
            "amendmandindic",
            "LastSEPA1.sepa.amendmandindic",
            Some("false"),
        )
        .indexed(true),
        HbciJobConstraint::new("sequencetype", "LastSEPA1.sepa.sequencetype", Some("FRST")),
        HbciJobConstraint::new(
            "targetdate",
            "LastSEPA1.sepa.targetdate",
            Some("1999-01-01"),
        ),
        HbciJobConstraint::new("type", "LastSEPA1.sepa.type", Some("CORE")),
        HbciJobConstraint::new("batchbook", "LastSEPA1.sepa.batchbook", Some("0")),
    ]
}

fn inst_ueb_sepa_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "InstUebSEPA1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "InstUebSEPA1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "InstUebSEPA1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "InstUebSEPA1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "InstUebSEPA1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "InstUebSEPA1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "InstUebSEPA1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "InstUebSEPA1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "InstUebSEPA1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "InstUebSEPA1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "InstUebSEPA1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "InstUebSEPA1.sepa.dst.bic", Some("")).indexed(true),
        HbciJobConstraint::new("dst.iban", "InstUebSEPA1.sepa.dst.iban", Some("")).indexed(true),
        HbciJobConstraint::new("dst.name", "InstUebSEPA1.sepa.dst.name", Some("")).indexed(true),
        HbciJobConstraint::new("btg.value", "InstUebSEPA1.sepa.btg.value", Some("")).indexed(true),
        HbciJobConstraint::new("btg.curr", "InstUebSEPA1.sepa.btg.curr", Some("EUR")).indexed(true),
        HbciJobConstraint::new("usage", "InstUebSEPA1.sepa.usage", Some("")).indexed(true),
        HbciJobConstraint::new("batchbook", "InstUebSEPA1.sepa.batchbook", Some("0")),
        HbciJobConstraint::new("sepaid", "InstUebSEPA1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "InstUebSEPA1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "InstUebSEPA1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        )
        .indexed(true),
        HbciJobConstraint::new("purposecode", "InstUebSEPA1.sepa.purposecode", Some(""))
            .indexed(true),
    ]
}

fn umb_sepa_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "UmbSEPA1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "UmbSEPA1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "UmbSEPA1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "UmbSEPA1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "UmbSEPA1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "UmbSEPA1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "UmbSEPA1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "UmbSEPA1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "UmbSEPA1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "UmbSEPA1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "UmbSEPA1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "UmbSEPA1.sepa.dst.bic", Some("")).indexed(true),
        HbciJobConstraint::new("dst.iban", "UmbSEPA1.sepa.dst.iban", Some("")).indexed(true),
        HbciJobConstraint::new("dst.name", "UmbSEPA1.sepa.dst.name", Some("")).indexed(true),
        HbciJobConstraint::new("btg.value", "UmbSEPA1.sepa.btg.value", Some("")).indexed(true),
        HbciJobConstraint::new("btg.curr", "UmbSEPA1.sepa.btg.curr", Some("EUR")).indexed(true),
        HbciJobConstraint::new("usage", "UmbSEPA1.sepa.usage", Some("")).indexed(true),
        HbciJobConstraint::new("sepaid", "UmbSEPA1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "UmbSEPA1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "UmbSEPA1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        )
        .indexed(true),
        HbciJobConstraint::new("purposecode", "UmbSEPA1.sepa.purposecode", Some("")).indexed(true),
    ]
}

fn dauer_sepa_new_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "DauerSEPANew1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "DauerSEPANew1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "DauerSEPANew1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "DauerSEPANew1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "DauerSEPANew1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "DauerSEPANew1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "DauerSEPANew1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "DauerSEPANew1.sepapain", None::<String>),
        HbciJobConstraint::new("src.bic", "DauerSEPANew1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "DauerSEPANew1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "DauerSEPANew1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "DauerSEPANew1.sepa.dst.bic", Some("")),
        HbciJobConstraint::new("dst.iban", "DauerSEPANew1.sepa.dst.iban", Some("")),
        HbciJobConstraint::new("dst.name", "DauerSEPANew1.sepa.dst.name", Some("")),
        HbciJobConstraint::new("btg.value", "DauerSEPANew1.sepa.btg.value", Some("")),
        HbciJobConstraint::new("btg.curr", "DauerSEPANew1.sepa.btg.curr", Some("EUR")),
        HbciJobConstraint::new("usage", "DauerSEPANew1.sepa.usage", Some("")),
        HbciJobConstraint::new("sepaid", "DauerSEPANew1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "DauerSEPANew1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "DauerSEPANew1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        ),
        HbciJobConstraint::new("purposecode", "DauerSEPANew1.sepa.purposecode", Some("")),
        HbciJobConstraint::new(
            "firstdate",
            "DauerSEPANew1.DauerDetails.firstdate",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "timeunit",
            "DauerSEPANew1.DauerDetails.timeunit",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "turnus",
            "DauerSEPANew1.DauerDetails.turnus",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "execday",
            "DauerSEPANew1.DauerDetails.execday",
            None::<String>,
        ),
        HbciJobConstraint::new("lastdate", "DauerSEPANew1.DauerDetails.lastdate", Some("")),
    ]
}

fn dauer_sepa_edit_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "DauerSEPAEdit1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "DauerSEPAEdit1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "DauerSEPAEdit1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "DauerSEPAEdit1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "DauerSEPAEdit1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "DauerSEPAEdit1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "DauerSEPAEdit1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "DauerSEPAEdit1.sepapain", None::<String>),
        HbciJobConstraint::new("orderid", "DauerSEPAEdit1.orderid", None::<String>),
        HbciJobConstraint::new("src.bic", "DauerSEPAEdit1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "DauerSEPAEdit1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "DauerSEPAEdit1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "DauerSEPAEdit1.sepa.dst.bic", Some("")),
        HbciJobConstraint::new("dst.iban", "DauerSEPAEdit1.sepa.dst.iban", Some("")),
        HbciJobConstraint::new("dst.name", "DauerSEPAEdit1.sepa.dst.name", Some("")),
        HbciJobConstraint::new("btg.value", "DauerSEPAEdit1.sepa.btg.value", Some("")),
        HbciJobConstraint::new("btg.curr", "DauerSEPAEdit1.sepa.btg.curr", Some("EUR")),
        HbciJobConstraint::new("usage", "DauerSEPAEdit1.sepa.usage", Some("")),
        HbciJobConstraint::new("date", "DauerSEPAEdit1.date", Some("")),
        HbciJobConstraint::new("sepaid", "DauerSEPAEdit1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "DauerSEPAEdit1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "DauerSEPAEdit1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        ),
        HbciJobConstraint::new("purposecode", "DauerSEPAEdit1.sepa.purposecode", Some("")),
        HbciJobConstraint::new(
            "firstdate",
            "DauerSEPAEdit1.DauerDetails.firstdate",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "timeunit",
            "DauerSEPAEdit1.DauerDetails.timeunit",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "turnus",
            "DauerSEPAEdit1.DauerDetails.turnus",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "execday",
            "DauerSEPAEdit1.DauerDetails.execday",
            None::<String>,
        ),
        HbciJobConstraint::new("lastdate", "DauerSEPAEdit1.DauerDetails.lastdate", Some("")),
    ]
}

fn dauer_sepa_del_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("src.bic", "DauerSEPADel1.My.bic", None::<String>),
        HbciJobConstraint::new("src.iban", "DauerSEPADel1.My.iban", None::<String>),
        HbciJobConstraint::new("src.country", "DauerSEPADel1.My.KIK.country", Some("")),
        HbciJobConstraint::new("src.blz", "DauerSEPADel1.My.KIK.blz", Some("")),
        HbciJobConstraint::new("src.number", "DauerSEPADel1.My.number", Some("")),
        HbciJobConstraint::new("src.subnumber", "DauerSEPADel1.My.subnumber", Some("")),
        HbciJobConstraint::new(
            "_sepadescriptor",
            "DauerSEPADel1.sepadescr",
            Some(PAIN_001_001_02_URN),
        ),
        HbciJobConstraint::new("_sepapain", "DauerSEPADel1.sepapain", None::<String>),
        HbciJobConstraint::new("orderid", "DauerSEPADel1.orderid", None::<String>),
        HbciJobConstraint::new("src.bic", "DauerSEPADel1.sepa.src.bic", Some("")),
        HbciJobConstraint::new("src.iban", "DauerSEPADel1.sepa.src.iban", Some("")),
        HbciJobConstraint::new("src.name", "DauerSEPADel1.sepa.src.name", Some("")),
        HbciJobConstraint::new("dst.bic", "DauerSEPADel1.sepa.dst.bic", Some("")),
        HbciJobConstraint::new("dst.iban", "DauerSEPADel1.sepa.dst.iban", Some("")),
        HbciJobConstraint::new("dst.name", "DauerSEPADel1.sepa.dst.name", Some("")),
        HbciJobConstraint::new("btg.value", "DauerSEPADel1.sepa.btg.value", Some("")),
        HbciJobConstraint::new("btg.curr", "DauerSEPADel1.sepa.btg.curr", Some("EUR")),
        HbciJobConstraint::new("usage", "DauerSEPADel1.sepa.usage", Some("")),
        HbciJobConstraint::new("date", "DauerSEPADel1.date", Some("")),
        HbciJobConstraint::new("sepaid", "DauerSEPADel1.sepa.sepaid", Some("")),
        HbciJobConstraint::new("pmtinfid", "DauerSEPADel1.sepa.pmtinfid", Some("")),
        HbciJobConstraint::new(
            "endtoendid",
            "DauerSEPADel1.sepa.endtoendid",
            Some("NOTPROVIDED"),
        ),
        HbciJobConstraint::new("purposecode", "DauerSEPADel1.sepa.purposecode", Some("")),
        HbciJobConstraint::new(
            "firstdate",
            "DauerSEPADel1.DauerDetails.firstdate",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "timeunit",
            "DauerSEPADel1.DauerDetails.timeunit",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "turnus",
            "DauerSEPADel1.DauerDetails.turnus",
            None::<String>,
        ),
        HbciJobConstraint::new(
            "execday",
            "DauerSEPADel1.DauerDetails.execday",
            None::<String>,
        ),
        HbciJobConstraint::new("lastdate", "DauerSEPADel1.DauerDetails.lastdate", Some("")),
    ]
}

fn acc_info_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.country", "AccInfo2.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "AccInfo2.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "AccInfo2.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "AccInfo2.KTV.subnumber", Some("")),
        HbciJobConstraint::new("all", "AccInfo2.allaccounts", Some("N")),
    ]
}

fn card_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.country", "CardList2.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "CardList2.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "CardList2.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "CardList2.KTV.subnumber", Some("")),
    ]
}

fn info_list_constraints() -> Vec<HbciJobConstraint> {
    vec![HbciJobConstraint::new(
        "maxentries",
        "InfoList4.maxentries",
        Some(""),
    )]
}

fn info_order_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("code", "InfoDetails4.InfoCodes.code", None::<String>),
        HbciJobConstraint::new("name", "InfoDetails4.Address.name1", Some("")),
        HbciJobConstraint::new("name2", "InfoDetails4.Address.name2", Some("")),
        HbciJobConstraint::new("street", "InfoDetails4.Address.street_pf", Some("")),
        HbciJobConstraint::new("ort", "InfoDetails4.Address.ort", Some("")),
        HbciJobConstraint::new("plz", "InfoDetails4.Address.plz_ort", Some("")),
        HbciJobConstraint::new("plz", "InfoDetails4.Address.plz", Some("")),
        HbciJobConstraint::new("country", "InfoDetails4.Address.country", Some("")),
        HbciJobConstraint::new("tel", "InfoDetails4.Address.tel", Some("")),
        HbciJobConstraint::new("fax", "InfoDetails4.Address.fax", Some("")),
        HbciJobConstraint::new("email", "InfoDetails4.Address.email", Some("")),
    ];
    constraints.extend((2..=10).map(|index| {
        HbciJobConstraint::new(
            format!("code_{index}"),
            format!("InfoDetails4.InfoCodes.code_{index}"),
            Some(""),
        )
    }));
    constraints
}

fn kums_all_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "KUmsZeit7.KTV.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "KUmsZeit7.KTV.iban", None::<String>),
        HbciJobConstraint::new("my.country", "KUmsZeit7.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "KUmsZeit7.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "KUmsZeit7.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "KUmsZeit7.KTV.subnumber", Some("")),
        HbciJobConstraint::new("startdate", "KUmsZeit7.startdate", Some("")),
        HbciJobConstraint::new("enddate", "KUmsZeit7.enddate", Some("")),
        HbciJobConstraint::new("maxentries", "KUmsZeit7.maxentries", Some("")),
        HbciJobConstraint::new("dummy", "KUmsZeit7.allaccounts", Some("N")),
    ]
}

fn kums_zeit_sepa_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "KUmsZeitSEPA7.KTV.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "KUmsZeitSEPA7.KTV.iban", None::<String>),
        HbciJobConstraint::new("startdate", "KUmsZeitSEPA7.startdate", Some("")),
        HbciJobConstraint::new("enddate", "KUmsZeitSEPA7.enddate", Some("")),
        HbciJobConstraint::new("maxentries", "KUmsZeitSEPA7.maxentries", Some("")),
        HbciJobConstraint::new("offset", "KUmsZeitSEPA7.offset", Some("")),
        HbciJobConstraint::new("all", "KUmsZeitSEPA7.allaccounts", Some("N")),
    ]
}

fn kums_new_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "KUmsNew7.KTV.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "KUmsNew7.KTV.iban", None::<String>),
        HbciJobConstraint::new("my.country", "KUmsNew7.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "KUmsNew7.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "KUmsNew7.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "KUmsNew7.KTV.subnumber", Some("")),
        HbciJobConstraint::new("maxentries", "KUmsNew7.maxentries", Some("")),
        HbciJobConstraint::new("dummyall", "KUmsNew7.allaccounts", Some("N")),
    ]
}

fn kums_all_camt_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "KUmsZeitCamt1.KTV.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "KUmsZeitCamt1.KTV.iban", None::<String>),
        HbciJobConstraint::new("my.country", "KUmsZeitCamt1.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "KUmsZeitCamt1.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "KUmsZeitCamt1.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "KUmsZeitCamt1.KTV.subnumber", Some("")),
        HbciJobConstraint::new(
            "suppformat",
            "KUmsZeitCamt1.formats.suppformat",
            Some(CAMT_052_001_01_URN),
        ),
        HbciJobConstraint::new("dummy", "KUmsZeitCamt1.allaccounts", Some("N")),
        HbciJobConstraint::new("startdate", "KUmsZeitCamt1.startdate", Some("")),
        HbciJobConstraint::new("enddate", "KUmsZeitCamt1.enddate", Some("")),
        HbciJobConstraint::new("maxentries", "KUmsZeitCamt1.maxentries", Some("")),
        HbciJobConstraint::new("offset", "KUmsZeitCamt1.offset", Some("")),
    ]
}

fn fest_cond_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("curr", "FestCondList3.curr", Some("EUR")),
        HbciJobConstraint::new("maxentries", "FestCondList3.maxentries", Some("")),
    ]
}

fn fest_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.number", "FestList4.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "FestList4.KTV.subnumber", Some("")),
        HbciJobConstraint::new("my.blz", "FestList4.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.country", "FestList4.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("dummy", "FestList4.allaccounts", Some("N")),
    ]
}

fn kontoauszug_pdf_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "KontoauszugPdf2.My.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "KontoauszugPdf2.My.iban", None::<String>),
        HbciJobConstraint::new("my.country", "KontoauszugPdf2.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "KontoauszugPdf2.My.KIK.blz", Some("")),
        HbciJobConstraint::new("my.number", "KontoauszugPdf2.My.number", Some("")),
        HbciJobConstraint::new("my.subnumber", "KontoauszugPdf2.My.subnumber", Some("")),
        HbciJobConstraint::new("idx", "KontoauszugPdf2.idx", Some("")),
        HbciJobConstraint::new("year", "KontoauszugPdf2.year", Some("")),
        HbciJobConstraint::new("maxentries", "KontoauszugPdf2.maxentries", Some("")),
        HbciJobConstraint::new("offset", "KontoauszugPdf2.offset", Some("")),
    ]
}

fn kontoauszug_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "Kontoauszug5.My.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "Kontoauszug5.My.iban", None::<String>),
        HbciJobConstraint::new("my.country", "Kontoauszug5.My.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "Kontoauszug5.My.KIK.blz", Some("")),
        HbciJobConstraint::new("my.number", "Kontoauszug5.My.number", Some("")),
        HbciJobConstraint::new("my.subnumber", "Kontoauszug5.My.subnumber", Some("")),
        HbciJobConstraint::new("format", "Kontoauszug5.format", Some("")),
        HbciJobConstraint::new("idx", "Kontoauszug5.idx", Some("")),
        HbciJobConstraint::new("year", "Kontoauszug5.year", Some("")),
        HbciJobConstraint::new("maxentries", "Kontoauszug5.maxentries", Some("")),
        HbciJobConstraint::new("offset", "Kontoauszug5.offset", Some("")),
    ]
}

fn receipt_constraints() -> Vec<HbciJobConstraint> {
    vec![HbciJobConstraint::new(
        "receipt",
        "Receipt1.receipt",
        Some(""),
    )]
}

fn saldo_req_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.bic", "Saldo7.KTV.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "Saldo7.KTV.iban", None::<String>),
        HbciJobConstraint::new("my.country", "Saldo7.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "Saldo7.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "Saldo7.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "Saldo7.KTV.subnumber", Some("")),
        HbciJobConstraint::new("dummyall", "Saldo7.allaccounts", Some("N")),
        HbciJobConstraint::new("maxentries", "Saldo7.maxentries", Some("")),
    ]
}

fn saldo_req_all_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("maxentries", "Saldo7.maxentries", Some("")),
        HbciJobConstraint::new("dummyall", "Saldo7.allaccounts", Some("J")),
        HbciJobConstraint::new("my.bic", "Saldo7.KTV.bic", None::<String>),
        HbciJobConstraint::new("my.iban", "Saldo7.KTV.iban", None::<String>),
        HbciJobConstraint::new("my.country", "Saldo7.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "Saldo7.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "Saldo7.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "Saldo7.KTV.subnumber", Some("")),
    ]
}

fn status_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("startdate", "Status4.startdate", Some("")),
        HbciJobConstraint::new("enddate", "Status4.enddate", Some("")),
        HbciJobConstraint::new("maxentries", "Status4.maxentries", Some("")),
        HbciJobConstraint::new("jobid", "", Some("")),
    ]
}

fn tan_media_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("mediatype", "TANMediaList4.mediatype", Some("0")),
        HbciJobConstraint::new("mediacategory", "TANMediaList4.mediacategory", Some("A")),
    ]
}

fn vop_auth_constraints() -> Vec<HbciJobConstraint> {
    vec![HbciJobConstraint::new(
        "vopid",
        "VoPAuth1.vopid",
        None::<String>,
    )]
}

fn tan2step_constraints() -> Vec<HbciJobConstraint> {
    let mut constraints = vec![
        HbciJobConstraint::new("process", "TAN2Step5.process", None::<String>),
        HbciJobConstraint::new("ordersegcode", "TAN2Step5.ordersegcode", Some("")),
        HbciJobConstraint::new("orderaccount.bic", "TAN2Step5.OrderAccount.bic", Some("")),
        HbciJobConstraint::new("orderaccount.iban", "TAN2Step5.OrderAccount.iban", Some("")),
        HbciJobConstraint::new(
            "orderaccount.number",
            "TAN2Step5.OrderAccount.number",
            Some(""),
        ),
        HbciJobConstraint::new(
            "orderaccount.subnumber",
            "TAN2Step5.OrderAccount.subnumber",
            Some(""),
        ),
        HbciJobConstraint::new(
            "orderaccount.blz",
            "TAN2Step5.OrderAccount.KIK.blz",
            Some(""),
        ),
        HbciJobConstraint::new(
            "orderaccount.country",
            "TAN2Step5.OrderAccount.KIK.country",
            Some("DE"),
        ),
        HbciJobConstraint::new("orderhash", "TAN2Step5.orderhash", Some("")),
        HbciJobConstraint::new("orderref", "TAN2Step5.orderref", Some("")),
        HbciJobConstraint::new("listidx", "TAN2Step5.listidx", Some("")),
        HbciJobConstraint::new("notlasttan", "TAN2Step5.notlasttan", Some("")),
        HbciJobConstraint::new("storno", "TAN2Step5.storno", Some("")),
        HbciJobConstraint::new("challengeklass", "TAN2Step5.challengeklass", Some("")),
    ];

    constraints.extend((1..=9).map(|index| {
        HbciJobConstraint::new(
            format!("ChallengeKlassParam{index}"),
            format!("TAN2Step5.ChallengeKlassParams.param{index}"),
            Some(""),
        )
    }));
    constraints.push(HbciJobConstraint::new(
        "tanmedia",
        "TAN2Step5.tanmedia",
        Some(""),
    ));

    constraints
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::*;
    use crate::callback::CallbackResponse;

    #[derive(Debug)]
    struct ScriptedCallback {
        events: Arc<Mutex<Vec<CallbackEvent>>>,
        responses: Arc<Mutex<VecDeque<CallbackResponse>>>,
    }

    #[async_trait]
    impl HbciCallback for ScriptedCallback {
        async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
            self.events.lock().expect("callback event lock").push(event);
            Ok(self
                .responses
                .lock()
                .expect("callback response lock")
                .pop_front()
                .unwrap_or_else(CallbackResponse::empty))
        }
    }

    #[tokio::test]
    async fn account_checks_correct_invalid_iban_through_callback_loop() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(VecDeque::from([
            CallbackResponse::value("DE89370400440532013002"),
            CallbackResponse::value("DE89370400440532013000"),
        ])));
        let callback = ScriptedCallback {
            events: events.clone(),
            responses,
        };
        let mut job = saldo_req_with_iban("DE89370400440532013001");

        job.verify_constraints().expect("constraints resolve");
        job.verify_account_checks(Some(&callback))
            .await
            .expect("account checks complete");

        assert_eq!(job.param("my.iban"), Some("DE89370400440532013000"));
        assert_eq!(
            job.lowlevel_param("Saldo7.KTV.iban"),
            Some("DE89370400440532013000")
        );

        let events = events.lock().expect("callback event lock");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].reason, CallbackReason::HaveIbanError);
        assert_eq!(events[0].data_type, CallbackDataType::Text);
        assert_eq!(
            events[0].current_value.as_deref(),
            Some("DE89370400440532013001")
        );
        assert_eq!(
            events[1].current_value.as_deref(),
            Some("DE89370400440532013002")
        );
    }

    #[tokio::test]
    async fn account_checks_accept_unchanged_invalid_iban_like_original() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(VecDeque::new()));
        let callback = ScriptedCallback {
            events: events.clone(),
            responses,
        };
        let mut job = saldo_req_with_iban("DE89370400440532013001");

        job.verify_constraints().expect("constraints resolve");
        job.verify_account_checks(Some(&callback))
            .await
            .expect("account checks complete");

        assert_eq!(job.param("my.iban"), Some("DE89370400440532013001"));
        assert_eq!(
            job.lowlevel_param("Saldo7.KTV.iban"),
            Some("DE89370400440532013001")
        );
        assert_eq!(events.lock().expect("callback event lock").len(), 1);
    }

    fn saldo_req_with_iban(iban: &str) -> HbciJob {
        let mut job = HbciJob::new("SaldoReq");
        job.set_param_account(
            "my",
            &Konto {
                country: Some("DE".to_owned()),
                blz: Some("37040044".to_owned()),
                number: Some("0532013000".to_owned()),
                bic: Some("COBADEFFXXX".to_owned()),
                iban: Some(iban.to_owned()),
                ..Konto::default()
            },
        );
        job
    }
}
