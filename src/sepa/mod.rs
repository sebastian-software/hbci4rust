use std::time::{SystemTime, UNIX_EPOCH};

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use serde::{Deserialize, Serialize};

use crate::error::{HbciError, HbciErrorKind, HbciResult};
use crate::gv_result::{GvrKUmsBTag, GvrKUmsLine, Konto, Saldo, Value};
use crate::tools::Properties;

pub const DATE_FORMAT: &str = "%Y-%m-%d";
pub const DATE_UNDEFINED: &str = "1999-01-01";
pub const DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
pub const ENDTOEND_ID_NOTPROVIDED: &str = "NOTPROVIDED";
pub const PAIN_001_001_02_URN: &str = "urn:sepade:xsd:pain.001.001.02";
pub const PAIN_001_001_02_SCHEMA_LOCATION: &str =
    "urn:sepade:xsd:pain.001.001.02 pain.001.001.02.xsd";
pub const PAIN_008_001_01_URN: &str = "urn:sepade:xsd:pain.008.001.01";
pub const PAIN_008_001_01_SCHEMA_LOCATION: &str =
    "urn:sepade:xsd:pain.008.001.01 pain.008.001.01.xsd";
pub const PAIN_008_001_02_URN: &str = "urn:iso:std:iso:20022:tech:xsd:pain.008.001.02";
pub const PAIN_008_001_02_SCHEMA_LOCATION: &str =
    "urn:iso:std:iso:20022:tech:xsd:pain.008.001.02 pain.008.001.02.xsd";
pub const CAMT_052_001_01_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.01";
pub const CAMT_052_001_02_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.02";
pub const CAMT_052_001_03_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.03";
pub const CAMT_052_001_04_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.04";
pub const CAMT_052_001_05_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.05";
pub const CAMT_052_001_06_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.06";
pub const CAMT_052_001_07_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.07";
pub const CAMT_052_001_08_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.08";
pub const CAMT_052_001_09_URN: &str = "urn:iso:std:iso:20022:tech:xsd:camt.052.001.09";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pain001Transfer {
    pub source: Konto,
    pub destination: Konto,
    pub value: Option<Value>,
    pub usage: Vec<String>,
    pub execution_date: Option<String>,
    pub end_to_end_id: Option<String>,
    pub payment_info_id: Option<String>,
    pub purpose_code: Option<String>,
    pub batch_book: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pain008DirectDebit {
    pub creditor: Konto,
    pub debtor: Konto,
    pub value: Option<Value>,
    pub usage: Vec<String>,
    pub collection_date: Option<String>,
    pub end_to_end_id: Option<String>,
    pub payment_info_id: Option<String>,
    pub purpose_code: Option<String>,
    pub debit_type: Option<String>,
    pub sequence_type: Option<String>,
    pub creditor_id: Option<String>,
    pub mandate_id: Option<String>,
    pub mandate_date_of_signature: Option<String>,
    pub batch_book: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SepaKind {
    Pain001,
    Pain008,
    Camt052,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SepaVersion {
    kind: SepaKind,
    major: u16,
    minor: u16,
    urn: &'static str,
    schema_file: Option<&'static str>,
    order: u16,
}

impl SepaVersion {
    pub const PAIN_001_001_02: Self = Self::pain(
        SepaKind::Pain001,
        1,
        2,
        1,
        PAIN_001_001_02_URN,
        "pain.001.001.02.xsd",
    );
    pub const PAIN_008_001_01: Self = Self::pain(
        SepaKind::Pain008,
        1,
        1,
        1,
        PAIN_008_001_01_URN,
        "pain.008.001.01.xsd",
    );
    pub const PAIN_008_001_02: Self = Self::pain(
        SepaKind::Pain008,
        1,
        2,
        5,
        PAIN_008_001_02_URN,
        "pain.008.001.02.xsd",
    );
    pub const CAMT_052_001_01: Self = Self::camt(1, CAMT_052_001_01_URN, "camt.052.001.01.xsd");
    pub const CAMT_052_001_02: Self = Self::camt(2, CAMT_052_001_02_URN, "camt.052.001.02.xsd");
    pub const CAMT_052_001_03: Self = Self::camt(3, CAMT_052_001_03_URN, "camt.052.001.03.xsd");
    pub const CAMT_052_001_04: Self = Self::camt(4, CAMT_052_001_04_URN, "camt.052.001.04.xsd");
    pub const CAMT_052_001_05: Self = Self::camt(5, CAMT_052_001_05_URN, "camt.052.001.05.xsd");
    pub const CAMT_052_001_06: Self = Self::camt(6, CAMT_052_001_06_URN, "camt.052.001.06.xsd");
    pub const CAMT_052_001_07: Self = Self::camt(7, CAMT_052_001_07_URN, "camt.052.001.07.xsd");
    pub const CAMT_052_001_08: Self = Self::camt(8, CAMT_052_001_08_URN, "camt.052.001.08.xsd");
    pub const CAMT_052_001_09: Self = Self::camt(9, CAMT_052_001_09_URN, "camt.052.001.09.xsd");

    const fn pain(
        kind: SepaKind,
        major: u16,
        minor: u16,
        order: u16,
        urn: &'static str,
        schema_file: &'static str,
    ) -> Self {
        Self {
            kind,
            major,
            minor,
            urn,
            schema_file: Some(schema_file),
            order,
        }
    }

    const fn camt(order: u16, urn: &'static str, schema_file: &'static str) -> Self {
        Self {
            kind: SepaKind::Camt052,
            major: 1,
            minor: order,
            urn,
            schema_file: Some(schema_file),
            order,
        }
    }

    pub const fn kind(self) -> SepaKind {
        self.kind
    }

    pub const fn major(self) -> u16 {
        self.major
    }

    pub const fn minor(self) -> u16 {
        self.minor
    }

    pub const fn urn(self) -> &'static str {
        self.urn
    }

    pub const fn schema_file(self) -> Option<&'static str> {
        self.schema_file
    }

    pub fn schema_location(self) -> Option<String> {
        self.schema_file
            .map(|schema_file| format!("{} {}", self.urn, schema_file))
    }

    pub fn known_camt_versions() -> &'static [Self] {
        &[
            Self::CAMT_052_001_01,
            Self::CAMT_052_001_02,
            Self::CAMT_052_001_03,
            Self::CAMT_052_001_04,
            Self::CAMT_052_001_05,
            Self::CAMT_052_001_06,
            Self::CAMT_052_001_07,
            Self::CAMT_052_001_08,
            Self::CAMT_052_001_09,
        ]
    }

    pub fn known_versions() -> &'static [Self] {
        &[
            Self::PAIN_001_001_02,
            Self::PAIN_008_001_01,
            Self::PAIN_008_001_02,
            Self::CAMT_052_001_01,
            Self::CAMT_052_001_02,
            Self::CAMT_052_001_03,
            Self::CAMT_052_001_04,
            Self::CAMT_052_001_05,
            Self::CAMT_052_001_06,
            Self::CAMT_052_001_07,
            Self::CAMT_052_001_08,
            Self::CAMT_052_001_09,
        ]
    }

    pub fn by_urn(urn: &str) -> Option<Self> {
        Self::known_versions()
            .iter()
            .copied()
            .find(|version| version.urn == urn)
    }

    pub fn find_greatest(versions: &[Self]) -> Option<Self> {
        versions
            .iter()
            .copied()
            .max_by_key(|version| (version.kind, version.order, version.major, version.minor))
    }

    pub fn autodetect(xml: &str) -> HbciResult<Option<Self>> {
        let namespace = root_namespace(xml)?;
        match namespace {
            None => Ok(None),
            Some(namespace) => Self::by_urn(&namespace)
                .map(Some)
                .ok_or_else(|| invalid_sepa_namespace(namespace)),
        }
    }

    pub fn choose(descriptor: Option<&str>, data: Option<&str>) -> HbciResult<Option<Self>> {
        let descriptor_version = descriptor
            .filter(|value| !value.is_empty())
            .and_then(Self::by_urn);
        let data_version = match data.filter(|value| !value.is_empty()) {
            Some(data) => Self::autodetect(data)?,
            None => None,
        };

        Ok(data_version.or(descriptor_version))
    }
}

fn invalid_sepa_namespace(namespace: String) -> HbciError {
    HbciError::new(
        HbciErrorKind::InvalidArgument,
        format!("invalid sepa-version: {namespace}"),
    )
}

pub fn parse_camt_report_shell(xml: &str, version: SepaVersion) -> HbciResult<Vec<GvrKUmsBTag>> {
    if version.kind != SepaKind::Camt052 {
        return Err(HbciError::new(
            HbciErrorKind::InvalidArgument,
            "CAMT report shell parser requires a CAMT.052 version",
        ));
    }

    let reports = parse_camt_reports(xml)?;
    Ok(reports.into_iter().map(GvrKUmsBTag::from).collect())
}

pub fn parse_pain_001_transfers(xml: &str) -> HbciResult<Vec<Pain001Transfer>> {
    parse_pain_001_transfer_shell(xml)
}

pub fn parse_pain_008_direct_debits(xml: &str) -> HbciResult<Vec<Pain008DirectDebit>> {
    parse_pain_008_direct_debit_shell(xml)
}

