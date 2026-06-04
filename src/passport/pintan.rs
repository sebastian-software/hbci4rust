use serde::{Deserialize, Serialize};

use crate::gv_result::Konto;

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
    pub accounts: Vec<Konto>,
}

fn fill_from_account(account: &mut Konto, source: &Konto) {
    copy_non_empty(&mut account.country, &source.country);
    copy_non_empty(&mut account.blz, &source.blz);
    copy_non_empty(&mut account.number, &source.number);
    copy_non_empty(&mut account.subnumber, &source.subnumber);
    copy_non_empty(&mut account.bic, &source.bic);
    copy_non_empty(&mut account.iban, &source.iban);
    copy_non_empty(&mut account.account_type, &source.account_type);
    copy_non_empty(&mut account.curr, &source.curr);
}

fn copy_non_empty(target: &mut Option<String>, source: &Option<String>) {
    if source.as_deref().is_some_and(|value| !value.is_empty()) {
        *target = source.clone();
    }
}

fn strip_leading_zeroes(value: &str) -> String {
    value.trim_start_matches('0').to_owned()
}
