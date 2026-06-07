use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::dialog::KnownReturncode;
use crate::gv_result::{HbciMsgStatus, Konto, Limit, Value};
use crate::tools::{ParameterFinder, ParameterQuery, Properties};

pub const ONESTEP_TAN_METHOD_ID: &str = "999";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinTanPassport {
    data: PinTanPassportData,
    #[serde(skip)]
    sca: PinTanScaState,
    #[serde(skip)]
    pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TanMethodSelection {
    Selected(String),
    OneStepFallback,
    NeedsUserSelection(Vec<TanMethodOption>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TanMethodOption {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PinTanScaState {
    pub challenge: Option<String>,
    pub hhd_uc: Option<String>,
    pub order_ref: Option<String>,
    pub sca_exempted: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PinTanScaUpdate {
    pub hitan_found: bool,
    pub sca_exempted: bool,
}

impl PinTanPassport {
    pub fn new(mut data: PinTanPassportData) -> Self {
        if data.twostep_mechanisms.is_empty() && !data.bpd_parameters.is_empty() {
            data.twostep_mechanisms = extract_twostep_mechanisms(&data.bpd_parameters);
        }
        Self {
            data,
            sca: PinTanScaState::default(),
            pin: None,
        }
    }

    pub fn data(&self) -> &PinTanPassportData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut PinTanPassportData {
        &mut self.data
    }

    pub fn sca_state(&self) -> &PinTanScaState {
        &self.sca
    }

    pub fn clear_sca_state(&mut self) {
        self.sca = PinTanScaState::default();
    }

    pub fn pin(&self) -> Option<&str> {
        self.pin.as_deref()
    }

    pub fn set_pin(&mut self, pin: impl Into<String>) {
        self.pin = Some(pin.into());
    }

    pub fn clear_pin(&mut self) {
        self.pin = None;
    }

    pub fn host(&self) -> Option<&str> {
        self.data.host.as_deref()
    }

    pub fn bpd_version(&self) -> &str {
        self.data.bpd_version.as_deref().unwrap_or("0")
    }

    pub fn upd_version(&self) -> &str {
        self.data.upd_version.as_deref().unwrap_or("0")
    }

    pub fn bank_name(&self) -> Option<&str> {
        self.data.bank_name.as_deref()
    }

    pub fn max_gv_per_message(&self) -> Option<u32> {
        self.data.max_gv_per_message
    }

    pub fn max_message_size_kb(&self) -> Option<u32> {
        self.data.max_message_size_kb
    }

    pub fn supported_languages(&self) -> &[String] {
        &self.data.supported_languages
    }

    pub fn supported_hbci_versions(&self) -> &[String] {
        &self.data.supported_hbci_versions
    }

    pub fn upd_usage(&self) -> Option<&str> {
        self.data.upd_usage.as_deref()
    }

    pub fn tan_media(&self) -> Option<&str> {
        self.data.tan_media.as_deref()
    }

    pub fn set_tan_media(&mut self, tan_media: impl Into<String>) {
        self.data.tan_media = Some(tan_media.into());
    }

    pub fn tan_media_names(&self) -> &[String] {
        &self.data.tan_media_names
    }

    pub fn tan_media_names_value(&self) -> String {
        self.data.tan_media_names.join("|")
    }

    pub fn tan_segment_version(&self) -> &str {
        self.current_twostep_mechanism()
            .and_then(|mechanism| mechanism.get("segversion").map(String::as_str))
            .or(self.data.tan_segment_version.as_deref())
            .unwrap_or("5")
    }

    pub fn bpd_parameters(&self) -> &Properties {
        &self.data.bpd_parameters
    }

    pub fn twostep_mechanisms(&self) -> &BTreeMap<String, Properties> {
        &self.data.twostep_mechanisms
    }

    pub fn allowed_twostep_mechanisms(&self) -> &[String] {
        &self.data.allowed_twostep_mechanisms
    }

    pub fn current_tan_method(&self) -> Option<&str> {
        self.data
            .tan_method
            .as_deref()
            .filter(|value| !value.is_empty())
    }

    pub fn set_current_tan_method(&mut self, method: impl Into<String>) {
        self.data.tan_method = Some(method.into());
    }

    pub fn bank_twostep_mechanism_ids(&self) -> Vec<String> {
        self.data.twostep_mechanisms.keys().cloned().collect()
    }

    pub fn allowed_bank_twostep_mechanism_ids(&self) -> Vec<String> {
        self.data
            .twostep_mechanisms
            .keys()
            .filter(|id| self.data.allowed_twostep_mechanisms.contains(id))
            .cloned()
            .collect()
    }

    pub fn one_step_allowed(&self) -> bool {
        if self.data.bpd_parameters.is_empty() {
            return true;
        }

        ParameterFinder::find_all_query(
            &self.data.bpd_parameters,
            &ParameterQuery::BPD_PINTAN_CAN1STEP,
        )
        .map(|values| values.values().any(|value| value == "J"))
        .unwrap_or(false)
    }

    pub fn determine_tan_method(&mut self) -> TanMethodSelection {
        if self.data.allowed_twostep_mechanisms.is_empty() && self.one_step_allowed() {
            return TanMethodSelection::OneStepFallback;
        }

        let user_options = self.allowed_twostep_options();
        let bank_options = self.bank_twostep_options();

        if user_options.is_empty() {
            if self.one_step_allowed() || bank_options.is_empty() {
                return TanMethodSelection::OneStepFallback;
            }
            return TanMethodSelection::NeedsUserSelection(bank_options);
        }

        if user_options.len() == 1 {
            let selected = user_options[0].id.clone();
            self.data.tan_method = Some(selected.clone());
            return TanMethodSelection::Selected(selected);
        }

        if let Some(current) = self.current_tan_method().map(str::to_owned)
            && user_options.iter().any(|option| option.id == current)
        {
            return TanMethodSelection::Selected(current);
        }

        TanMethodSelection::NeedsUserSelection(user_options)
    }

    pub fn tan_media_required(&self) -> bool {
        let version = self.tan_segment_version().parse::<u32>().unwrap_or(0);
        version >= 3
            && self
                .current_secmech_info()
                .get("needtanmedia")
                .is_some_and(|value| value == "2")
    }

    pub fn tan_media_for_hktan_without_callback(&self) -> Option<String> {
        if let Some(tan_media) = self.tan_media().filter(|value| !value.is_empty()) {
            return Some(tan_media.to_owned());
        }
        if self.tan_media_required() {
            return Some("noref".to_owned());
        }
        None
    }

    pub fn update_sca_state_from_response_values(
        &mut self,
        values: &BTreeMap<String, String>,
        message_prefix: &str,
        status: &HbciMsgStatus,
    ) -> PinTanScaUpdate {
        if status
            .return_value_for_code(KnownReturncode::W3076)
            .is_some()
        {
            self.sca = PinTanScaState {
                sca_exempted: true,
                ..PinTanScaState::default()
            };
            return PinTanScaUpdate {
                hitan_found: false,
                sca_exempted: true,
            };
        }

        let Some(root) = tan2step_response_roots(values, message_prefix)
            .into_iter()
            .next()
        else {
            return PinTanScaUpdate::default();
        };

        self.sca.sca_exempted = false;
        if let Some(challenge) = optional_value(values, &format!("{root}.challenge"))
            && challenge != "nochallenge"
        {
            self.sca.challenge = Some(challenge);
        }
        if let Some(hhd_uc) = optional_value(values, &format!("{root}.challenge_hhd_uc")) {
            self.sca.hhd_uc = Some(hhd_uc);
        }
        if let Some(order_ref) = optional_value(values, &format!("{root}.orderref")) {
            self.sca.order_ref = Some(order_ref);
        }

        PinTanScaUpdate {
            hitan_found: true,
            sca_exempted: false,
        }
    }

    pub fn only_bpd_gvs(&self) -> bool {
        self.upd_usage() == Some("0")
    }

    pub fn user_name(&self) -> Option<&str> {
        self.data.user_name.as_deref()
    }

    pub fn customer_id(&self) -> &str {
        self.data
            .customer_id
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(&self.data.user_id)
    }

    pub fn accounts(&self) -> &[Konto] {
        &self.data.accounts
    }

    pub fn first_account(&self) -> Option<&Konto> {
        self.accounts().first()
    }

    pub fn account_by_number(&self, number: impl Into<String>) -> Konto {
        let mut account = Konto {
            number: Some(number.into()),
            curr: Some("EUR".to_owned()),
            ..Konto::default()
        };
        self.fill_account_info(&mut account);

        if account.blz.as_deref().is_none_or(str::is_empty) {
            if !self.data.blz.is_empty() {
                account.blz = Some(self.data.blz.clone());
            }
            if !self.data.country.is_empty() {
                account.country = Some(self.data.country.clone());
            }
            let customer_id = self.customer_id();
            if !customer_id.is_empty() {
                account.customer_id = Some(customer_id.to_owned());
                account.name = Some(customer_id.to_owned());
            }
        }

        account
    }

    pub fn fill_account_info(&self, account: &mut Konto) {
        let have_number = account
            .number
            .as_deref()
            .map(strip_leading_zeroes)
            .is_some_and(|number| !number.is_empty());
        let number = account.number.as_deref().map(strip_leading_zeroes);
        let subnumber = account.subnumber.as_deref().map(strip_leading_zeroes);
        let iban = account.iban.as_deref().map(strip_leading_zeroes);

        for candidate in self.accounts() {
            let candidate_number = candidate.number.as_deref().map(strip_leading_zeroes);
            let candidate_subnumber = candidate.subnumber.as_deref().map(strip_leading_zeroes);
            let candidate_iban = candidate.iban.as_deref().map(strip_leading_zeroes);

            let number_matches = have_number
                && number == candidate_number
                && (subnumber.as_deref().is_none_or(str::is_empty)
                    || subnumber == candidate_subnumber);
            let iban_matches =
                iban.as_deref().is_some_and(|iban| !iban.is_empty()) && iban == candidate_iban;

            if number_matches || iban_matches {
                fill_from_account(account, candidate);
                break;
            }
        }
    }

    pub fn update_accounts_from_values(
        &mut self,
        values: &BTreeMap<String, String>,
        prefix: &str,
    ) -> usize {
        let mut accounts = accounts_from_values(values, prefix);
        if accounts.is_empty() {
            return 0;
        }

        preserve_missing_sepa_fields(&mut accounts, &self.data.accounts);
        let count = accounts.len();
        self.data.accounts = accounts;
        count
    }

    pub fn update_parameter_versions_from_values(
        &mut self,
        values: &BTreeMap<String, String>,
        prefix: &str,
    ) -> usize {
        let mut updated = 0;

        if let Some(version) = optional_value(values, &format!("{prefix}.BPD.BPA.version")) {
            self.data.bpd_version = Some(version);
            updated += 1;
        }
        if let Some(version) = optional_value(values, &format!("{prefix}.UPD.UPA.version")) {
            self.data.upd_version = Some(version);
            updated += 1;
        }

        updated
    }

    pub fn update_parameter_data_from_values(
        &mut self,
        values: &BTreeMap<String, String>,
        prefix: &str,
    ) -> usize {
        let mut updated = self.update_parameter_versions_from_values(values, prefix);
        let bpa_prefix = format!("{prefix}.BPD.BPA");
        let upa_prefix = format!("{prefix}.UPD.UPA");

        if let Some(bank_name) = optional_value(values, &format!("{bpa_prefix}.kiname")) {
            self.data.bank_name = Some(bank_name);
            updated += 1;
        }
        if let Some(max_gv_per_message) = optional_u32(values, &format!("{bpa_prefix}.numgva")) {
            self.data.max_gv_per_message = Some(max_gv_per_message);
            updated += 1;
        }
        if let Some(max_message_size_kb) = optional_u32(values, &format!("{bpa_prefix}.maxmsgsize"))
        {
            self.data.max_message_size_kb = Some(max_message_size_kb);
            updated += 1;
        }
        let bpd_parameters = prefixed_values(values, &format!("{prefix}.BPD."));
        if !bpd_parameters.is_empty() {
            self.data.twostep_mechanisms = extract_twostep_mechanisms(&bpd_parameters);
            self.data.bpd_parameters = bpd_parameters;
            updated += 1;
        }

        let supported_languages =
            counted_value_keys(values, &format!("{bpa_prefix}.SuppLangs.lang"))
                .into_iter()
                .filter_map(|key| optional_value(values, &key))
                .collect::<Vec<_>>();
        if !supported_languages.is_empty() {
            self.data.supported_languages = supported_languages;
            updated += 1;
        }

        let supported_hbci_versions =
            counted_value_keys(values, &format!("{bpa_prefix}.SuppVersions.version"))
                .into_iter()
                .filter_map(|key| optional_value(values, &key))
                .collect::<Vec<_>>();
        if !supported_hbci_versions.is_empty() {
            self.data.supported_hbci_versions = supported_hbci_versions;
            updated += 1;
        }

        if let Some(usage) = optional_value(values, &format!("{upa_prefix}.usage")) {
            self.data.upd_usage = Some(usage);
            updated += 1;
        }
        if let Some(user_name) = optional_value(values, &format!("{upa_prefix}.username")) {
            self.data.user_name = Some(user_name);
            updated += 1;
        }
        if let Some(tan_media_names) =
            optional_value(values, &format!("{prefix}.UPD.tanmedia.names"))
        {
            self.data.tan_media_names = split_pipe_values(&tan_media_names);
            updated += 1;
        }

        updated
    }

    pub fn update_allowed_twostep_mechanisms_from_status(
        &mut self,
        status: &HbciMsgStatus,
    ) -> usize {
        let mut mechanisms = status
            .return_values_for_code(KnownReturncode::W3920)
            .into_iter()
            .flat_map(|value| value.params.iter())
            .filter(|value| !value.is_empty())
            .cloned()
            .collect::<Vec<_>>();
        mechanisms.sort();
        mechanisms.dedup();

        if mechanisms.is_empty() || mechanisms == self.data.allowed_twostep_mechanisms {
            return 0;
        }

        let updated = mechanisms.len();
        self.data.allowed_twostep_mechanisms = mechanisms;
        updated
    }

    pub fn tan2step_parameter(&self, name: &str) -> Option<String> {
        if let Some(value) = self
            .current_twostep_mechanism()
            .and_then(|mechanism| mechanism.get(name))
            .filter(|value| !value.is_empty())
        {
            return Some(value.clone());
        }

        let path = format!(
            "Params*.TAN2StepPar{}.ParTAN2Step*.{name}",
            self.tan_segment_version()
        );
        ParameterFinder::get_value(self.bpd_parameters(), Some(&path), None)
    }

    pub fn order_hash_mode_code(&self) -> Option<String> {
        if let Some(value) = self
            .current_twostep_mechanism()
            .and_then(|mechanism| mechanism.get("orderhashmode"))
            .filter(|value| !value.is_empty())
        {
            return Some(value.clone());
        }

        let query =
            ParameterQuery::BPD_PINTAN_ORDERHASHMODE.with_parameters(&[self.tan_segment_version()]);
        ParameterFinder::get_value_query(self.bpd_parameters(), &query, None)
            .ok()
            .flatten()
    }

    pub fn current_secmech_info(&self) -> Properties {
        if let Some(mechanism) = self.current_twostep_mechanism() {
            let mut info = mechanism.clone();
            if !info.contains_key("segversion") {
                info.insert(
                    "segversion".to_owned(),
                    self.data
                        .tan_segment_version
                        .as_deref()
                        .unwrap_or("5")
                        .to_owned(),
                );
            }
            if let Some(tan_method) = self.current_tan_method() {
                info.insert("secfunc".to_owned(), tan_method.to_owned());
            }
            return info;
        }

        let mut info = Properties::new();
        info.insert(
            "segversion".to_owned(),
            self.tan_segment_version().to_owned(),
        );

        if let Some(tan_method) = self
            .data
            .tan_method
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            info.insert("secfunc".to_owned(), tan_method.to_owned());
        }

        for name in [
            "id",
            "needchallengeklass",
            "needorderaccount",
            "orderhashmode",
            "process",
            "zkamethod_name",
            "zkamethod_version",
        ] {
            if let Some(value) = self.tan2step_parameter(name) {
                info.insert(name.to_owned(), value);
            }
        }

        info
    }

    fn current_twostep_mechanism(&self) -> Option<&Properties> {
        self.current_tan_method()
            .and_then(|tan_method| self.data.twostep_mechanisms.get(tan_method))
    }

    fn bank_twostep_options(&self) -> Vec<TanMethodOption> {
        self.data
            .twostep_mechanisms
            .iter()
            .map(|(id, mechanism)| TanMethodOption::from_mechanism(id, mechanism))
            .collect()
    }

    fn allowed_twostep_options(&self) -> Vec<TanMethodOption> {
        self.data
            .twostep_mechanisms
            .iter()
            .filter(|(id, _)| self.data.allowed_twostep_mechanisms.contains(id))
            .map(|(id, mechanism)| TanMethodOption::from_mechanism(id, mechanism))
            .collect()
    }
}

impl TanMethodOption {
    fn from_mechanism(id: &str, mechanism: &Properties) -> Self {
        Self {
            id: id.to_owned(),
            name: mechanism
                .get("name")
                .filter(|value| !value.is_empty())
                .cloned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinTanPassportData {
    pub country: String,
    pub blz: String,
    pub host: Option<String>,
    pub user_id: String,
    pub customer_id: Option<String>,
    pub filter: Option<String>,
    pub tan_method: Option<String>,
    pub tan_media: Option<String>,
    #[serde(default)]
    pub tan_media_names: Vec<String>,
    #[serde(default)]
    pub tan_segment_version: Option<String>,
    #[serde(default)]
    pub bpd_version: Option<String>,
    #[serde(default)]
    pub upd_version: Option<String>,
    #[serde(default)]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub max_gv_per_message: Option<u32>,
    #[serde(default)]
    pub max_message_size_kb: Option<u32>,
    #[serde(default)]
    pub supported_languages: Vec<String>,
    #[serde(default)]
    pub supported_hbci_versions: Vec<String>,
    #[serde(default)]
    pub upd_usage: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
    #[serde(default)]
    pub accounts: Vec<Konto>,
    #[serde(default)]
    pub bpd_parameters: Properties,
    #[serde(default)]
    pub twostep_mechanisms: BTreeMap<String, Properties>,
    #[serde(default)]
    pub allowed_twostep_mechanisms: Vec<String>,
}

fn fill_from_account(account: &mut Konto, source: &Konto) {
    copy_non_empty(&mut account.country, &source.country);
    copy_non_empty(&mut account.blz, &source.blz);
    copy_non_empty(&mut account.number, &source.number);
    copy_non_empty(&mut account.subnumber, &source.subnumber);
    copy_non_empty(&mut account.bic, &source.bic);
    copy_non_empty(&mut account.iban, &source.iban);
    copy_non_empty(&mut account.customer_id, &source.customer_id);
    copy_non_empty(&mut account.name, &source.name);
    copy_non_empty(&mut account.name2, &source.name2);
    copy_non_empty(&mut account.acctype, &source.acctype);
    copy_non_empty(&mut account.account_type, &source.account_type);
    copy_non_empty(&mut account.curr, &source.curr);
    if account.limit.is_none() {
        account.limit = source.limit.clone();
    }
    if !source.allowed_gvs.is_empty() {
        account.allowed_gvs = source.allowed_gvs.clone();
    }
}

fn copy_non_empty(target: &mut Option<String>, source: &Option<String>) {
    if source.as_deref().is_some_and(|value| !value.is_empty()) {
        *target = source.clone();
    }
}

fn strip_leading_zeroes(value: &str) -> String {
    value.trim_start_matches('0').to_owned()
}

fn accounts_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Vec<Konto> {
    counted_prefixes(values, &format!("{prefix}.KInfo"))
        .into_iter()
        .filter_map(|prefix| account_from_values(values, &prefix))
        .collect()
}

fn account_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<Konto> {
    let number = optional_value(values, &format!("{prefix}.KTV.number"))?;

    Some(Konto {
        country: optional_value(values, &format!("{prefix}.KTV.KIK.country")),
        blz: optional_value(values, &format!("{prefix}.KTV.KIK.blz")),
        number: Some(number),
        subnumber: optional_value(values, &format!("{prefix}.KTV.subnumber")),
        bic: optional_value(values, &format!("{prefix}.KTV.bic"))
            .or_else(|| optional_value(values, &format!("{prefix}.bic"))),
        iban: optional_value(values, &format!("{prefix}.KTV.iban"))
            .or_else(|| optional_value(values, &format!("{prefix}.iban"))),
        customer_id: optional_value(values, &format!("{prefix}.customerid")),
        name: optional_value(values, &format!("{prefix}.name1")),
        name2: optional_value(values, &format!("{prefix}.name2")),
        creditorid: None,
        acctype: optional_value(values, &format!("{prefix}.acctype")),
        account_type: optional_value(values, &format!("{prefix}.konto")),
        curr: optional_value(values, &format!("{prefix}.cur")),
        limit: limit_from_values(values, &format!("{prefix}.KLimit")),
        allowed_gvs: counted_prefixes(values, &format!("{prefix}.AllowedGV"))
            .into_iter()
            .filter_map(|prefix| optional_value(values, &format!("{prefix}.code")))
            .collect(),
    })
}

fn limit_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<Limit> {
    Some(Limit {
        limit_type: optional_value(values, &format!("{prefix}.limittype"))?,
        value: value_from_values(values, &format!("{prefix}.BTG")),
        days: optional_u32(values, &format!("{prefix}.limitdays")),
    })
}

fn value_from_values(values: &BTreeMap<String, String>, prefix: &str) -> Option<Value> {
    values.get(&format!("{prefix}.value")).map(|value| Value {
        value: value.to_owned(),
        curr: optional_value(values, &format!("{prefix}.curr")),
    })
}

fn preserve_missing_sepa_fields(accounts: &mut [Konto], previous_accounts: &[Konto]) {
    for account in accounts {
        if account
            .iban
            .as_deref()
            .is_some_and(|value| !value.is_empty())
            && account
                .bic
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        {
            continue;
        }

        if let Some(previous) = previous_accounts
            .iter()
            .find(|previous| accounts_match_by_number_and_bank(account, previous))
        {
            if account.iban.as_deref().is_none_or(str::is_empty) {
                copy_non_empty(&mut account.iban, &previous.iban);
            }
            if account.bic.as_deref().is_none_or(str::is_empty) {
                copy_non_empty(&mut account.bic, &previous.bic);
            }
        }
    }
}

fn accounts_match_by_number_and_bank(account: &Konto, previous: &Konto) -> bool {
    account.number == previous.number
        && account.blz == previous.blz
        && account.country == previous.country
}

fn counted_prefixes(values: &BTreeMap<String, String>, base: &str) -> Vec<String> {
    let mut prefixes = Vec::new();
    let mut index = 1;

    loop {
        let prefix = if index == 1 {
            base.to_owned()
        } else {
            format!("{base}_{index}")
        };
        let child_prefix = format!("{prefix}.");
        if !values.keys().any(|key| key.starts_with(&child_prefix)) {
            break;
        }

        prefixes.push(prefix);
        index += 1;
    }

    prefixes
}

fn counted_value_keys(values: &BTreeMap<String, String>, base: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let mut index = 1;

    loop {
        let key = if index == 1 {
            base.to_owned()
        } else {
            format!("{base}_{index}")
        };
        if !values.contains_key(&key) {
            break;
        }

        keys.push(key);
        index += 1;
    }

    keys
}

fn optional_u32(values: &BTreeMap<String, String>, key: &str) -> Option<u32> {
    optional_value(values, key).and_then(|value| value.parse().ok())
}

fn optional_value(values: &BTreeMap<String, String>, key: &str) -> Option<String> {
    values.get(key).and_then(|value| {
        if value.is_empty() {
            None
        } else {
            Some(value.to_owned())
        }
    })
}

fn split_pipe_values(value: &str) -> Vec<String> {
    value
        .split('|')
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn tan2step_response_roots(values: &BTreeMap<String, String>, message_prefix: &str) -> Vec<String> {
    let message_prefix = format!("{message_prefix}.");
    let mut roots = values
        .keys()
        .filter(|key| key.starts_with(&message_prefix))
        .filter_map(|key| {
            let parts = key.split('.').collect::<Vec<_>>();
            let index = parts.iter().position(|part| {
                part.split_once('_')
                    .map(|(base, _)| base)
                    .unwrap_or(part)
                    .starts_with("TAN2StepRes")
            })?;
            Some(parts[..=index].join("."))
        })
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots
}

fn prefixed_values(values: &BTreeMap<String, String>, prefix: &str) -> Properties {
    values
        .iter()
        .filter_map(|(key, value)| {
            key.strip_prefix(prefix)
                .map(|key| (key.to_owned(), value.clone()))
        })
        .collect()
}

fn extract_twostep_mechanisms(bpd: &Properties) -> BTreeMap<String, Properties> {
    let mut mechanisms = BTreeMap::new();

    for (secfunc, segversion, header) in bpd
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .filter_map(|(key, value)| {
            tan2step_secfunc_header(key).map(|(segversion, header)| (value, segversion, header))
        })
    {
        if let Some(previous) = mechanisms.get(secfunc)
            && mechanism_segversion(previous) > segversion
        {
            continue;
        }

        let entry = twostep_mechanism_entry(bpd, &header);
        mechanisms.insert(secfunc.clone(), entry);
    }

    mechanisms
}

fn twostep_mechanism_entry(bpd: &Properties, header: &str) -> Properties {
    let mut entry = Properties::new();
    if let Some((segversion, _)) = tan2step_header_segment_version(header) {
        entry.insert("segversion".to_owned(), segversion.to_string());
    }

    let prefix = format!("{header}.");
    for (key, value) in bpd
        .iter()
        .filter(|(key, _)| key.starts_with(prefix.as_str()))
    {
        if let Some(name) = key.rsplit('.').next() {
            entry.insert(name.to_owned(), value.clone());
        }
    }

    entry
}

fn tan2step_secfunc_header(key: &str) -> Option<(i32, String)> {
    if !key.starts_with("Params") || !key.ends_with(".secfunc") {
        return None;
    }

    let header = key.rsplit_once('.')?.0.to_owned();
    let (segversion, _) = tan2step_header_segment_version(&header)?;
    Some((segversion, header))
}

fn tan2step_header_segment_version(header: &str) -> Option<(i32, &str)> {
    let subkey = header.split_once('.')?.1;
    let rest = subkey.strip_prefix("TAN2StepPar")?;
    let digits_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits_len == 0 {
        return None;
    }

    let segversion = rest.get(..digits_len)?.parse().ok()?;
    let after_version = rest.get(digits_len..)?;
    if !after_version.starts_with(".ParTAN2Step") {
        return None;
    }

    Some((segversion, after_version))
}

fn mechanism_segversion(mechanism: &Properties) -> i32 {
    mechanism
        .get("segversion")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}