pub fn generate_pain_001_001_02_transfer(sepa_params: &Properties) -> HbciResult<String> {
    let sepa_id = text_or_generated_message_id(sepa_params.get("sepaid").map(String::as_str));
    let pmt_inf_id = text_or_default(
        sepa_params.get("pmtinfid").map(String::as_str),
        sepa_id.as_str(),
    );
    let execution_date =
        text_or_default(sepa_params.get("date").map(String::as_str), DATE_UNDEFINED);
    let end_to_end_id = text_or_default(
        sepa_params.get("endtoendid").map(String::as_str),
        ENDTOEND_ID_NOTPROVIDED,
    );
    let currency = text_or_default(sepa_params.get("btg.curr").map(String::as_str), "EUR");
    let source_name = required_text(sepa_params, "src.name")?;
    let source_iban = required_text(sepa_params, "src.iban")?;
    let source_bic = required_text(sepa_params, "src.bic")?;
    let destination_name = required_text(sepa_params, "dst.name")?;
    let destination_iban = required_text(sepa_params, "dst.iban")?;
    let destination_bic = sepa_params
        .get("dst.bic")
        .map(String::as_str)
        .unwrap_or_default();
    let amount = required_text(sepa_params, "btg.value")?;
    let usage = sepa_params
        .get("usage")
        .map(String::as_str)
        .unwrap_or_default();
    let creation_datetime = current_xml_datetime();

    let mut writer = Writer::new(Vec::new());
    write_xml_event(
        &mut writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )?;

    let mut document = BytesStart::new("Document");
    document.push_attribute(("xmlns", PAIN_001_001_02_URN));
    document.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    document.push_attribute(("xsi:schemaLocation", PAIN_001_001_02_SCHEMA_LOCATION));
    write_xml_event(&mut writer, Event::Start(document))?;
    write_start(&mut writer, "pain.001.001.02")?;

    write_start(&mut writer, "GrpHdr")?;
    write_text_element(&mut writer, "MsgId", &sepa_id)?;
    write_text_element(&mut writer, "CreDtTm", &creation_datetime)?;
    write_text_element(&mut writer, "NbOfTxs", "1")?;
    write_text_element(&mut writer, "CtrlSum", amount)?;
    write_text_element(&mut writer, "Grpg", "GRPD")?;
    write_start(&mut writer, "InitgPty")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "InitgPty")?;
    write_end(&mut writer, "GrpHdr")?;

    write_start(&mut writer, "PmtInf")?;
    write_text_element(&mut writer, "PmtInfId", &pmt_inf_id)?;
    write_text_element(&mut writer, "PmtMtd", "TRF")?;
    write_start(&mut writer, "PmtTpInf")?;
    write_start(&mut writer, "SvcLvl")?;
    write_text_element(&mut writer, "Cd", "SEPA")?;
    write_end(&mut writer, "SvcLvl")?;
    write_end(&mut writer, "PmtTpInf")?;
    write_text_element(&mut writer, "ReqdExctnDt", &execution_date)?;
    write_start(&mut writer, "Dbtr")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "Dbtr")?;
    write_start(&mut writer, "DbtrAcct")?;
    write_start(&mut writer, "Id")?;
    write_text_element(&mut writer, "IBAN", source_iban)?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "DbtrAcct")?;
    write_start(&mut writer, "DbtrAgt")?;
    write_start(&mut writer, "FinInstnId")?;
    write_text_element(&mut writer, "BIC", source_bic)?;
    write_end(&mut writer, "FinInstnId")?;
    write_end(&mut writer, "DbtrAgt")?;
    write_text_element(&mut writer, "ChrgBr", "SLEV")?;

    write_start(&mut writer, "CdtTrfTxInf")?;
    write_start(&mut writer, "PmtId")?;
    write_text_element(&mut writer, "EndToEndId", &end_to_end_id)?;
    write_end(&mut writer, "PmtId")?;
    write_start(&mut writer, "Amt")?;
    let mut instructed_amount = BytesStart::new("InstdAmt");
    instructed_amount.push_attribute(("Ccy", currency.as_str()));
    write_xml_event(&mut writer, Event::Start(instructed_amount))?;
    write_xml_event(&mut writer, Event::Text(BytesText::new(amount)))?;
    write_end(&mut writer, "InstdAmt")?;
    write_end(&mut writer, "Amt")?;
    write_start(&mut writer, "CdtrAgt")?;
    write_start(&mut writer, "FinInstnId")?;
    write_text_element(&mut writer, "BIC", destination_bic)?;
    write_end(&mut writer, "FinInstnId")?;
    write_end(&mut writer, "CdtrAgt")?;
    write_start(&mut writer, "Cdtr")?;
    write_text_element(&mut writer, "Nm", destination_name)?;
    write_end(&mut writer, "Cdtr")?;
    write_start(&mut writer, "CdtrAcct")?;
    write_start(&mut writer, "Id")?;
    write_text_element(&mut writer, "IBAN", destination_iban)?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "CdtrAcct")?;
    if !usage.is_empty() {
        write_start(&mut writer, "RmtInf")?;
        write_text_element(&mut writer, "Ustrd", usage)?;
        write_end(&mut writer, "RmtInf")?;
    }
    write_end(&mut writer, "CdtTrfTxInf")?;
    write_end(&mut writer, "PmtInf")?;

    write_end(&mut writer, "pain.001.001.02")?;
    write_end(&mut writer, "Document")?;

    String::from_utf8(writer.into_inner()).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            "failed to encode PAIN.001.001.02 document as UTF-8",
            err,
        )
    })
}

pub fn generate_pain_001_001_02_transfers(sepa_params: &Properties) -> HbciResult<String> {
    let sepa_id = text_or_generated_message_id(sepa_params.get("sepaid").map(String::as_str));
    let pmt_inf_id = text_or_default(
        sepa_params.get("pmtinfid").map(String::as_str),
        sepa_id.as_str(),
    );
    let execution_date =
        text_or_default(sepa_params.get("date").map(String::as_str), DATE_UNDEFINED);
    let source_name = required_text(sepa_params, "src.name")?;
    let source_iban = required_text(sepa_params, "src.iban")?;
    let source_bic = required_text(sepa_params, "src.bic")?;
    let transfer_indices = sepa_transaction_indices(sepa_params);
    let total = sum_sepa_transaction_values(sepa_params)?;
    let creation_datetime = current_xml_datetime();

    let mut writer = Writer::new(Vec::new());
    write_xml_event(
        &mut writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )?;

    let mut document = BytesStart::new("Document");
    document.push_attribute(("xmlns", PAIN_001_001_02_URN));
    document.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    document.push_attribute(("xsi:schemaLocation", PAIN_001_001_02_SCHEMA_LOCATION));
    write_xml_event(&mut writer, Event::Start(document))?;
    write_start(&mut writer, "pain.001.001.02")?;

    write_start(&mut writer, "GrpHdr")?;
    write_text_element(&mut writer, "MsgId", &sepa_id)?;
    write_text_element(&mut writer, "CreDtTm", &creation_datetime)?;
    write_text_element(&mut writer, "NbOfTxs", &transfer_indices.len().to_string())?;
    write_text_element(&mut writer, "CtrlSum", &total.value)?;
    write_text_element(&mut writer, "Grpg", "GRPD")?;
    write_start(&mut writer, "InitgPty")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "InitgPty")?;
    write_end(&mut writer, "GrpHdr")?;

    write_start(&mut writer, "PmtInf")?;
    write_text_element(&mut writer, "PmtInfId", &pmt_inf_id)?;
    write_text_element(&mut writer, "PmtMtd", "TRF")?;
    write_start(&mut writer, "PmtTpInf")?;
    write_start(&mut writer, "SvcLvl")?;
    write_text_element(&mut writer, "Cd", "SEPA")?;
    write_end(&mut writer, "SvcLvl")?;
    write_end(&mut writer, "PmtTpInf")?;
    write_text_element(&mut writer, "ReqdExctnDt", &execution_date)?;
    write_start(&mut writer, "Dbtr")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "Dbtr")?;
    write_start(&mut writer, "DbtrAcct")?;
    write_start(&mut writer, "Id")?;
    write_text_element(&mut writer, "IBAN", source_iban)?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "DbtrAcct")?;
    write_start(&mut writer, "DbtrAgt")?;
    write_start(&mut writer, "FinInstnId")?;
    write_text_element(&mut writer, "BIC", source_bic)?;
    write_end(&mut writer, "FinInstnId")?;
    write_end(&mut writer, "DbtrAgt")?;
    write_text_element(&mut writer, "ChrgBr", "SLEV")?;

    for index in transfer_indices {
        write_pain_001_credit_transfer(&mut writer, sepa_params, index)?;
    }
    write_end(&mut writer, "PmtInf")?;

    write_end(&mut writer, "pain.001.001.02")?;
    write_end(&mut writer, "Document")?;

    String::from_utf8(writer.into_inner()).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            "failed to encode PAIN.001.001.02 document as UTF-8",
            err,
        )
    })
}

pub fn sum_pain_001_transfer_values(sepa_params: &Properties) -> HbciResult<Value> {
    sum_sepa_transaction_values(sepa_params)
}

pub fn sum_sepa_transaction_values(sepa_params: &Properties) -> HbciResult<Value> {
    let transfer_indices = sepa_transaction_indices(sepa_params);
    let first_index = transfer_indices.first().copied().flatten();
    let currency = text_or_default(sepa_param(sepa_params, "btg.curr", first_index), "EUR");
    let mut cents = 0_i64;

    for index in transfer_indices {
        let amount = sepa_required_param(sepa_params, "btg.value", index)?;
        cents += decimal_amount_to_cents(amount).ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("invalid SEPA amount for btg.value: {amount}"),
            )
        })?;

        let current_currency = text_or_default(sepa_param(sepa_params, "btg.curr", index), "EUR");
        if current_currency != currency {
            return Err(HbciError::new(
                HbciErrorKind::InvalidArgument,
                "mixed currencies on multiple transactions",
            ));
        }
    }

    Ok(Value {
        value: cents_to_decimal_amount(cents),
        curr: Some(currency),
    })
}

