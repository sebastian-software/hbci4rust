use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::gv_result::{Konto, Limit, Value};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinTanPassport {
    data: PinTanPassportData,
}

impl PinTanPassport {
    pub fn new(data: PinTanPassportData) -> Self {
        Self { data }
    }

    pub fn data(&self) -> &PinTanPassportData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut PinTanPassportData {
        &mut self.data
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

    pub fn only_bpd_gvs(&self) -> bool {
        self.upd_usage() == Some("0")
    }

    pub fn user_name(&self) -> Option<&str> {
        self.data.user_name.as_deref()
    }

    pub fn accounts(&self) -> &[Konto] {
        &self.data.accounts
    }

    pub fn first_account(&self) -> Option<&Konto> {
        self.accounts().first()
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

        updated
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
