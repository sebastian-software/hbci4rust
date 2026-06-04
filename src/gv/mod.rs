use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{HbciError, HbciErrorKind, HbciResult};

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
}

impl HbciJob {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.params.insert(name.into(), value.into());
    }

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    pub fn params(&self) -> &BTreeMap<String, String> {
        &self.params
    }
}