pub fn generate_pain_008_001_01_direct_debits(sepa_params: &Properties) -> HbciResult<String> {
    let sepa_id = text_or_generated_message_id(sepa_params.get("sepaid").map(String::as_str));
    let pmt_inf_id = text_or_default(
        sepa_params.get("pmtinfid").map(String::as_str),
        sepa_id.as_str(),
    );
    let collection_date = text_or_default(
        sepa_params.get("targetdate").map(String::as_str),
        DATE_UNDEFINED,
    );
    let sequence_type =
        text_or_default(sepa_params.get("sequencetype").map(String::as_str), "FRST");
    let source_name = required_text(sepa_params, "src.name")?;
    let source_iban = required_text(sepa_params, "src.iban")?;
    let source_bic = required_text(sepa_params, "src.bic")?;
    let debit_indices = sepa_transaction_indices(sepa_params);
    let total = sum_sepa_transaction_values(sepa_params)?;
    let creation_datetime = current_xml_datetime();

    let mut writer = Writer::new(Vec::new());
    write_xml_event(
        &mut writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )?;

    let mut document = BytesStart::new("Document");
    document.push_attribute(("xmlns", PAIN_008_001_01_URN));
    document.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    document.push_attribute(("xsi:schemaLocation", PAIN_008_001_01_SCHEMA_LOCATION));
    write_xml_event(&mut writer, Event::Start(document))?;
    write_start(&mut writer, "pain.008.001.01")?;

    write_start(&mut writer, "GrpHdr")?;
    write_text_element(&mut writer, "MsgId", &sepa_id)?;
    write_text_element(&mut writer, "CreDtTm", &creation_datetime)?;
    write_text_element(&mut writer, "NbOfTxs", &debit_indices.len().to_string())?;
    write_text_element(&mut writer, "CtrlSum", &total.value)?;
    write_text_element(&mut writer, "Grpg", "GRPD")?;
    write_start(&mut writer, "InitgPty")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "InitgPty")?;
    write_end(&mut writer, "GrpHdr")?;

    write_start(&mut writer, "PmtInf")?;
    write_text_element(&mut writer, "PmtInfId", &pmt_inf_id)?;
    write_text_element(&mut writer, "PmtMtd", "DD")?;
    write_start(&mut writer, "PmtTpInf")?;
    write_start(&mut writer, "SvcLvl")?;
    write_text_element(&mut writer, "Cd", "SEPA")?;
    write_end(&mut writer, "SvcLvl")?;
    write_text_element(&mut writer, "SeqTp", &sequence_type)?;
    write_end(&mut writer, "PmtTpInf")?;
    write_text_element(&mut writer, "ReqdColltnDt", &collection_date)?;
    write_start(&mut writer, "Cdtr")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "Cdtr")?;
    write_start(&mut writer, "CdtrAcct")?;
    write_start(&mut writer, "Id")?;
    write_text_element(&mut writer, "IBAN", source_iban)?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "CdtrAcct")?;
    write_start(&mut writer, "CdtrAgt")?;
    write_start(&mut writer, "FinInstnId")?;
    write_text_element(&mut writer, "BIC", source_bic)?;
    write_end(&mut writer, "FinInstnId")?;
    write_end(&mut writer, "CdtrAgt")?;
    write_text_element(&mut writer, "ChrgBr", "SLEV")?;

    for index in debit_indices {
        write_pain_008_direct_debit(&mut writer, sepa_params, index)?;
    }
    write_end(&mut writer, "PmtInf")?;

    write_end(&mut writer, "pain.008.001.01")?;
    write_end(&mut writer, "Document")?;

    String::from_utf8(writer.into_inner()).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            "failed to encode PAIN.008.001.01 document as UTF-8",
            err,
        )
    })
}

pub fn generate_pain_008_001_01_direct_debit(sepa_params: &Properties) -> HbciResult<String> {
    let sepa_id = text_or_generated_message_id(sepa_params.get("sepaid").map(String::as_str));
    let pmt_inf_id = text_or_default(
        sepa_params.get("pmtinfid").map(String::as_str),
        sepa_id.as_str(),
    );
    let collection_date = text_or_default(
        sepa_params.get("targetdate").map(String::as_str),
        DATE_UNDEFINED,
    );
    let sequence_type =
        text_or_default(sepa_params.get("sequencetype").map(String::as_str), "FRST");
    let currency = text_or_default(sepa_params.get("btg.curr").map(String::as_str), "EUR");
    let source_name = required_text(sepa_params, "src.name")?;
    let source_iban = required_text(sepa_params, "src.iban")?;
    let source_bic = required_text(sepa_params, "src.bic")?;
    let destination_name = required_text(sepa_params, "dst.name")?;
    let destination_iban = required_text(sepa_params, "dst.iban")?;
    let destination_bic = sepa_params
        .get("dst.bic")
        .map(String::as_str)
        .unwrap_or_default();
    let amount = required_text(sepa_params, "btg.value")?;
    let creditor_id = required_text(sepa_params, "creditorid")?;
    let mandate_id = required_text(sepa_params, "mandateid")?;
    let mandate_date = required_text(sepa_params, "manddateofsig")?;
    let amend_mandate_indicator = text_or_default(
        sepa_params.get("amendmandindic").map(String::as_str),
        "false",
    );
    let end_to_end_id = text_or_default(
        sepa_params.get("endtoendid").map(String::as_str),
        ENDTOEND_ID_NOTPROVIDED,
    );
    let usage = sepa_params
        .get("usage")
        .map(String::as_str)
        .unwrap_or_default();
    let creation_datetime = current_xml_datetime();

    let mut writer = Writer::new(Vec::new());
    write_xml_event(
        &mut writer,
        Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)),
    )?;

    let mut document = BytesStart::new("Document");
    document.push_attribute(("xmlns", PAIN_008_001_01_URN));
    document.push_attribute(("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    document.push_attribute(("xsi:schemaLocation", PAIN_008_001_01_SCHEMA_LOCATION));
    write_xml_event(&mut writer, Event::Start(document))?;
    write_start(&mut writer, "pain.008.001.01")?;

    write_start(&mut writer, "GrpHdr")?;
    write_text_element(&mut writer, "MsgId", &sepa_id)?;
    write_text_element(&mut writer, "CreDtTm", &creation_datetime)?;
    write_text_element(&mut writer, "NbOfTxs", "1")?;
    write_text_element(&mut writer, "CtrlSum", amount)?;
    write_text_element(&mut writer, "Grpg", "GRPD")?;
    write_start(&mut writer, "InitgPty")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "InitgPty")?;
    write_end(&mut writer, "GrpHdr")?;

    write_start(&mut writer, "PmtInf")?;
    write_text_element(&mut writer, "PmtInfId", &pmt_inf_id)?;
    write_text_element(&mut writer, "PmtMtd", "DD")?;
    write_start(&mut writer, "PmtTpInf")?;
    write_start(&mut writer, "SvcLvl")?;
    write_text_element(&mut writer, "Cd", "SEPA")?;
    write_end(&mut writer, "SvcLvl")?;
    write_text_element(&mut writer, "SeqTp", &sequence_type)?;
    write_end(&mut writer, "PmtTpInf")?;
    write_text_element(&mut writer, "ReqdColltnDt", &collection_date)?;
    write_start(&mut writer, "Cdtr")?;
    write_text_element(&mut writer, "Nm", source_name)?;
    write_end(&mut writer, "Cdtr")?;
    write_start(&mut writer, "CdtrAcct")?;
    write_start(&mut writer, "Id")?;
    write_text_element(&mut writer, "IBAN", source_iban)?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "CdtrAcct")?;
    write_start(&mut writer, "CdtrAgt")?;
    write_start(&mut writer, "FinInstnId")?;
    write_text_element(&mut writer, "BIC", source_bic)?;
    write_end(&mut writer, "FinInstnId")?;
    write_end(&mut writer, "CdtrAgt")?;
    write_text_element(&mut writer, "ChrgBr", "SLEV")?;

    write_start(&mut writer, "DrctDbtTxInf")?;
    write_start(&mut writer, "PmtId")?;
    write_text_element(&mut writer, "EndToEndId", &end_to_end_id)?;
    write_end(&mut writer, "PmtId")?;
    let mut instructed_amount = BytesStart::new("InstdAmt");
    instructed_amount.push_attribute(("Ccy", currency.as_str()));
    write_xml_event(&mut writer, Event::Start(instructed_amount))?;
    write_xml_event(&mut writer, Event::Text(BytesText::new(amount)))?;
    write_end(&mut writer, "InstdAmt")?;
    write_start(&mut writer, "DrctDbtTx")?;
    write_start(&mut writer, "MndtRltdInf")?;
    write_text_element(&mut writer, "MndtId", mandate_id)?;
    write_text_element(&mut writer, "DtOfSgntr", mandate_date)?;
    write_text_element(&mut writer, "AmdmntInd", &amend_mandate_indicator)?;
    if amend_mandate_indicator == "true" {
        write_start(&mut writer, "AmdmntInfDtls")?;
        write_start(&mut writer, "OrgnlDbtrAgt")?;
        write_start(&mut writer, "FinInstnId")?;
        write_start(&mut writer, "PrtryId")?;
        write_text_element(&mut writer, "Id", "SMNDA")?;
        write_end(&mut writer, "PrtryId")?;
        write_end(&mut writer, "FinInstnId")?;
        write_end(&mut writer, "OrgnlDbtrAgt")?;
        write_end(&mut writer, "AmdmntInfDtls")?;
    }
    write_end(&mut writer, "MndtRltdInf")?;
    write_start(&mut writer, "CdtrSchmeId")?;
    write_start(&mut writer, "Id")?;
    write_start(&mut writer, "PrvtId")?;
    write_start(&mut writer, "OthrId")?;
    write_text_element(&mut writer, "Id", creditor_id)?;
    write_text_element(&mut writer, "IdTp", "SEPA")?;
    write_end(&mut writer, "OthrId")?;
    write_end(&mut writer, "PrvtId")?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "CdtrSchmeId")?;
    write_end(&mut writer, "DrctDbtTx")?;
    write_start(&mut writer, "DbtrAgt")?;
    write_start(&mut writer, "FinInstnId")?;
    write_text_element(&mut writer, "BIC", destination_bic)?;
    write_end(&mut writer, "FinInstnId")?;
    write_end(&mut writer, "DbtrAgt")?;
    write_start(&mut writer, "Dbtr")?;
    write_text_element(&mut writer, "Nm", destination_name)?;
    write_optional_postal_address(&mut writer, sepa_params, None)?;
    write_end(&mut writer, "Dbtr")?;
    write_start(&mut writer, "DbtrAcct")?;
    write_start(&mut writer, "Id")?;
    write_text_element(&mut writer, "IBAN", destination_iban)?;
    write_end(&mut writer, "Id")?;
    write_end(&mut writer, "DbtrAcct")?;
    if !usage.is_empty() {
        write_start(&mut writer, "RmtInf")?;
        write_text_element(&mut writer, "Ustrd", usage)?;
        write_end(&mut writer, "RmtInf")?;
    }
    write_end(&mut writer, "DrctDbtTxInf")?;
    write_end(&mut writer, "PmtInf")?;

    write_end(&mut writer, "pain.008.001.01")?;
    write_end(&mut writer, "Document")?;

    String::from_utf8(writer.into_inner()).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            "failed to encode PAIN.008.001.01 document as UTF-8",
            err,
        )
    })
}

