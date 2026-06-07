use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::callback::{CallbackDataType, CallbackEvent, CallbackReason, HbciCallback};
use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv_result::{Konto, Value};
use crate::protocol::normalize_iso_date;
use crate::sepa::{CAMT_052_001_01_URN, PAIN_001_001_02_URN};

pub const PINTAN_JOB_NAMES: &[&str] = &[
    "AccInfo",
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
        self.params.insert(name.into(), value.into());
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

    pub(crate) async fn verify_account_checks(
        &mut self,
        callback: Option<&dyn HbciCallback>,
    ) -> HbciResult<()> {
        match self.name.as_str() {
            "KUmsAll" | "KUmsAllCamt" | "KUmsNew" | "SaldoReq" | "SaldoReqAll" => {
                self.check_account_crc("my", callback).await
            }
            _ => Ok(()),
        }
    }

    fn resolved_constraint_value(
        &self,
        constraint: &HbciJobConstraint,
    ) -> HbciResult<Option<ResolvedConstraintValue>> {
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

        value.to_owned()
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

fn indexed_destination_name(destination: &str, index: usize) -> String {
    let parts = destination.split('.').collect::<Vec<_>>();
    if !matches!(parts.len(), 3 | 4) || !parts.iter().all(|part| is_word_part(part)) {
        return destination.to_owned();
    }

    let base = format!("{}.{}.{}[{index}]", parts[0], parts[1], parts[2]);
    if parts.len() == 4 {
        format!("{base}.{}", parts[3])
    } else {
        base
    }
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
        "DauerSEPAEdit" => dauer_sepa_edit_constraints(),
        "DauerSEPAList" => dauer_sepa_list_constraints(),
        "DauerSEPANew" => dauer_sepa_new_constraints(),
        "KUmsAll" => kums_all_constraints(),
        "KUmsAllCamt" => kums_all_camt_constraints(),
        "KUmsNew" => kums_new_constraints(),
        "SaldoReq" => saldo_req_constraints(),
        "SaldoReqAll" => saldo_req_all_constraints(),
        "TANMediaList" => tan_media_list_constraints(),
        "TAN2Step" => tan2step_constraints(),
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

fn acc_info_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("my.country", "AccInfo2.KTV.KIK.country", Some("DE")),
        HbciJobConstraint::new("my.blz", "AccInfo2.KTV.KIK.blz", None::<String>),
        HbciJobConstraint::new("my.number", "AccInfo2.KTV.number", None::<String>),
        HbciJobConstraint::new("my.subnumber", "AccInfo2.KTV.subnumber", Some("")),
        HbciJobConstraint::new("all", "AccInfo2.allaccounts", Some("N")),
    ]
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

fn tan_media_list_constraints() -> Vec<HbciJobConstraint> {
    vec![
        HbciJobConstraint::new("mediatype", "TANMediaList4.mediatype", Some("0")),
        HbciJobConstraint::new("mediacategory", "TANMediaList4.mediacategory", Some("A")),
    ]
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
