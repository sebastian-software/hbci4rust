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
    #[serde(default)]
    constraints: Vec<HbciJobConstraint>,
}

impl HbciJob {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            constraints: constraints_for_job(&name),
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

    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }

    pub fn params(&self) -> &BTreeMap<String, String> {
        &self.params
    }

    pub fn constraints(&self) -> &[HbciJobConstraint] {
        &self.constraints
    }

    pub fn constraint(&self, frontend_name: &str) -> Option<&HbciJobConstraint> {
        self.constraints
            .iter()
            .find(|constraint| constraint.frontend_name == frontend_name)
    }
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
        "SaldoReq" => saldo_req_constraints(),
        "SaldoReqAll" => vec![
            HbciJobConstraint::new("dummyall", "Saldo7.allaccounts", Some("J")),
            HbciJobConstraint::new("maxentries", "Saldo7.maxentries", Some("")),
        ],
        _ => Vec::new(),
    }
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