fn write_optional_postal_address(
    writer: &mut Writer<Vec<u8>>,
    sepa_params: &Properties,
    index: Option<usize>,
) -> HbciResult<()> {
    let Some(country) =
        sepa_param(sepa_params, "dst.addr.country", index).filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    write_start(writer, "PstlAdr")?;
    for name in ["dst.addr.line1", "dst.addr.line2"] {
        if let Some(line) = sepa_param(sepa_params, name, index).filter(|value| !value.is_empty()) {
            write_text_element(writer, "AdrLine", line)?;
        }
    }
    write_text_element(writer, "Ctry", country)?;
    write_end(writer, "PstlAdr")
}

fn write_pain_001_credit_transfer(
    writer: &mut Writer<Vec<u8>>,
    sepa_params: &Properties,
    index: Option<usize>,
) -> HbciResult<()> {
    let end_to_end_id = text_or_default(
        pain_001_param(sepa_params, "endtoendid", index),
        ENDTOEND_ID_NOTPROVIDED,
    );
    let currency = text_or_default(pain_001_param(sepa_params, "btg.curr", index), "EUR");
    let destination_name = pain_001_required_param(sepa_params, "dst.name", index)?;
    let destination_iban = pain_001_required_param(sepa_params, "dst.iban", index)?;
    let destination_bic = pain_001_param(sepa_params, "dst.bic", index).unwrap_or_default();
    let amount = pain_001_required_param(sepa_params, "btg.value", index)?;
    let usage = pain_001_param(sepa_params, "usage", index).unwrap_or_default();

    write_start(writer, "CdtTrfTxInf")?;
    write_start(writer, "PmtId")?;
    write_text_element(writer, "EndToEndId", &end_to_end_id)?;
    write_end(writer, "PmtId")?;
    write_start(writer, "Amt")?;
    let mut instructed_amount = BytesStart::new("InstdAmt");
    instructed_amount.push_attribute(("Ccy", currency.as_str()));
    write_xml_event(writer, Event::Start(instructed_amount))?;
    write_xml_event(writer, Event::Text(BytesText::new(amount)))?;
    write_end(writer, "InstdAmt")?;
    write_end(writer, "Amt")?;
    write_start(writer, "CdtrAgt")?;
    write_start(writer, "FinInstnId")?;
    write_text_element(writer, "BIC", destination_bic)?;
    write_end(writer, "FinInstnId")?;
    write_end(writer, "CdtrAgt")?;
    write_start(writer, "Cdtr")?;
    write_text_element(writer, "Nm", destination_name)?;
    write_end(writer, "Cdtr")?;
    write_start(writer, "CdtrAcct")?;
    write_start(writer, "Id")?;
    write_text_element(writer, "IBAN", destination_iban)?;
    write_end(writer, "Id")?;
    write_end(writer, "CdtrAcct")?;
    if !usage.is_empty() {
        write_start(writer, "RmtInf")?;
        write_text_element(writer, "Ustrd", usage)?;
        write_end(writer, "RmtInf")?;
    }
    write_end(writer, "CdtTrfTxInf")
}

fn write_pain_008_direct_debit(
    writer: &mut Writer<Vec<u8>>,
    sepa_params: &Properties,
    index: Option<usize>,
) -> HbciResult<()> {
    let end_to_end_id = text_or_default(
        sepa_param(sepa_params, "endtoendid", index),
        ENDTOEND_ID_NOTPROVIDED,
    );
    let currency = text_or_default(sepa_param(sepa_params, "btg.curr", index), "EUR");
    let destination_name = sepa_required_param(sepa_params, "dst.name", index)?;
    let destination_iban = sepa_required_param(sepa_params, "dst.iban", index)?;
    let destination_bic = sepa_param(sepa_params, "dst.bic", index).unwrap_or_default();
    let amount = sepa_required_param(sepa_params, "btg.value", index)?;
    let creditor_id = sepa_required_param(sepa_params, "creditorid", index)?;
    let mandate_id = sepa_required_param(sepa_params, "mandateid", index)?;
    let mandate_date = sepa_required_param(sepa_params, "manddateofsig", index)?;
    let amend_mandate_indicator =
        text_or_default(sepa_param(sepa_params, "amendmandindic", index), "false");
    let usage = sepa_param(sepa_params, "usage", index).unwrap_or_default();

    write_start(writer, "DrctDbtTxInf")?;
    write_start(writer, "PmtId")?;
    write_text_element(writer, "EndToEndId", &end_to_end_id)?;
    write_end(writer, "PmtId")?;
    let mut instructed_amount = BytesStart::new("InstdAmt");
    instructed_amount.push_attribute(("Ccy", currency.as_str()));
    write_xml_event(writer, Event::Start(instructed_amount))?;
    write_xml_event(writer, Event::Text(BytesText::new(amount)))?;
    write_end(writer, "InstdAmt")?;
    write_start(writer, "DrctDbtTx")?;
    write_start(writer, "MndtRltdInf")?;
    write_text_element(writer, "MndtId", mandate_id)?;
    write_text_element(writer, "DtOfSgntr", mandate_date)?;
    write_text_element(writer, "AmdmntInd", &amend_mandate_indicator)?;
    if amend_mandate_indicator == "true" {
        write_start(writer, "AmdmntInfDtls")?;
        write_start(writer, "OrgnlDbtrAgt")?;
        write_start(writer, "FinInstnId")?;
        write_start(writer, "PrtryId")?;
        write_text_element(writer, "Id", "SMNDA")?;
        write_end(writer, "PrtryId")?;
        write_end(writer, "FinInstnId")?;
        write_end(writer, "OrgnlDbtrAgt")?;
        write_end(writer, "AmdmntInfDtls")?;
    }
    write_end(writer, "MndtRltdInf")?;
    write_start(writer, "CdtrSchmeId")?;
    write_start(writer, "Id")?;
    write_start(writer, "PrvtId")?;
    write_start(writer, "OthrId")?;
    write_text_element(writer, "Id", creditor_id)?;
    write_text_element(writer, "IdTp", "SEPA")?;
    write_end(writer, "OthrId")?;
    write_end(writer, "PrvtId")?;
    write_end(writer, "Id")?;
    write_end(writer, "CdtrSchmeId")?;
    write_end(writer, "DrctDbtTx")?;
    write_start(writer, "DbtrAgt")?;
    write_start(writer, "FinInstnId")?;
    write_text_element(writer, "BIC", destination_bic)?;
    write_end(writer, "FinInstnId")?;
    write_end(writer, "DbtrAgt")?;
    write_start(writer, "Dbtr")?;
    write_text_element(writer, "Nm", destination_name)?;
    write_optional_postal_address(writer, sepa_params, index)?;
    write_end(writer, "Dbtr")?;
    write_start(writer, "DbtrAcct")?;
    write_start(writer, "Id")?;
    write_text_element(writer, "IBAN", destination_iban)?;
    write_end(writer, "Id")?;
    write_end(writer, "DbtrAcct")?;
    if !usage.is_empty() {
        write_start(writer, "RmtInf")?;
        write_text_element(writer, "Ustrd", usage)?;
        write_end(writer, "RmtInf")?;
    }
    write_end(writer, "DrctDbtTxInf")
}

fn sepa_transaction_indices(sepa_params: &Properties) -> Vec<Option<usize>> {
    let Some(max_index) = sepa_params
        .keys()
        .filter_map(|key| sepa_param_index(key))
        .max()
    else {
        return vec![None];
    };

    (0..=max_index).map(Some).collect()
}

fn pain_001_required_param<'a>(
    sepa_params: &'a Properties,
    name: &str,
    index: Option<usize>,
) -> HbciResult<&'a str> {
    sepa_required_param(sepa_params, name, index)
}

fn sepa_required_param<'a>(
    sepa_params: &'a Properties,
    name: &str,
    index: Option<usize>,
) -> HbciResult<&'a str> {
    sepa_param(sepa_params, name, index)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!(
                    "missing required SEPA parameter: {}",
                    indexed_sepa_param_name(name, index)
                ),
            )
        })
}

fn pain_001_param<'a>(
    sepa_params: &'a Properties,
    name: &str,
    index: Option<usize>,
) -> Option<&'a str> {
    sepa_param(sepa_params, name, index)
}

fn sepa_param<'a>(
    sepa_params: &'a Properties,
    name: &str,
    index: Option<usize>,
) -> Option<&'a str> {
    sepa_params
        .get(&indexed_sepa_param_name(name, index))
        .map(String::as_str)
}

fn indexed_sepa_param_name(name: &str, index: Option<usize>) -> String {
    let Some(index) = index else {
        return name.to_owned();
    };

    if let Some((head, tail)) = name.split_once('.') {
        format!("{head}[{index}].{tail}")
    } else {
        format!("{name}[{index}]")
    }
}

fn sepa_param_index(name: &str) -> Option<usize> {
    let start = name.find('[')?;
    let end = name[start + 1..].find(']')? + start + 1;
    name[start + 1..end].parse().ok()
}

fn required_text<'a>(properties: &'a Properties, name: &str) -> HbciResult<&'a str> {
    properties
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            HbciError::new(
                HbciErrorKind::InvalidArgument,
                format!("missing required SEPA parameter: {name}"),
            )
        })
}

fn text_or_default(value: Option<&str>, default_value: &str) -> String {
    value
        .filter(|value| !value.is_empty())
        .unwrap_or(default_value)
        .to_owned()
}

fn text_or_generated_message_id(value: Option<&str>) -> String {
    value
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(generated_sepa_message_id)
}

fn generated_sepa_message_id() -> String {
    format!("{}:0000", current_xml_datetime())
}

fn current_xml_datetime() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    unix_seconds_to_utc_datetime(seconds)
}

fn unix_seconds_to_utc_datetime(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    let (year, month, day) = utc_civil_from_unix_days(days);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}")
}

fn utc_civil_from_unix_days(days_since_unix_epoch: i64) -> (i32, u32, u32) {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    (year as i32, month as u32, day as u32)
}

fn write_start(writer: &mut Writer<Vec<u8>>, name: &str) -> HbciResult<()> {
    write_xml_event(writer, Event::Start(BytesStart::new(name)))
}

fn write_end(writer: &mut Writer<Vec<u8>>, name: &str) -> HbciResult<()> {
    write_xml_event(writer, Event::End(BytesEnd::new(name)))
}

fn write_text_element(writer: &mut Writer<Vec<u8>>, name: &str, value: &str) -> HbciResult<()> {
    write_start(writer, name)?;
    write_xml_event(writer, Event::Text(BytesText::new(value)))?;
    write_end(writer, name)
}

fn write_xml_event(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> HbciResult<()> {
    writer.write_event(event).map_err(|err| {
        HbciError::with_source(
            HbciErrorKind::Protocol,
            "failed to write SEPA XML document",
            err,
        )
    })
}

#[derive(Debug, Clone, Default)]
struct Pain001PaymentInfo {
    source: Konto,
    payment_info_id: Option<String>,
    execution_date: Option<String>,
    batch_book: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct Pain001TransferDraft {
    destination: Konto,
    value: Option<Value>,
    usage: Vec<String>,
    end_to_end_id: Option<String>,
    purpose_code: Option<String>,
}

fn parse_pain_001_transfer_shell(xml: &str) -> HbciResult<Vec<Pain001Transfer>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack = Vec::new();
    let mut transfers = Vec::new();
    let mut group_initiator_name = None::<String>;
    let mut payment = None::<Pain001PaymentInfo>;
    let mut transfer = None::<Pain001TransferDraft>;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "PmtInf" {
                    payment = Some(Pain001PaymentInfo::default());
                } else if payment.is_some() && name == "CdtTrfTxInf" {
                    transfer = Some(Pain001TransferDraft::default());
                } else if transfer.is_some()
                    && name == "InstdAmt"
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(transfer) = &mut transfer
                {
                    transfer.value = Some(Value {
                        value: String::new(),
                        curr: Some(currency),
                    });
                }
                stack.push(name);
            }
            Ok(Event::Empty(event)) => {
                let name = local_name(event.name().as_ref());
                if transfer.is_some()
                    && name == "InstdAmt"
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(transfer) = &mut transfer
                {
                    transfer.value = Some(Value {
                        value: String::new(),
                        curr: Some(currency),
                    });
                }
            }
            Ok(Event::Text(event)) => {
                let text = event.decode().map_err(|err| {
                    HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to decode XML text",
                        err,
                    )
                })?;
                let text = text.trim();
                if !text.is_empty() {
                    collect_pain_001_text(
                        &stack,
                        text,
                        &mut group_initiator_name,
                        &mut payment,
                        &mut transfer,
                    );
                }
            }
            Ok(Event::End(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "CdtTrfTxInf" {
                    if let (Some(payment), Some(transfer)) = (&payment, transfer.take()) {
                        transfers.push(pain_001_transfer_from_parts(
                            payment,
                            transfer,
                            group_initiator_name.as_deref(),
                        ));
                    }
                } else if name == "PmtInf" {
                    payment = None;
                }

                stack.pop();
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(HbciError::with_source(
                    HbciErrorKind::Protocol,
                    "failed to parse PAIN.001 document",
                    err,
                ));
            }
        }
    }

    Ok(transfers)
}

fn collect_pain_001_text(
    stack: &[String],
    text: &str,
    group_initiator_name: &mut Option<String>,
    payment: &mut Option<Pain001PaymentInfo>,
    transfer: &mut Option<Pain001TransferDraft>,
) {
    if let Some(transfer) = transfer {
        collect_pain_001_transfer_text(stack, text, transfer);
    } else if let Some(payment) = payment {
        collect_pain_001_payment_text(stack, text, payment);
    } else if path_ends_with(stack, &["GrpHdr", "InitgPty", "Nm"]) {
        *group_initiator_name = Some(text.to_owned());
    }
}

fn collect_pain_001_payment_text(stack: &[String], text: &str, payment: &mut Pain001PaymentInfo) {
    if path_ends_with(stack, &["PmtInf", "PmtInfId"]) {
        payment.payment_info_id = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "DbtrAcct", "Id", "IBAN"]) {
        payment.source.iban = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "DbtrAgt", "FinInstnId", "BIC"])
        || path_ends_with(stack, &["PmtInf", "DbtrAgt", "FinInstnId", "BICFI"])
    {
        payment.source.bic = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "ReqdExctnDt"])
        || path_ends_with(stack, &["PmtInf", "ReqdExctnDt", "Dt"])
    {
        payment.execution_date = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "ReqdExctnDt", "DtTm"]) {
        payment.execution_date = Some(text.chars().take(10).collect());
    } else if path_ends_with(stack, &["PmtInf", "BtchBookg"]) {
        payment.batch_book = Some(text.to_owned());
    }
}

fn collect_pain_001_transfer_text(
    stack: &[String],
    text: &str,
    transfer: &mut Pain001TransferDraft,
) {
    if path_ends_with(stack, &["CdtTrfTxInf", "PmtId", "EndToEndId"]) {
        transfer.end_to_end_id = Some(text.to_owned());
    } else if path_ends_with(stack, &["CdtTrfTxInf", "Cdtr", "Nm"]) {
        transfer.destination.name = Some(text.to_owned());
    } else if path_ends_with(stack, &["CdtTrfTxInf", "CdtrAcct", "Id", "IBAN"]) {
        transfer.destination.iban = Some(text.to_owned());
    } else if path_ends_with(stack, &["CdtTrfTxInf", "CdtrAgt", "FinInstnId", "BIC"])
        || path_ends_with(stack, &["CdtTrfTxInf", "CdtrAgt", "FinInstnId", "BICFI"])
    {
        transfer.destination.bic = Some(text.to_owned());
    } else if path_ends_with(stack, &["CdtTrfTxInf", "Amt", "InstdAmt"]) {
        if let Some(value) = &mut transfer.value {
            value.value = normalize_decimal_amount(text);
        } else {
            transfer.value = Some(Value {
                value: normalize_decimal_amount(text),
                curr: None,
            });
        }
    } else if path_ends_with(stack, &["CdtTrfTxInf", "RmtInf", "Ustrd"]) {
        transfer.usage.push(text.to_owned());
    } else if path_ends_with(stack, &["CdtTrfTxInf", "Purp", "Cd"]) {
        transfer.purpose_code = Some(text.to_owned());
    }
}

fn pain_001_transfer_from_parts(
    payment: &Pain001PaymentInfo,
    transfer: Pain001TransferDraft,
    group_initiator_name: Option<&str>,
) -> Pain001Transfer {
    let mut source = payment.source.clone();
    source.name = group_initiator_name.map(str::to_owned);

    Pain001Transfer {
        source,
        destination: transfer.destination,
        value: transfer.value,
        usage: transfer.usage,
        execution_date: payment.execution_date.clone(),
        end_to_end_id: transfer.end_to_end_id,
        payment_info_id: payment.payment_info_id.clone(),
        purpose_code: transfer.purpose_code,
        batch_book: payment.batch_book.clone(),
    }
}

#[derive(Debug, Clone, Default)]
struct Pain008PaymentInfo {
    creditor: Konto,
    payment_info_id: Option<String>,
    collection_date: Option<String>,
    batch_book: Option<String>,
    debit_type: Option<String>,
    sequence_type: Option<String>,
    creditor_id: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct Pain008DirectDebitDraft {
    debtor: Konto,
    value: Option<Value>,
    usage: Vec<String>,
    end_to_end_id: Option<String>,
    purpose_code: Option<String>,
    mandate_id: Option<String>,
    mandate_date_of_signature: Option<String>,
}

fn parse_pain_008_direct_debit_shell(xml: &str) -> HbciResult<Vec<Pain008DirectDebit>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack = Vec::new();
    let mut direct_debits = Vec::new();
    let mut group_initiator_name = None::<String>;
    let mut payment = None::<Pain008PaymentInfo>;
    let mut direct_debit = None::<Pain008DirectDebitDraft>;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "PmtInf" {
                    payment = Some(Pain008PaymentInfo::default());
                } else if payment.is_some() && name == "DrctDbtTxInf" {
                    direct_debit = Some(Pain008DirectDebitDraft::default());
                } else if direct_debit.is_some()
                    && name == "InstdAmt"
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(direct_debit) = &mut direct_debit
                {
                    direct_debit.value = Some(Value {
                        value: String::new(),
                        curr: Some(currency),
                    });
                }
                stack.push(name);
            }
            Ok(Event::Empty(event)) => {
                let name = local_name(event.name().as_ref());
                if direct_debit.is_some()
                    && name == "InstdAmt"
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(direct_debit) = &mut direct_debit
                {
                    direct_debit.value = Some(Value {
                        value: String::new(),
                        curr: Some(currency),
                    });
                }
            }
            Ok(Event::Text(event)) => {
                let text = event.decode().map_err(|err| {
                    HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to decode XML text",
                        err,
                    )
                })?;
                let text = text.trim();
                if !text.is_empty() {
                    collect_pain_008_text(
                        &stack,
                        text,
                        &mut group_initiator_name,
                        &mut payment,
                        &mut direct_debit,
                    );
                }
            }
            Ok(Event::End(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "DrctDbtTxInf" {
                    if let (Some(payment), Some(direct_debit)) = (&payment, direct_debit.take()) {
                        direct_debits.push(pain_008_direct_debit_from_parts(
                            payment,
                            direct_debit,
                            group_initiator_name.as_deref(),
                        ));
                    }
                } else if name == "PmtInf" {
                    payment = None;
                }

                stack.pop();
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(HbciError::with_source(
                    HbciErrorKind::Protocol,
                    "failed to parse PAIN.008 document",
                    err,
                ));
            }
        }
    }

    Ok(direct_debits)
}

fn collect_pain_008_text(
    stack: &[String],
    text: &str,
    group_initiator_name: &mut Option<String>,
    payment: &mut Option<Pain008PaymentInfo>,
    direct_debit: &mut Option<Pain008DirectDebitDraft>,
) {
    if let Some(direct_debit) = direct_debit {
        collect_pain_008_direct_debit_text(stack, text, direct_debit);
    } else if let Some(payment) = payment {
        collect_pain_008_payment_text(stack, text, payment);
    } else if path_ends_with(stack, &["GrpHdr", "InitgPty", "Nm"]) {
        *group_initiator_name = Some(text.to_owned());
    }
}

fn collect_pain_008_payment_text(stack: &[String], text: &str, payment: &mut Pain008PaymentInfo) {
    if path_ends_with(stack, &["PmtInf", "PmtInfId"]) {
        payment.payment_info_id = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "Cdtr", "Nm"]) {
        payment.creditor.name = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "CdtrAcct", "Id", "IBAN"]) {
        payment.creditor.iban = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "CdtrAgt", "FinInstnId", "BIC"])
        || path_ends_with(stack, &["PmtInf", "CdtrAgt", "FinInstnId", "BICFI"])
    {
        payment.creditor.bic = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "ReqdColltnDt"])
        || path_ends_with(stack, &["PmtInf", "ReqdColltnDt", "Dt"])
    {
        payment.collection_date = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "ReqdColltnDt", "DtTm"]) {
        payment.collection_date = Some(text.chars().take(10).collect());
    } else if path_ends_with(stack, &["PmtInf", "BtchBookg"]) {
        payment.batch_book = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "PmtTpInf", "LclInstrm", "Cd"]) {
        payment.debit_type = Some(text.to_owned());
    } else if path_ends_with(stack, &["PmtInf", "PmtTpInf", "SeqTp"]) {
        payment.sequence_type = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["PmtInf", "CdtrSchmeId", "Id", "PrvtId", "Othr", "Id"],
    ) || path_ends_with(
        stack,
        &["PmtInf", "CdtrSchmeId", "Id", "PrvtId", "OthrId", "Id"],
    ) {
        payment.creditor_id = Some(text.to_owned());
    }
}

fn collect_pain_008_direct_debit_text(
    stack: &[String],
    text: &str,
    direct_debit: &mut Pain008DirectDebitDraft,
) {
    if path_ends_with(stack, &["DrctDbtTxInf", "PmtId", "EndToEndId"]) {
        direct_debit.end_to_end_id = Some(text.to_owned());
    } else if path_ends_with(stack, &["DrctDbtTxInf", "Dbtr", "Nm"]) {
        direct_debit.debtor.name = Some(text.to_owned());
    } else if path_ends_with(stack, &["DrctDbtTxInf", "DbtrAcct", "Id", "IBAN"]) {
        direct_debit.debtor.iban = Some(text.to_owned());
    } else if path_ends_with(stack, &["DrctDbtTxInf", "DbtrAgt", "FinInstnId", "BIC"])
        || path_ends_with(stack, &["DrctDbtTxInf", "DbtrAgt", "FinInstnId", "BICFI"])
    {
        direct_debit.debtor.bic = Some(text.to_owned());
    } else if path_ends_with(stack, &["DrctDbtTxInf", "InstdAmt"]) {
        if let Some(value) = &mut direct_debit.value {
            value.value = normalize_decimal_amount(text);
        } else {
            direct_debit.value = Some(Value {
                value: normalize_decimal_amount(text),
                curr: None,
            });
        }
    } else if path_ends_with(stack, &["DrctDbtTxInf", "RmtInf", "Ustrd"]) {
        direct_debit.usage.push(text.to_owned());
    } else if path_ends_with(stack, &["DrctDbtTxInf", "Purp", "Cd"]) {
        direct_debit.purpose_code = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["DrctDbtTxInf", "DrctDbtTx", "MndtRltdInf", "MndtId"],
    ) {
        direct_debit.mandate_id = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["DrctDbtTxInf", "DrctDbtTx", "MndtRltdInf", "DtOfSgntr"],
    ) {
        direct_debit.mandate_date_of_signature = Some(text.to_owned());
    }
}

fn pain_008_direct_debit_from_parts(
    payment: &Pain008PaymentInfo,
    direct_debit: Pain008DirectDebitDraft,
    group_initiator_name: Option<&str>,
) -> Pain008DirectDebit {
    let mut creditor = payment.creditor.clone();
    if creditor.name.is_none() {
        creditor.name = group_initiator_name.map(str::to_owned);
    }

    Pain008DirectDebit {
        creditor,
        debtor: direct_debit.debtor,
        value: direct_debit.value,
        usage: direct_debit.usage,
        collection_date: payment.collection_date.clone(),
        end_to_end_id: direct_debit.end_to_end_id,
        payment_info_id: payment.payment_info_id.clone(),
        purpose_code: direct_debit.purpose_code,
        debit_type: payment.debit_type.clone(),
        sequence_type: payment.sequence_type.clone(),
        creditor_id: payment.creditor_id.clone(),
        mandate_id: direct_debit.mandate_id,
        mandate_date_of_signature: direct_debit.mandate_date_of_signature,
        batch_book: payment.batch_book.clone(),
    }
}

#[derive(Debug, Clone, Default)]
struct CamtReport {
    account: Konto,
    balances: Vec<CamtBalance>,
    entries: Vec<CamtEntry>,
}

#[derive(Debug, Clone, Default)]
struct CamtBalance {
    code: Option<String>,
    amount: Option<String>,
    currency: Option<String>,
    credit_debit: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct CamtEntry {
    amount: Option<String>,
    currency: Option<String>,
    credit_debit: Option<String>,
    reversal: bool,
    booking_date: Option<String>,
    value_date: Option<String>,
    text: Option<String>,
    customer_ref: Option<String>,
    has_details: bool,
    seen_detail: bool,
    collecting_first_detail: bool,
    seen_tx_details: bool,
    collecting_first_tx: bool,
    tx: CamtTxDetails,
}

#[derive(Debug, Clone, Default)]
struct CamtTxDetails {
    id: Option<String>,
    acct_svcr_ref: Option<String>,
    end_to_end_id: Option<String>,
    mandate_id: Option<String>,
    debtor: Konto,
    creditor: Konto,
    usages: Vec<String>,
    purpose_code: Option<String>,
    return_reason_code: Option<String>,
    return_additional: Vec<String>,
    instructed_amount: Option<String>,
    instructed_amount_currency: Option<String>,
    proprietary_bank_code: Option<String>,
}

impl From<CamtReport> for GvrKUmsBTag {
    fn from(report: CamtReport) -> Self {
        let mut tag = GvrKUmsBTag {
            my: report.account,
            start_type: 'F',
            end_type: 'F',
            ..GvrKUmsBTag::default()
        };

        if let Some(start) = report.balances.first().and_then(camt_start_balance) {
            tag.start = Some(start);
        }
        if let Some(end) = report.balances.get(1).and_then(camt_end_balance) {
            tag.end = Some(end);
        }

        let mut saldo = tag
            .start
            .as_ref()
            .and_then(|saldo| decimal_amount_to_cents(&saldo.value.value))
            .unwrap_or(0);
        let saldo_curr = tag
            .start
            .as_ref()
            .and_then(|saldo| saldo.value.curr.clone())
            .or_else(|| tag.my.curr.clone());

        for entry in report.entries {
            if let Some(line) = camt_line_from_entry(entry, saldo, saldo_curr.as_deref()) {
                saldo = line
                    .saldo
                    .as_ref()
                    .and_then(|saldo| decimal_amount_to_cents(&saldo.value.value))
                    .unwrap_or(saldo);
                tag.lines.push(line);
            }
        }
        camt_correct_line_balances_from_end(&mut tag);

        tag
    }
}

fn camt_correct_line_balances_from_end(tag: &mut GvrKUmsBTag) {
    let missing_start_timestamp = tag
        .start
        .as_ref()
        .and_then(|saldo| saldo.date.as_ref())
        .is_none();
    let Some(mut end_saldo) = tag
        .end
        .as_ref()
        .filter(|saldo| saldo.date.is_some())
        .and_then(|saldo| decimal_amount_to_cents(&saldo.value.value))
    else {
        return;
    };

    if !missing_start_timestamp {
        return;
    }

    for line in tag.lines.iter_mut().rev() {
        if let Some(saldo) = &mut line.saldo {
            saldo.value.value = cents_to_decimal_amount(end_saldo);
        }

        let line_value = line
            .value
            .as_ref()
            .and_then(|value| decimal_amount_to_cents(&value.value))
            .unwrap_or(0);
        end_saldo -= line_value;
    }
}

fn camt_start_balance(balance: &CamtBalance) -> Option<Saldo> {
    let code = balance.code.as_deref()?;
    if !matches!(code, "PRCD" | "ITBD" | "OPBD") {
        return None;
    }

    let date = if code == "PRCD" {
        balance.date.as_deref().and_then(add_one_iso_day)
    } else {
        balance.date.clone()
    };

    Some(camt_saldo_from_balance(balance, date))
}

fn camt_end_balance(balance: &CamtBalance) -> Option<Saldo> {
    let code = balance.code.as_deref()?;
    if !matches!(code, "CLBD" | "ITBD") {
        return None;
    }

    Some(camt_saldo_from_balance(balance, balance.date.clone()))
}

fn camt_saldo_from_balance(balance: &CamtBalance, date: Option<String>) -> Saldo {
    Saldo {
        value: Value {
            value: camt_signed_amount(
                balance.amount.as_deref().unwrap_or("0"),
                balance.credit_debit.as_deref(),
            ),
            curr: balance.currency.clone(),
        },
        date,
        time: None,
    }
}

fn parse_camt_reports(xml: &str) -> HbciResult<Vec<CamtReport>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut stack = Vec::new();
    let mut reports = Vec::new();
    let mut report = None::<CamtReport>;
    let mut balance = None::<CamtBalance>;
    let mut entry = None::<CamtEntry>;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "Rpt" {
                    report = Some(CamtReport::default());
                } else if report.is_some() && name == "Bal" {
                    balance = Some(CamtBalance::default());
                } else if report.is_some() && name == "Ntry" {
                    entry = Some(CamtEntry::default());
                } else if entry.is_some() && name == "NtryDtls" {
                    if let Some(entry) = &mut entry {
                        entry.has_details = true;
                        if !entry.seen_detail {
                            entry.seen_detail = true;
                            entry.collecting_first_detail = true;
                        } else {
                            entry.collecting_first_detail = false;
                        }
                    }
                } else if name == "TxDtls"
                    && entry
                        .as_ref()
                        .is_some_and(|entry| entry.collecting_first_detail)
                {
                    if let Some(entry) = &mut entry {
                        if !entry.seen_tx_details {
                            entry.seen_tx_details = true;
                            entry.collecting_first_tx = true;
                        } else {
                            entry.collecting_first_tx = false;
                        }
                    }
                } else if balance.is_some()
                    && name == "Amt"
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(balance) = &mut balance
                {
                    balance.currency = Some(currency);
                } else if entry.is_some()
                    && name == "Amt"
                    && path_ends_with(&stack, &["Rpt", "Ntry"])
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(entry) = &mut entry
                {
                    entry.currency = Some(currency);
                } else if entry
                    .as_ref()
                    .is_some_and(|entry| entry.collecting_first_tx)
                    && name == "Amt"
                    && path_ends_with(&stack, &["NtryDtls", "TxDtls", "AmtDtls", "InstdAmt"])
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(entry) = &mut entry
                {
                    entry.tx.instructed_amount_currency = Some(currency);
                }
                stack.push(name);
            }
            Ok(Event::Empty(event)) => {
                let name = local_name(event.name().as_ref());
                if entry.is_some() && name == "NtryDtls" {
                    if let Some(entry) = &mut entry {
                        entry.has_details = true;
                        if !entry.seen_detail {
                            entry.seen_detail = true;
                        }
                        entry.collecting_first_detail = false;
                    }
                } else if name == "TxDtls"
                    && entry
                        .as_ref()
                        .is_some_and(|entry| entry.collecting_first_detail)
                {
                    if let Some(entry) = &mut entry {
                        if !entry.seen_tx_details {
                            entry.seen_tx_details = true;
                        }
                        entry.collecting_first_tx = false;
                    }
                } else if balance.is_some()
                    && name == "Amt"
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(balance) = &mut balance
                {
                    balance.currency = Some(currency);
                } else if entry.is_some()
                    && name == "Amt"
                    && path_ends_with(&stack, &["Rpt", "Ntry"])
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(entry) = &mut entry
                {
                    entry.currency = Some(currency);
                } else if entry
                    .as_ref()
                    .is_some_and(|entry| entry.collecting_first_tx)
                    && name == "Amt"
                    && path_ends_with(&stack, &["NtryDtls", "TxDtls", "AmtDtls", "InstdAmt"])
                    && let Some(currency) = attr_value(&reader, &event, b"Ccy")?
                    && let Some(entry) = &mut entry
                {
                    entry.tx.instructed_amount_currency = Some(currency);
                }
            }
            Ok(Event::Text(event)) => {
                let text = event.decode().map_err(|err| {
                    HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to decode XML text",
                        err,
                    )
                })?;
                let text = text.trim();
                if !text.is_empty() {
                    collect_camt_text(&stack, text, &mut report, &mut balance, &mut entry);
                }
            }
            Ok(Event::End(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "TxDtls" {
                    if let Some(entry) = &mut entry {
                        entry.collecting_first_tx = false;
                    }
                } else if name == "NtryDtls" {
                    if let Some(entry) = &mut entry {
                        entry.collecting_first_detail = false;
                    }
                } else if name == "Bal" {
                    if let (Some(report), Some(balance)) = (&mut report, balance.take()) {
                        report.balances.push(balance);
                    }
                } else if name == "Ntry" {
                    if let (Some(report), Some(entry)) = (&mut report, entry.take()) {
                        report.entries.push(entry);
                    }
                } else if name == "Rpt"
                    && let Some(report) = report.take()
                {
                    reports.push(report);
                }
                stack.pop();
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(HbciError::with_source(
                    HbciErrorKind::Protocol,
                    "failed to parse CAMT document",
                    err,
                ));
            }
        }
    }

    Ok(reports)
}

fn collect_camt_text(
    stack: &[String],
    text: &str,
    report: &mut Option<CamtReport>,
    balance: &mut Option<CamtBalance>,
    entry: &mut Option<CamtEntry>,
) {
    let Some(report) = report else {
        return;
    };

    if path_ends_with(stack, &["Rpt", "Acct", "Id", "IBAN"]) {
        report.account.iban = Some(text.to_owned());
    } else if path_ends_with(stack, &["Rpt", "Acct", "Ccy"]) {
        report.account.curr = Some(text.to_owned());
    } else if path_ends_with(stack, &["Rpt", "Acct", "Svcr", "FinInstnId", "BIC"])
        || path_ends_with(stack, &["Rpt", "Acct", "Svcr", "FinInstnId", "BICFI"])
    {
        report.account.bic = Some(text.to_owned());
    }

    if let Some(entry) = entry {
        if path_ends_with(stack, &["Rpt", "Ntry", "Amt"]) {
            entry.amount = Some(text.to_owned());
        } else if path_ends_with(stack, &["Rpt", "Ntry", "CdtDbtInd"]) {
            entry.credit_debit = Some(text.to_owned());
        } else if path_ends_with(stack, &["Rpt", "Ntry", "RvslInd"]) {
            entry.reversal = text.eq_ignore_ascii_case("true");
        } else if path_ends_with(stack, &["Rpt", "Ntry", "BookgDt", "Dt"])
            || path_ends_with(stack, &["Rpt", "Ntry", "BookgDt", "DtTm"])
        {
            entry.booking_date = Some(text.chars().take(10).collect());
        } else if path_ends_with(stack, &["Rpt", "Ntry", "ValDt", "Dt"])
            || path_ends_with(stack, &["Rpt", "Ntry", "ValDt", "DtTm"])
        {
            entry.value_date = Some(text.chars().take(10).collect());
        } else if path_ends_with(stack, &["Rpt", "Ntry", "AddtlNtryInf"]) {
            entry.text = Some(text.to_owned());
        } else if path_ends_with(stack, &["Rpt", "Ntry", "AcctSvcrRef"]) {
            entry.customer_ref = Some(text.to_owned());
        }

        if entry.collecting_first_tx {
            collect_camt_tx_text(stack, text, &mut entry.tx);
        }
    }

    let Some(balance) = balance else {
        return;
    };

    if path_ends_with(stack, &["Bal", "Tp", "CdOrPrtry", "Cd"]) {
        balance.code = Some(text.to_owned());
    } else if path_ends_with(stack, &["Bal", "Amt"]) {
        balance.amount = Some(text.to_owned());
    } else if path_ends_with(stack, &["Bal", "CdtDbtInd"]) {
        balance.credit_debit = Some(text.to_owned());
    } else if path_ends_with(stack, &["Bal", "Dt", "Dt"])
        || path_ends_with(stack, &["Bal", "Dt", "DtTm"])
    {
        balance.date = Some(text.chars().take(10).collect());
    }
}

fn collect_camt_tx_text(stack: &[String], text: &str, tx: &mut CamtTxDetails) {
    if path_ends_with(stack, &["NtryDtls", "TxDtls", "Refs", "Prtry", "Ref"]) {
        tx.id = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "Refs", "AcctSvcrRef"]) {
        tx.acct_svcr_ref = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "Refs", "EndToEndId"]) {
        tx.end_to_end_id = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "Refs", "MndtId"]) {
        tx.mandate_id = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["NtryDtls", "TxDtls", "RltdPties", "DbtrAcct", "Id", "IBAN"],
    ) {
        tx.debtor.iban = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["NtryDtls", "TxDtls", "RltdPties", "CdtrAcct", "Id", "IBAN"],
    ) {
        tx.creditor.iban = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "RltdPties", "Dbtr", "Nm"])
        || path_ends_with(
            stack,
            &["NtryDtls", "TxDtls", "RltdPties", "Dbtr", "Pty", "Nm"],
        )
    {
        tx.debtor.name = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "RltdPties", "Cdtr", "Nm"])
        || path_ends_with(
            stack,
            &["NtryDtls", "TxDtls", "RltdPties", "Cdtr", "Pty", "Nm"],
        )
    {
        tx.creditor.name = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["NtryDtls", "TxDtls", "RltdPties", "UltmtDbtr", "Nm"],
    ) {
        tx.debtor.name2 = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &["NtryDtls", "TxDtls", "RltdPties", "UltmtCdtr", "Nm"],
    ) {
        tx.creditor.name2 = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Dbtr",
            "Id",
            "PrvtId",
            "Othr",
            "Id",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Dbtr",
            "Id",
            "PrvtId",
            "OthrId",
            "Id",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Dbtr",
            "Pty",
            "Id",
            "PrvtId",
            "Othr",
            "Id",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Dbtr",
            "Pty",
            "Id",
            "PrvtId",
            "OthrId",
            "Id",
        ],
    ) {
        tx.debtor.creditorid = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Cdtr",
            "Id",
            "PrvtId",
            "Othr",
            "Id",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Cdtr",
            "Id",
            "PrvtId",
            "OthrId",
            "Id",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Cdtr",
            "Pty",
            "Id",
            "PrvtId",
            "Othr",
            "Id",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdPties",
            "Cdtr",
            "Pty",
            "Id",
            "PrvtId",
            "OthrId",
            "Id",
        ],
    ) {
        tx.creditor.creditorid = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdAgts",
            "DbtrAgt",
            "FinInstnId",
            "BIC",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdAgts",
            "DbtrAgt",
            "FinInstnId",
            "BICFI",
        ],
    ) {
        tx.debtor.bic = Some(text.to_owned());
    } else if path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdAgts",
            "CdtrAgt",
            "FinInstnId",
            "BIC",
        ],
    ) || path_ends_with(
        stack,
        &[
            "NtryDtls",
            "TxDtls",
            "RltdAgts",
            "CdtrAgt",
            "FinInstnId",
            "BICFI",
        ],
    ) {
        tx.creditor.bic = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "RmtInf", "Ustrd"]) {
        tx.usages.push(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "Purp", "Cd"]) {
        tx.purpose_code = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "RtrInf", "Rsn", "Cd"]) {
        tx.return_reason_code = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "RtrInf", "AddtlInf"]) {
        tx.return_additional.push(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "AmtDtls", "InstdAmt", "Amt"]) {
        tx.instructed_amount = Some(text.to_owned());
    } else if path_ends_with(stack, &["NtryDtls", "TxDtls", "BkTxCd", "Prtry", "Cd"]) {
        tx.proprietary_bank_code = Some(text.to_owned());
    }
}

fn camt_line_from_entry(
    entry: CamtEntry,
    current_saldo: i64,
    saldo_curr: Option<&str>,
) -> Option<GvrKUmsLine> {
    if entry.has_details && !entry.seen_tx_details {
        return None;
    }

    let value = Value {
        value: camt_signed_amount(
            entry.amount.as_deref().unwrap_or("0"),
            entry.credit_debit.as_deref(),
        ),
        curr: entry
            .currency
            .clone()
            .or_else(|| saldo_curr.map(ToOwned::to_owned)),
    };
    let value_cents = decimal_amount_to_cents(&value.value).unwrap_or(0);
    let next_saldo = current_saldo + value_cents;

    let mut bdate = entry.booking_date;
    let mut valuta = entry.value_date;
    if bdate.is_none() {
        bdate.clone_from(&valuta);
    }
    if valuta.is_none() {
        valuta.clone_from(&bdate);
    }

    let mut line = GvrKUmsLine {
        valuta,
        bdate: bdate.clone(),
        value: Some(value.clone()),
        is_storno: entry.reversal,
        saldo: Some(Saldo {
            value: Value {
                value: cents_to_decimal_amount(next_saldo),
                curr: value.curr.clone(),
            },
            date: bdate,
            time: None,
        }),
        customerref: entry.customer_ref.clone(),
        text: entry.text.clone(),
        other: Some(Konto::default()),
        is_sepa: true,
        is_camt: true,
        ..GvrKUmsLine::default()
    };

    if !entry.has_details
        && let Some(text) = entry.text
    {
        line.add_usage(Some(text));
    } else if entry.has_details {
        let tx = entry.tx;
        let is_return = tx
            .return_reason_code
            .as_deref()
            .is_some_and(|code| !code.is_empty());
        let mut use_debtor = entry.credit_debit.as_deref() == Some("CRDT");
        if is_return {
            use_debtor = !use_debtor;
        }
        let other = if use_debtor {
            tx.debtor.clone()
        } else {
            tx.creditor.clone()
        };

        line.id = tx
            .id
            .or_else(|| entry.customer_ref.clone())
            .or(tx.acct_svcr_ref);
        line.end_to_end_id = tx.end_to_end_id;
        line.mandate_id = tx.mandate_id;
        line.other = Some(other);
        line.usage.extend(tx.usages);
        camt_apply_proprietary_bank_code(&mut line, tx.proprietary_bank_code.as_deref());
        line.purposecode = tx.purpose_code;

        if is_return {
            if let Some(instructed_amount) = tx.instructed_amount {
                line.orig_value = Some(Value {
                    value: normalize_decimal_amount(&instructed_amount),
                    curr: tx.instructed_amount_currency,
                });
            }

            if !tx.return_additional.is_empty() {
                line.additional = Some(tx.return_additional.join(","));
            }
        }
    }

    Some(line)
}

fn camt_apply_proprietary_bank_code(line: &mut GvrKUmsLine, code: Option<&str>) {
    let Some(code) = code.filter(|code| code.contains('+')) else {
        return;
    };

    let parts = java_like_plus_split(code);
    if parts.len() == 4 {
        line.gvcode = Some(parts[1].to_owned());
        line.primanota = Some(parts[2].to_owned());
        line.addkey = Some(parts[3].to_owned());
    }
}

fn java_like_plus_split(code: &str) -> Vec<&str> {
    let mut parts = code.split('+').collect::<Vec<_>>();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    parts
}

fn attr_value(
    reader: &quick_xml::Reader<&[u8]>,
    event: &quick_xml::events::BytesStart<'_>,
    key: &[u8],
) -> HbciResult<Option<String>> {
    for attr in event.attributes() {
        let attr = attr.map_err(|err| {
            HbciError::with_source(HbciErrorKind::Protocol, "failed to read XML attribute", err)
        })?;
        if attr.key.as_ref() == key {
            let value = attr
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
                .map_err(|err| {
                    HbciError::with_source(
                        HbciErrorKind::Protocol,
                        "failed to decode XML attribute",
                        err,
                    )
                })?;
            return Ok(Some(value.into_owned()));
        }
    }

    Ok(None)
}

fn path_ends_with(stack: &[String], suffix: &[&str]) -> bool {
    stack.len() >= suffix.len()
        && stack[stack.len() - suffix.len()..]
            .iter()
            .map(String::as_str)
            .eq(suffix.iter().copied())
}

fn local_name(name: &[u8]) -> String {
    let name = String::from_utf8_lossy(name);
    name.rsplit(':').next().unwrap_or(&name).to_owned()
}

fn camt_signed_amount(amount: &str, credit_debit: Option<&str>) -> String {
    let mut amount = normalize_decimal_amount(amount);
    if credit_debit == Some("DBIT") && amount != "0.00" && !amount.starts_with('-') {
        amount.insert(0, '-');
    }
    amount
}

fn normalize_decimal_amount(amount: &str) -> String {
    decimal_amount_to_cents(amount)
        .map(cents_to_decimal_amount)
        .unwrap_or_else(|| amount.trim().replace(',', "."))
}

fn decimal_amount_to_cents(amount: &str) -> Option<i64> {
    let amount = amount.trim().replace(',', ".");
    let negative = amount.starts_with('-');
    let amount = amount.trim_start_matches(['+', '-']);
    let (integer, fraction) = amount.split_once('.').unwrap_or((amount, ""));
    let integer = if integer.is_empty() {
        0
    } else {
        integer.parse::<i64>().ok()?
    };
    let mut fraction_digits = fraction.chars().filter(|ch| ch.is_ascii_digit());
    let tens = fraction_digits
        .next()
        .and_then(|ch| ch.to_digit(10))
        .unwrap_or(0) as i64;
    let ones = fraction_digits
        .next()
        .and_then(|ch| ch.to_digit(10))
        .unwrap_or(0) as i64;
    let round = fraction_digits
        .next()
        .and_then(|ch| ch.to_digit(10))
        .is_some_and(|digit| digit >= 5) as i64;
    let cents = integer * 100 + tens * 10 + ones + round;

    Some(if negative { -cents } else { cents })
}

fn cents_to_decimal_amount(cents: i64) -> String {
    let negative = cents < 0;
    let cents = cents.abs();
    let prefix = if negative { "-" } else { "" };
    format!("{prefix}{}.{:02}", cents / 100, cents % 100)
}

fn add_one_iso_day(date: &str) -> Option<String> {
    let (year, month, day) = parse_iso_date(date)?;
    civil_from_days(days_from_civil(year, month, day) + 1)
}

fn parse_iso_date(date: &str) -> Option<(i32, u32, u32)> {
    let mut parts = date.split('-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let day = parts.next()?.parse().ok()?;
    Some((year, month, day))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i32 {
    let year = year - (month <= 2) as i32;
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month = month as i32;
    let day = day as i32;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(days: i32) -> Option<String> {
    let days = days + 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = days - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = year + (month <= 2) as i32;

    Some(format!("{year:04}-{month:02}-{day:02}"))
}

fn root_namespace(xml: &str) -> HbciResult<Option<String>> {
    let mut reader = quick_xml::Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                for attr in event.attributes() {
                    let attr = attr.map_err(|err| {
                        HbciError::with_source(
                            HbciErrorKind::Protocol,
                            "failed to read XML root attribute",
                            err,
                        )
                    })?;
                    if attr.key.as_ref() == b"xmlns" {
                        let value = attr
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .map_err(|err| {
                                HbciError::with_source(
                                    HbciErrorKind::Protocol,
                                    "failed to decode XML root namespace",
                                    err,
                                )
                            })?;
                        return Ok(Some(value.into_owned()));
                    }
                }
                return Ok(None);
            }
            Ok(Event::Decl(_)) | Ok(Event::Comment(_)) | Ok(Event::Text(_)) => {}
            Ok(Event::Eof) => return Ok(None),
            Ok(_) => {}
            Err(err) => {
                return Err(HbciError::with_source(
                    HbciErrorKind::Protocol,
                    "failed to parse XML document",
                    err,
                ));
            }
        }
    }
}
