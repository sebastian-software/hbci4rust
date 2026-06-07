use hbci4rust::sepa::{
    CAMT_052_001_01_URN, CAMT_052_001_02_URN, CAMT_052_001_04_URN, CAMT_052_001_07_URN,
    CAMT_052_001_08_URN, ENDTOEND_ID_NOTPROVIDED, PAIN_001_001_02_URN, PAIN_008_001_01_URN,
    PAIN_008_001_02_URN, SepaKind, SepaVersion, generate_pain_001_001_02_transfer,
    generate_pain_001_001_02_transfers, generate_pain_008_001_01_direct_debit,
    generate_pain_008_001_01_direct_debits, parse_camt_report_shell, parse_pain_001_transfers,
    parse_pain_008_direct_debits, sum_pain_001_transfer_values, sum_sepa_transaction_values,
};
use hbci4rust::{HbciErrorKind, Properties};

fn camt_document(urn: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="{urn}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <BkToCstmrAcctRpt/>
</Document>"#
    )
}

fn camt_report_shell_document() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="{CAMT_052_001_01_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct>
        <Id><IBAN>DE12345678901234567890</IBAN></Id>
        <Ccy>EUR</Ccy>
        <Svcr><FinInstnId><BIC>ABCDEFG1ABC</BIC></FinInstnId></Svcr>
      </Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>PRCD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">100</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-19</Dt></Dt>
      </Bal>
      <Bal>
        <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">110.50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    )
}

fn pain_001_001_02_document() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="{PAIN_001_001_02_URN}">
  <pain.001.001.02>
    <GrpHdr>
      <InitgPty><Nm>Sender Name</Nm></InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-OLD</PmtInfId>
      <ReqdExctnDt>2026-01-02</ReqdExctnDt>
      <DbtrAcct><Id><IBAN>DE11111111111111111111</IBAN></Id></DbtrAcct>
      <DbtrAgt><FinInstnId><BIC>SRCBICOLD</BIC></FinInstnId></DbtrAgt>
      <CdtTrfTxInf>
        <PmtId><EndToEndId>E2E-OLD</EndToEndId></PmtId>
        <Amt><InstdAmt Ccy="EUR">12.3</InstdAmt></Amt>
        <CdtrAgt><FinInstnId><BIC>DSTBICOLD</BIC></FinInstnId></CdtrAgt>
        <Cdtr><Nm>Receiver Old</Nm></Cdtr>
        <CdtrAcct><Id><IBAN>DE22222222222222222222</IBAN></Id></CdtrAcct>
        <RmtInf><Ustrd>Old Usage</Ustrd></RmtInf>
      </CdtTrfTxInf>
    </PmtInf>
  </pain.001.001.02>
</Document>"#
    )
}

fn pain_001_001_09_document() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pain.001.001.09">
  <CstmrCdtTrfInitn>
    <GrpHdr>
      <InitgPty><Nm>Modern Sender</Nm></InitgPty>
    </GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-NEW</PmtInfId>
      <BtchBookg>true</BtchBookg>
      <ReqdExctnDt><Dt>2026-03-04</Dt></ReqdExctnDt>
      <DbtrAcct><Id><IBAN>DE33333333333333333333</IBAN></Id></DbtrAcct>
      <DbtrAgt><FinInstnId><BICFI>SRCBICNEW</BICFI></FinInstnId></DbtrAgt>
      <CdtTrfTxInf>
        <PmtId><EndToEndId>E2E-NEW</EndToEndId></PmtId>
        <Amt><InstdAmt Ccy="EUR">1.005</InstdAmt></Amt>
        <CdtrAgt><FinInstnId><BICFI>DSTBICNEW</BICFI></FinInstnId></CdtrAgt>
        <Cdtr><Nm>Receiver New</Nm></Cdtr>
        <CdtrAcct><Id><IBAN>DE44444444444444444444</IBAN></Id></CdtrAcct>
        <Purp><Cd>GDDS</Cd></Purp>
        <RmtInf>
          <Ustrd>Line 1</Ustrd>
          <Ustrd>Line 2</Ustrd>
        </RmtInf>
      </CdtTrfTxInf>
    </PmtInf>
  </CstmrCdtTrfInitn>
</Document>"#
}

#[test]
fn camt_version_by_urn_matches_original_known_versions() {
    let version = SepaVersion::by_urn(CAMT_052_001_04_URN).expect("known CAMT version");

    assert_eq!(version, SepaVersion::CAMT_052_001_04);
    assert_eq!(version.kind(), SepaKind::Camt052);
    assert_eq!(version.major(), 1);
    assert_eq!(version.minor(), 4);
    assert_eq!(version.urn(), CAMT_052_001_04_URN);
    assert_eq!(version.schema_file(), Some("camt.052.001.04.xsd"));
    assert_eq!(
        version.schema_location().as_deref(),
        Some("urn:iso:std:iso:20022:tech:xsd:camt.052.001.04 camt.052.001.04.xsd")
    );
}

#[test]
fn camt_version_find_greatest_uses_original_order() {
    let highest = SepaVersion::find_greatest(&[
        SepaVersion::by_urn("urn:iso:std:iso:20022:tech:xsd:camt.052.001.02")
            .expect("known version"),
        SepaVersion::by_urn("urn:iso:std:iso:20022:tech:xsd:camt.052.001.05")
            .expect("known version"),
        SepaVersion::by_urn(CAMT_052_001_07_URN).expect("known version"),
    ]);

    assert_eq!(highest, Some(SepaVersion::CAMT_052_001_07));
}

#[test]
fn pain_001_parser_reads_old_transfer_fields_like_original() {
    let transfers =
        parse_pain_001_transfers(&pain_001_001_02_document()).expect("PAIN.001.001.02 parses");

    assert_eq!(transfers.len(), 1);
    let transfer = &transfers[0];
    assert_eq!(transfer.payment_info_id.as_deref(), Some("PMT-OLD"));
    assert_eq!(transfer.source.name.as_deref(), Some("Sender Name"));
    assert_eq!(
        transfer.source.iban.as_deref(),
        Some("DE11111111111111111111")
    );
    assert_eq!(transfer.source.bic.as_deref(), Some("SRCBICOLD"));
    assert_eq!(transfer.destination.name.as_deref(), Some("Receiver Old"));
    assert_eq!(
        transfer.destination.iban.as_deref(),
        Some("DE22222222222222222222")
    );
    assert_eq!(transfer.destination.bic.as_deref(), Some("DSTBICOLD"));
    assert_eq!(
        transfer.value.as_ref().map(|value| value.value.as_str()),
        Some("12.30")
    );
    assert_eq!(
        transfer
            .value
            .as_ref()
            .and_then(|value| value.curr.as_deref()),
        Some("EUR")
    );
    assert_eq!(transfer.usage, ["Old Usage".to_owned()]);
    assert_eq!(transfer.execution_date.as_deref(), Some("2026-01-02"));
    assert_eq!(transfer.end_to_end_id.as_deref(), Some("E2E-OLD"));
    assert_eq!(transfer.purpose_code, None);
}

#[test]
fn pain_001_parser_reads_new_transfer_fields_like_original() {
    let transfers =
        parse_pain_001_transfers(pain_001_001_09_document()).expect("PAIN.001.001.09 parses");

    assert_eq!(transfers.len(), 1);
    let transfer = &transfers[0];
    assert_eq!(transfer.payment_info_id.as_deref(), Some("PMT-NEW"));
    assert_eq!(transfer.source.name.as_deref(), Some("Modern Sender"));
    assert_eq!(
        transfer.source.iban.as_deref(),
        Some("DE33333333333333333333")
    );
    assert_eq!(transfer.source.bic.as_deref(), Some("SRCBICNEW"));
    assert_eq!(transfer.destination.name.as_deref(), Some("Receiver New"));
    assert_eq!(
        transfer.destination.iban.as_deref(),
        Some("DE44444444444444444444")
    );
    assert_eq!(transfer.destination.bic.as_deref(), Some("DSTBICNEW"));
    assert_eq!(
        transfer.value.as_ref().map(|value| value.value.as_str()),
        Some("1.01")
    );
    assert_eq!(transfer.usage, ["Line 1".to_owned(), "Line 2".to_owned()]);
    assert_eq!(transfer.execution_date.as_deref(), Some("2026-03-04"));
    assert_eq!(transfer.end_to_end_id.as_deref(), Some("E2E-NEW"));
    assert_eq!(transfer.purpose_code.as_deref(), Some("GDDS"));
    assert_eq!(transfer.batch_book.as_deref(), Some("true"));
}

#[test]
fn pain_001_generator_writes_single_transfer_defaults_like_original() {
    let params = Properties::from([
        ("sepaid".to_owned(), "SEPA-GEN".to_owned()),
        ("src.name".to_owned(), "Sender Name".to_owned()),
        ("src.iban".to_owned(), "DE11111111111111111111".to_owned()),
        ("src.bic".to_owned(), "SRCBICOLD".to_owned()),
        ("dst.name".to_owned(), "Receiver Old".to_owned()),
        ("dst.iban".to_owned(), "DE22222222222222222222".to_owned()),
        ("btg.value".to_owned(), "12.30".to_owned()),
        ("usage".to_owned(), "Invoice reference".to_owned()),
    ]);

    let xml = generate_pain_001_001_02_transfer(&params).expect("PAIN.001 XML generates");

    assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(xml.contains(&format!(r#"xmlns="{PAIN_001_001_02_URN}""#)));
    assert!(xml.contains("<MsgId>SEPA-GEN</MsgId>"));
    assert!(xml.contains("<EndToEndId>NOTPROVIDED</EndToEndId>"));

    let transfers = parse_pain_001_transfers(&xml).expect("generated PAIN.001 parses");
    assert_eq!(transfers.len(), 1);
    let transfer = &transfers[0];
    assert_eq!(transfer.payment_info_id.as_deref(), Some("SEPA-GEN"));
    assert_eq!(transfer.source.name.as_deref(), Some("Sender Name"));
    assert_eq!(
        transfer.source.iban.as_deref(),
        Some("DE11111111111111111111")
    );
    assert_eq!(transfer.source.bic.as_deref(), Some("SRCBICOLD"));
    assert_eq!(transfer.destination.name.as_deref(), Some("Receiver Old"));
    assert_eq!(
        transfer.destination.iban.as_deref(),
        Some("DE22222222222222222222")
    );
    assert_eq!(
        transfer.value.as_ref().map(|value| value.value.as_str()),
        Some("12.30")
    );
    assert_eq!(
        transfer
            .value
            .as_ref()
            .and_then(|value| value.curr.as_deref()),
        Some("EUR")
    );
    assert_eq!(transfer.usage, ["Invoice reference".to_owned()]);
    assert_eq!(transfer.execution_date.as_deref(), Some("1999-01-01"));
    assert_eq!(
        transfer.end_to_end_id.as_deref(),
        Some(ENDTOEND_ID_NOTPROVIDED)
    );
}

#[test]
fn pain_001_generator_writes_indexed_multi_transfers_like_original() {
    let params = Properties::from([
        ("sepaid".to_owned(), "SEPA-MULTI".to_owned()),
        ("pmtinfid".to_owned(), "PMT-MULTI".to_owned()),
        ("src.name".to_owned(), "Sender Name".to_owned()),
        ("src.iban".to_owned(), "DE02123456780000000000".to_owned()),
        ("src.bic".to_owned(), "MARKDEF1100".to_owned()),
        ("dst[0].name".to_owned(), "Receiver One".to_owned()),
        (
            "dst[0].iban".to_owned(),
            "DE99123456780000000000".to_owned(),
        ),
        ("dst[0].bic".to_owned(), "DEUTDEDB277".to_owned()),
        ("btg[0].value".to_owned(), "12.30".to_owned()),
        ("btg[0].curr".to_owned(), "EUR".to_owned()),
        ("usage[0]".to_owned(), "Usage one".to_owned()),
        ("endtoendid[0]".to_owned(), "E2E-1".to_owned()),
        ("dst[1].name".to_owned(), "Receiver Two".to_owned()),
        (
            "dst[1].iban".to_owned(),
            "DE77123456780000000000".to_owned(),
        ),
        ("dst[1].bic".to_owned(), "COBADEFFXXX".to_owned()),
        ("btg[1].value".to_owned(), "20.00".to_owned()),
        ("btg[1].curr".to_owned(), "EUR".to_owned()),
        ("usage[1]".to_owned(), "Usage two".to_owned()),
        ("endtoendid[1]".to_owned(), "E2E-2".to_owned()),
    ]);

    let total = sum_pain_001_transfer_values(&params).expect("total is calculated");
    let xml = generate_pain_001_001_02_transfers(&params).expect("multi PAIN.001 generates");
    let transfers = parse_pain_001_transfers(&xml).expect("generated PAIN.001 parses");

    assert_eq!(total.value, "32.30");
    assert_eq!(total.curr.as_deref(), Some("EUR"));
    assert!(xml.contains("<NbOfTxs>2</NbOfTxs>"));
    assert!(xml.contains("<CtrlSum>32.30</CtrlSum>"));
    assert!(xml.contains("<PmtInfId>PMT-MULTI</PmtInfId>"));
    assert!(xml.contains("<Cdtr><Nm>Receiver One</Nm></Cdtr>"));
    assert!(xml.contains("<Cdtr><Nm>Receiver Two</Nm></Cdtr>"));
    assert_eq!(transfers.len(), 2);
    assert_eq!(
        transfers[0].destination.name.as_deref(),
        Some("Receiver One")
    );
    assert_eq!(
        transfers[1].destination.name.as_deref(),
        Some("Receiver Two")
    );
    assert_eq!(
        transfers[1]
            .value
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("20.00")
    );
}

#[test]
fn pain_001_multi_sum_rejects_mixed_currencies_like_original() {
    let params = Properties::from([
        ("src.name".to_owned(), "Sender Name".to_owned()),
        ("src.iban".to_owned(), "DE02123456780000000000".to_owned()),
        ("src.bic".to_owned(), "MARKDEF1100".to_owned()),
        ("dst[0].name".to_owned(), "Receiver One".to_owned()),
        (
            "dst[0].iban".to_owned(),
            "DE99123456780000000000".to_owned(),
        ),
        ("btg[0].value".to_owned(), "12.30".to_owned()),
        ("btg[0].curr".to_owned(), "EUR".to_owned()),
        ("dst[1].name".to_owned(), "Receiver Two".to_owned()),
        (
            "dst[1].iban".to_owned(),
            "DE77123456780000000000".to_owned(),
        ),
        ("btg[1].value".to_owned(), "20.00".to_owned()),
        ("btg[1].curr".to_owned(), "USD".to_owned()),
    ]);

    let err = sum_pain_001_transfer_values(&params).expect_err("mixed currencies are rejected");

    assert_eq!(err.kind(), HbciErrorKind::InvalidArgument);
    assert!(err.to_string().contains("mixed currencies"));
}

#[test]
fn pain_008_generator_writes_single_direct_debit_defaults_like_original() {
    let params = Properties::from([
        ("sepaid".to_owned(), "SEPA-LAST".to_owned()),
        ("src.name".to_owned(), "Creditor Name".to_owned()),
        ("src.iban".to_owned(), "DE02123456780000000000".to_owned()),
        ("src.bic".to_owned(), "MARKDEF1100".to_owned()),
        ("dst.name".to_owned(), "Debtor Name".to_owned()),
        ("dst.iban".to_owned(), "DE99123456780000000000".to_owned()),
        ("dst.bic".to_owned(), "DEUTDEDB277".to_owned()),
        ("btg.value".to_owned(), "12.30".to_owned()),
        ("usage".to_owned(), "Direct debit usage".to_owned()),
        ("creditorid".to_owned(), "DE98ZZZ09999999999".to_owned()),
        ("mandateid".to_owned(), "MND-123".to_owned()),
        ("manddateofsig".to_owned(), "2026-01-02".to_owned()),
    ]);

    let xml = generate_pain_008_001_01_direct_debit(&params).expect("PAIN.008 XML generates");

    assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(xml.contains(&format!(r#"xmlns="{PAIN_008_001_01_URN}""#)));
    assert!(xml.contains("<pain.008.001.01>"));
    assert!(xml.contains("<MsgId>SEPA-LAST</MsgId>"));
    assert!(xml.contains("<NbOfTxs>1</NbOfTxs>"));
    assert!(xml.contains("<CtrlSum>12.30</CtrlSum>"));
    assert!(xml.contains("<PmtInfId>SEPA-LAST</PmtInfId>"));
    assert!(xml.contains("<PmtMtd>DD</PmtMtd>"));
    assert!(xml.contains("<ReqdColltnDt>1999-01-01</ReqdColltnDt>"));
    assert!(xml.contains("<SeqTp>FRST</SeqTp>"));
    assert!(xml.contains("<Cdtr><Nm>Creditor Name</Nm></Cdtr>"));
    assert!(xml.contains("<CdtrAcct><Id><IBAN>DE02123456780000000000</IBAN></Id></CdtrAcct>"));
    assert!(xml.contains("<Id>DE98ZZZ09999999999</Id>"));
    assert!(xml.contains("<MndtId>MND-123</MndtId>"));
    assert!(xml.contains("<DtOfSgntr>2026-01-02</DtOfSgntr>"));
    assert!(xml.contains("<AmdmntInd>false</AmdmntInd>"));
    assert!(xml.contains("<EndToEndId>NOTPROVIDED</EndToEndId>"));
    assert!(xml.contains(r#"<InstdAmt Ccy="EUR">12.30</InstdAmt>"#));
    assert!(xml.contains("<Dbtr><Nm>Debtor Name</Nm></Dbtr>"));
    assert!(xml.contains("<Ustrd>Direct debit usage</Ustrd>"));
}

#[test]
fn pain_008_generator_writes_indexed_multi_direct_debits_like_original() {
    let params = Properties::from([
        ("sepaid".to_owned(), "SEPA-MULTI-LAST".to_owned()),
        ("pmtinfid".to_owned(), "PMT-MULTI-LAST".to_owned()),
        ("targetdate".to_owned(), "2026-03-15".to_owned()),
        ("src.name".to_owned(), "Creditor Name".to_owned()),
        ("src.iban".to_owned(), "DE02123456780000000000".to_owned()),
        ("src.bic".to_owned(), "MARKDEF1100".to_owned()),
        ("dst[0].name".to_owned(), "Debtor One".to_owned()),
        (
            "dst[0].iban".to_owned(),
            "DE99123456780000000000".to_owned(),
        ),
        ("dst[0].bic".to_owned(), "DEUTDEDB277".to_owned()),
        ("btg[0].value".to_owned(), "12.30".to_owned()),
        ("btg[0].curr".to_owned(), "EUR".to_owned()),
        ("usage[0]".to_owned(), "Debit usage one".to_owned()),
        ("endtoendid[0]".to_owned(), "E2E-LAST-1".to_owned()),
        ("creditorid[0]".to_owned(), "DE98ZZZ09999999999".to_owned()),
        ("mandateid[0]".to_owned(), "MND-1".to_owned()),
        ("manddateofsig[0]".to_owned(), "2026-01-02".to_owned()),
        ("dst[1].name".to_owned(), "Debtor Two".to_owned()),
        (
            "dst[1].iban".to_owned(),
            "DE77123456780000000000".to_owned(),
        ),
        ("dst[1].bic".to_owned(), "COBADEFFXXX".to_owned()),
        ("btg[1].value".to_owned(), "20.00".to_owned()),
        ("btg[1].curr".to_owned(), "EUR".to_owned()),
        ("usage[1]".to_owned(), "Debit usage two".to_owned()),
        ("endtoendid[1]".to_owned(), "E2E-LAST-2".to_owned()),
        ("creditorid[1]".to_owned(), "DE98ZZZ09999999999".to_owned()),
        ("mandateid[1]".to_owned(), "MND-2".to_owned()),
        ("manddateofsig[1]".to_owned(), "2026-01-03".to_owned()),
    ]);

    let total = sum_sepa_transaction_values(&params).expect("total is calculated");
    let xml = generate_pain_008_001_01_direct_debits(&params).expect("multi PAIN.008 generates");
    let debits = parse_pain_008_direct_debits(&xml).expect("generated PAIN.008 parses");

    assert_eq!(total.value, "32.30");
    assert_eq!(total.curr.as_deref(), Some("EUR"));
    assert!(xml.contains("<NbOfTxs>2</NbOfTxs>"));
    assert!(xml.contains("<CtrlSum>32.30</CtrlSum>"));
    assert!(xml.contains("<PmtInfId>PMT-MULTI-LAST</PmtInfId>"));
    assert!(xml.contains("<ReqdColltnDt>2026-03-15</ReqdColltnDt>"));
    assert!(xml.contains("<EndToEndId>E2E-LAST-1</EndToEndId>"));
    assert!(xml.contains("<EndToEndId>E2E-LAST-2</EndToEndId>"));
    assert!(xml.contains("<MndtId>MND-1</MndtId>"));
    assert!(xml.contains("<MndtId>MND-2</MndtId>"));
    assert!(xml.contains("<Dbtr><Nm>Debtor One</Nm></Dbtr>"));
    assert!(xml.contains("<Dbtr><Nm>Debtor Two</Nm></Dbtr>"));
    assert_eq!(debits.len(), 2);
    assert_eq!(debits[0].debtor.name.as_deref(), Some("Debtor One"));
    assert_eq!(debits[1].debtor.name.as_deref(), Some("Debtor Two"));
    assert_eq!(
        debits[1].value.as_ref().map(|value| value.value.as_str()),
        Some("20.00")
    );
    assert_eq!(debits[1].mandate_id.as_deref(), Some("MND-2"));
}

#[test]
fn pain_008_multi_sum_rejects_mixed_currencies_like_original() {
    let params = Properties::from([
        ("src.name".to_owned(), "Creditor Name".to_owned()),
        ("src.iban".to_owned(), "DE02123456780000000000".to_owned()),
        ("src.bic".to_owned(), "MARKDEF1100".to_owned()),
        ("dst[0].name".to_owned(), "Debtor One".to_owned()),
        (
            "dst[0].iban".to_owned(),
            "DE99123456780000000000".to_owned(),
        ),
        ("btg[0].value".to_owned(), "12.30".to_owned()),
        ("btg[0].curr".to_owned(), "EUR".to_owned()),
        ("creditorid[0]".to_owned(), "DE98ZZZ09999999999".to_owned()),
        ("mandateid[0]".to_owned(), "MND-1".to_owned()),
        ("manddateofsig[0]".to_owned(), "2026-01-02".to_owned()),
        ("dst[1].name".to_owned(), "Debtor Two".to_owned()),
        (
            "dst[1].iban".to_owned(),
            "DE77123456780000000000".to_owned(),
        ),
        ("btg[1].value".to_owned(), "20.00".to_owned()),
        ("btg[1].curr".to_owned(), "USD".to_owned()),
        ("creditorid[1]".to_owned(), "DE98ZZZ09999999999".to_owned()),
        ("mandateid[1]".to_owned(), "MND-2".to_owned()),
        ("manddateofsig[1]".to_owned(), "2026-01-03".to_owned()),
    ]);

    let err = sum_sepa_transaction_values(&params).expect_err("mixed currencies are rejected");

    assert_eq!(err.kind(), HbciErrorKind::InvalidArgument);
    assert!(err.to_string().contains("mixed currencies"));
}

#[test]
fn pain_008_parser_reads_direct_debit_fields_like_original() {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="{PAIN_008_001_02_URN}">
  <CstmrDrctDbtInitn>
    <GrpHdr><InitgPty><Nm>Creditor Group Name</Nm></InitgPty></GrpHdr>
    <PmtInf>
      <PmtInfId>PMT-DLS</PmtInfId>
      <PmtTpInf>
        <SvcLvl><Cd>SEPA</Cd></SvcLvl>
        <LclInstrm><Cd>CORE</Cd></LclInstrm>
        <SeqTp>FRST</SeqTp>
      </PmtTpInf>
      <ReqdColltnDt>2026-01-02</ReqdColltnDt>
      <Cdtr><Nm>Creditor Name</Nm></Cdtr>
      <CdtrAcct><Id><IBAN>DE02123456780000000000</IBAN></Id></CdtrAcct>
      <CdtrAgt><FinInstnId><BIC>MARKDEF1100</BIC></FinInstnId></CdtrAgt>
      <CdtrSchmeId><Id><PrvtId><Othr><Id>DE98ZZZ09999999999</Id></Othr></PrvtId></Id></CdtrSchmeId>
      <DrctDbtTxInf>
        <PmtId><EndToEndId>E2E-DLS</EndToEndId></PmtId>
        <InstdAmt Ccy="EUR">19.95</InstdAmt>
        <DrctDbtTx><MndtRltdInf><MndtId>MND-DLS</MndtId><DtOfSgntr>2026-01-04</DtOfSgntr></MndtRltdInf></DrctDbtTx>
        <DbtrAgt><FinInstnId><BIC>DEUTDEDB277</BIC></FinInstnId></DbtrAgt>
        <Dbtr><Nm>Debtor Name</Nm></Dbtr>
        <DbtrAcct><Id><IBAN>DE99123456780000000000</IBAN></Id></DbtrAcct>
        <Purp><Cd>GDDS</Cd></Purp>
        <RmtInf><Ustrd>Recurring debit usage</Ustrd></RmtInf>
      </DrctDbtTxInf>
    </PmtInf>
  </CstmrDrctDbtInitn>
</Document>"#
    );

    let debits = parse_pain_008_direct_debits(&xml).expect("PAIN.008.001.02 parses");

    assert_eq!(debits.len(), 1);
    let debit = &debits[0];
    assert_eq!(debit.creditor.name.as_deref(), Some("Creditor Name"));
    assert_eq!(
        debit.creditor.iban.as_deref(),
        Some("DE02123456780000000000")
    );
    assert_eq!(debit.creditor.bic.as_deref(), Some("MARKDEF1100"));
    assert_eq!(debit.debtor.name.as_deref(), Some("Debtor Name"));
    assert_eq!(debit.debtor.iban.as_deref(), Some("DE99123456780000000000"));
    assert_eq!(debit.debtor.bic.as_deref(), Some("DEUTDEDB277"));
    assert_eq!(
        debit.value.as_ref().map(|value| value.value.as_str()),
        Some("19.95")
    );
    assert_eq!(
        debit.value.as_ref().and_then(|value| value.curr.as_deref()),
        Some("EUR")
    );
    assert_eq!(debit.usage, ["Recurring debit usage".to_owned()]);
    assert_eq!(debit.collection_date.as_deref(), Some("2026-01-02"));
    assert_eq!(debit.end_to_end_id.as_deref(), Some("E2E-DLS"));
    assert_eq!(debit.payment_info_id.as_deref(), Some("PMT-DLS"));
    assert_eq!(debit.purpose_code.as_deref(), Some("GDDS"));
    assert_eq!(debit.debit_type.as_deref(), Some("CORE"));
    assert_eq!(debit.sequence_type.as_deref(), Some("FRST"));
    assert_eq!(debit.creditor_id.as_deref(), Some("DE98ZZZ09999999999"));
    assert_eq!(debit.mandate_id.as_deref(), Some("MND-DLS"));
    assert_eq!(
        debit.mandate_date_of_signature.as_deref(),
        Some("2026-01-04")
    );
}

#[test]
fn camt_version_autodetects_root_namespace() {
    let version = SepaVersion::autodetect(&camt_document(CAMT_052_001_08_URN)).expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_08));
}

#[test]
fn camt_version_autodetects_no_namespace_as_none_like_original_test002() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-parse-none.xml");

    let version = SepaVersion::autodetect(xml).expect("valid upstream fixture");

    assert_eq!(version, None);
}

#[test]
fn camt_version_autodetect_rejects_invalid_namespace_like_original_test003() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-parse-invalid.xml");

    let err = SepaVersion::autodetect(xml).expect_err("invalid namespace is rejected");

    assert_eq!(err.kind(), HbciErrorKind::InvalidArgument);
    assert!(err.message().contains("invalid sepa-version"));
}

#[test]
fn camt_version_choose_prefers_xml_data_over_descriptor_like_original() {
    let version = SepaVersion::choose(
        Some(CAMT_052_001_01_URN),
        Some(&camt_document(CAMT_052_001_08_URN)),
    )
    .expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_08));
}

#[test]
fn camt_version_choose_falls_back_to_descriptor_when_xml_has_no_namespace() {
    let version = SepaVersion::choose(
        Some(CAMT_052_001_01_URN),
        Some("<Document><BkToCstmrAcctRpt/></Document>"),
    )
    .expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_01));
}

#[test]
fn camt_report_shell_parses_account_and_balances_like_original() {
    let days = parse_camt_report_shell(&camt_report_shell_document(), SepaVersion::CAMT_052_001_01)
        .expect("CAMT shell parses");

    assert_eq!(days.len(), 1);
    let day = &days[0];
    assert_eq!(day.my.iban.as_deref(), Some("DE12345678901234567890"));
    assert_eq!(day.my.bic.as_deref(), Some("ABCDEFG1ABC"));
    assert_eq!(day.my.curr.as_deref(), Some("EUR"));
    assert_eq!(day.start_type, 'F');
    assert_eq!(day.end_type, 'F');
    assert!(day.lines.is_empty());
    assert_eq!(
        day.start.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 100.00 EUR")
    );
    assert_eq!(
        day.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 110.50 EUR")
    );
}

#[test]
fn camt_report_shell_maps_debit_balances_negative_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_01_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">12.34</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">1.2</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );
    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_01).expect("CAMT shell parses");

    let day = &days[0];
    assert_eq!(
        day.start.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 -12.34 EUR")
    );
    assert_eq!(
        day.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 -1.20 EUR")
    );
}

#[test]
fn camt_report_shell_accepts_multiple_reports_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_01_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE11111111111111111111</IBAN></Id><Ccy>EUR</Ccy></Acct>
    </Rpt>
    <Rpt>
      <Acct>
        <Id><IBAN>DE22222222222222222222</IBAN></Id>
        <Ccy>USD</Ccy>
        <Svcr><FinInstnId><BICFI>USBICFI0</BICFI></FinInstnId></Svcr>
      </Acct>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_01).expect("CAMT shell parses");

    assert_eq!(days.len(), 2);
    assert_eq!(days[0].my.iban.as_deref(), Some("DE11111111111111111111"));
    assert_eq!(days[1].my.iban.as_deref(), Some("DE22222222222222222222"));
    assert_eq!(days[1].my.curr.as_deref(), Some("USD"));
    assert_eq!(days[1].my.bic.as_deref(), Some("USBICFI0"));
}

#[test]
fn camt_basic_entry_lines_map_amount_dates_and_running_balance_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_01_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">100.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Bal>
        <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">110.50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <RvslInd>false</RvslInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <ValDt><Dt>2018-07-21</Dt></ValDt>
        <AcctSvcrRef>NONREF</AcctSvcrRef>
        <AddtlNtryInf>DAUERAUFTRAG</AddtlNtryInf>
      </Ntry>
      <Ntry>
        <Amt Ccy="EUR">0.50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <ValDt><Dt>2018-07-22</Dt></ValDt>
        <AcctSvcrRef>NONREF2</AcctSvcrRef>
        <AddtlNtryInf>EINZAHLUNG</AddtlNtryInf>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_01).expect("CAMT lines parse");
    let day = &days[0];

    assert_eq!(day.lines.len(), 2);
    let first = &day.lines[0];
    assert!(first.is_sepa);
    assert!(first.is_camt);
    assert!(!first.is_storno);
    assert_eq!(
        first.value.as_ref().map(ToString::to_string).as_deref(),
        Some("10.00 EUR")
    );
    assert_eq!(
        first.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 110.00 EUR")
    );
    assert_eq!(first.bdate.as_deref(), Some("2018-07-20"));
    assert_eq!(first.valuta.as_deref(), Some("2018-07-21"));
    assert_eq!(first.customerref.as_deref(), Some("NONREF"));
    assert_eq!(first.text.as_deref(), Some("DAUERAUFTRAG"));
    assert_eq!(first.usage, vec!["DAUERAUFTRAG"]);

    let second = &day.lines[1];
    assert_eq!(
        second.value.as_ref().map(ToString::to_string).as_deref(),
        Some("0.50 EUR")
    );
    assert_eq!(
        second.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 110.50 EUR")
    );
    assert_eq!(second.valuta.as_deref(), Some("2018-07-22"));
    assert_eq!(second.customerref.as_deref(), Some("NONREF2"));
}

#[test]
fn camt_entry_lines_correct_running_balances_backwards_from_end_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_01_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>FWDB</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">999.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Bal>
        <Tp><CdOrPrtry><Cd>CLBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">107.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
      </Ntry>
      <Ntry>
        <Amt Ccy="EUR">3</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_01).expect("CAMT lines parse");
    let day = &days[0];

    assert!(day.start.is_none());
    assert_eq!(
        day.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 107.00 EUR")
    );
    assert_eq!(
        day.lines[0]
            .saldo
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("2018-07-20 110.00 EUR")
    );
    assert_eq!(
        day.lines[1]
            .saldo
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("2018-07-20 107.00 EUR")
    );
    assert_eq!(
        day.lines[1]
            .value
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("-3.00 EUR")
    );
}

#[test]
fn camt_basic_entry_lines_map_debit_storno_and_date_fallback_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_01_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">100</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">5.5</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <RvslInd>true</RvslInd>
        <ValDt><Dt>2018-07-22</Dt></ValDt>
        <AcctSvcrRef>REF-DEBIT</AcctSvcrRef>
        <AddtlNtryInf>LASTSCHRIFT</AddtlNtryInf>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_01).expect("CAMT lines parse");
    let line = &days[0].lines[0];

    assert!(line.is_storno);
    assert_eq!(line.bdate.as_deref(), Some("2018-07-22"));
    assert_eq!(line.valuta.as_deref(), Some("2018-07-22"));
    assert_eq!(
        line.value.as_ref().map(ToString::to_string).as_deref(),
        Some("-5.50 EUR")
    );
    assert_eq!(
        line.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-22 94.50 EUR")
    );
    assert_eq!(line.customerref.as_deref(), Some("REF-DEBIT"));
    assert_eq!(line.usage, vec!["LASTSCHRIFT"]);
}

#[test]
fn camt_transaction_details_map_credit_debtor_side_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_02_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <AcctSvcrRef>ENTRY-REF</AcctSvcrRef>
        <NtryDtls>
          <TxDtls>
            <Refs>
              <Prtry><Ref>TX-ID</Ref></Prtry>
              <AcctSvcrRef>TX-SVCR</AcctSvcrRef>
              <EndToEndId>E2E-123</EndToEndId>
              <MndtId>MND-123</MndtId>
            </Refs>
            <RltdPties>
              <Dbtr>
                <Nm>Debtor Name</Nm>
                <Id><PrvtId><Othr><Id>DE98ZZZ09999999999</Id></Othr></PrvtId></Id>
              </Dbtr>
              <DbtrAcct><Id><IBAN>DE02123456780000000000</IBAN></Id></DbtrAcct>
              <UltmtDbtr><Nm>Ultimate Debtor</Nm></UltmtDbtr>
              <Cdtr><Nm>Ignored Creditor</Nm></Cdtr>
              <CdtrAcct><Id><IBAN>DE99999999999999999999</IBAN></Id></CdtrAcct>
            </RltdPties>
            <RltdAgts>
              <DbtrAgt><FinInstnId><BIC>DEBTBIC0</BIC></FinInstnId></DbtrAgt>
              <CdtrAgt><FinInstnId><BIC>IGNORED0</BIC></FinInstnId></CdtrAgt>
            </RltdAgts>
            <RmtInf>
              <Ustrd>Invoice 1</Ustrd>
              <Ustrd>Invoice 2</Ustrd>
            </RmtInf>
            <Purp><Cd>GDDS</Cd></Purp>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_02).expect("CAMT details parse");
    let line = &days[0].lines[0];

    assert_eq!(line.customerref.as_deref(), Some("ENTRY-REF"));
    assert_eq!(line.id.as_deref(), Some("TX-ID"));
    assert_eq!(line.end_to_end_id.as_deref(), Some("E2E-123"));
    assert_eq!(line.mandate_id.as_deref(), Some("MND-123"));
    assert_eq!(line.usage, vec!["Invoice 1", "Invoice 2"]);
    assert_eq!(line.purposecode.as_deref(), Some("GDDS"));

    let other = line.other.as_ref().expect("counter account is present");
    assert_eq!(other.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(other.name.as_deref(), Some("Debtor Name"));
    assert_eq!(other.name2.as_deref(), Some("Ultimate Debtor"));
    assert_eq!(other.bic.as_deref(), Some("DEBTBIC0"));
    assert_eq!(other.creditorid.as_deref(), Some("DE98ZZZ09999999999"));
}

#[test]
fn camt_transaction_details_map_debit_creditor_side_and_id_fallback_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_02_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">5</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <AcctSvcrRef>ENTRY-REF</AcctSvcrRef>
        <NtryDtls>
          <TxDtls>
            <Refs>
              <AcctSvcrRef>TX-SVCR</AcctSvcrRef>
              <EndToEndId>NOTPROVIDED</EndToEndId>
            </Refs>
            <RltdPties>
              <Dbtr><Nm>Ignored Debtor</Nm></Dbtr>
              <DbtrAcct><Id><IBAN>DE99999999999999999999</IBAN></Id></DbtrAcct>
              <Cdtr>
                <Nm>Creditor Name</Nm>
                <Id><PrvtId><Othr><Id>DE12ZZZ00000000000</Id></Othr></PrvtId></Id>
              </Cdtr>
              <CdtrAcct><Id><IBAN>DE03123456780000000000</IBAN></Id></CdtrAcct>
              <UltmtCdtr><Nm>Ultimate Creditor</Nm></UltmtCdtr>
            </RltdPties>
            <RltdAgts>
              <CdtrAgt><FinInstnId><BICFI>CRDTBIC0</BICFI></FinInstnId></CdtrAgt>
            </RltdAgts>
            <RmtInf><Ustrd>Debit Usage</Ustrd></RmtInf>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_02).expect("CAMT details parse");
    let line = &days[0].lines[0];

    assert_eq!(
        line.value.as_ref().map(ToString::to_string).as_deref(),
        Some("-5.00 EUR")
    );
    assert_eq!(line.id.as_deref(), Some("ENTRY-REF"));
    assert_eq!(line.end_to_end_id.as_deref(), Some("NOTPROVIDED"));
    assert_eq!(line.usage, vec!["Debit Usage"]);

    let other = line.other.as_ref().expect("counter account is present");
    assert_eq!(other.iban.as_deref(), Some("DE03123456780000000000"));
    assert_eq!(other.name.as_deref(), Some("Creditor Name"));
    assert_eq!(other.name2.as_deref(), Some("Ultimate Creditor"));
    assert_eq!(other.bic.as_deref(), Some("CRDTBIC0"));
    assert_eq!(other.creditorid.as_deref(), Some("DE12ZZZ00000000000"));
}

#[test]
fn camt_transaction_details_return_info_flips_counterparty_and_maps_original_amount_like_original()
{
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_02_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <AcctSvcrRef>ENTRY-REF</AcctSvcrRef>
        <NtryDtls>
          <TxDtls>
            <AmtDtls>
              <InstdAmt><Amt Ccy="USD">42.424</Amt></InstdAmt>
            </AmtDtls>
            <RltdPties>
              <Dbtr><Nm>Original Debtor</Nm></Dbtr>
              <DbtrAcct><Id><IBAN>DE02123456780000000000</IBAN></Id></DbtrAcct>
              <Cdtr><Nm>Returned Creditor</Nm></Cdtr>
              <CdtrAcct><Id><IBAN>DE03123456780000000000</IBAN></Id></CdtrAcct>
              <UltmtCdtr><Nm>Ultimate Returned Creditor</Nm></UltmtCdtr>
            </RltdPties>
            <RltdAgts>
              <DbtrAgt><FinInstnId><BIC>DEBTBIC0</BIC></FinInstnId></DbtrAgt>
              <CdtrAgt><FinInstnId><BIC>CRETDBIC</BIC></FinInstnId></CdtrAgt>
            </RltdAgts>
            <RtrInf>
              <Rsn><Cd>AC01</Cd></Rsn>
              <AddtlInf>Wrong account</AddtlInf>
              <AddtlInf>Returned by bank</AddtlInf>
            </RtrInf>
            <RmtInf><Ustrd>Return Usage</Ustrd></RmtInf>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_02).expect("CAMT return parses");
    let line = &days[0].lines[0];

    assert_eq!(
        line.value.as_ref().map(ToString::to_string).as_deref(),
        Some("10.00 EUR")
    );
    assert_eq!(
        line.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 60.00 EUR")
    );
    assert_eq!(
        line.orig_value.as_ref().map(ToString::to_string).as_deref(),
        Some("42.42 USD")
    );
    assert_eq!(
        line.additional.as_deref(),
        Some("Wrong account,Returned by bank")
    );
    assert_eq!(line.usage, vec!["Return Usage"]);

    let other = line.other.as_ref().expect("counter account is present");
    assert_eq!(other.iban.as_deref(), Some("DE03123456780000000000"));
    assert_eq!(other.name.as_deref(), Some("Returned Creditor"));
    assert_eq!(other.name2.as_deref(), Some("Ultimate Returned Creditor"));
    assert_eq!(other.bic.as_deref(), Some("CRETDBIC"));
}

#[test]
fn camt_transaction_details_maps_proprietary_bank_code_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_02_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <NtryDtls>
          <TxDtls>
            <BkTxCd><Prtry><Cd>SBOOK+152+9245+53</Cd></Prtry></BkTxCd>
            <Purp><Cd>GDDS</Cd></Purp>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_02).expect("CAMT details parse");
    let line = &days[0].lines[0];

    assert_eq!(line.gvcode.as_deref(), Some("152"));
    assert_eq!(line.primanota.as_deref(), Some("9245"));
    assert_eq!(line.addkey.as_deref(), Some("53"));
    assert_eq!(line.purposecode.as_deref(), Some("GDDS"));
}

#[test]
fn camt_transaction_details_ignores_malformed_proprietary_bank_code_like_java_split() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_02_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <BookgDt><Dt>2018-07-20</Dt></BookgDt>
        <NtryDtls>
          <TxDtls>
            <BkTxCd><Prtry><Cd>SBOOK+152+9245+</Cd></Prtry></BkTxCd>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_02).expect("CAMT details parse");
    let line = &days[0].lines[0];

    assert_eq!(line.gvcode, None);
    assert_eq!(line.primanota, None);
    assert_eq!(line.addkey, None);
}

#[test]
fn camt_transaction_details_skip_entry_when_first_detail_has_no_tx_like_original() {
    let xml = format!(
        r#"<Document xmlns="{CAMT_052_001_02_URN}">
  <BkToCstmrAcctRpt>
    <Rpt>
      <Acct><Id><IBAN>DE12345678901234567890</IBAN></Id><Ccy>EUR</Ccy></Acct>
      <Bal>
        <Tp><CdOrPrtry><Cd>ITBD</Cd></CdOrPrtry></Tp>
        <Amt Ccy="EUR">50</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <Dt><Dt>2018-07-20</Dt></Dt>
      </Bal>
      <Ntry>
        <Amt Ccy="EUR">10</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <NtryDtls/>
      </Ntry>
    </Rpt>
  </BkToCstmrAcctRpt>
</Document>"#
    );

    let days =
        parse_camt_report_shell(&xml, SepaVersion::CAMT_052_001_02).expect("CAMT details parse");

    assert!(days[0].lines.is_empty());
}

#[test]
fn camt_upstream_05200102_fixture_matches_original_test004_observable_fields() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-parse-05200102.xml");

    let version = SepaVersion::autodetect(xml)
        .expect("valid upstream fixture")
        .expect("known CAMT version");
    assert_eq!(version, SepaVersion::CAMT_052_001_02);

    let days = parse_camt_report_shell(xml, version).expect("upstream CAMT fixture parses");

    assert_eq!(days.len(), 1);
    let day = &days[0];
    assert_eq!(day.lines.len(), 2);
    assert_eq!(
        day.start.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 100.00 EUR")
    );
    assert_eq!(
        day.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 110.50 EUR")
    );
    assert_eq!(day.my.iban.as_deref(), Some("DE12345678901234567890"));
    assert_eq!(day.my.bic.as_deref(), Some("ABCDEFG1ABC"));
    assert_eq!(day.my.curr.as_deref(), Some("EUR"));
    assert_eq!(day.start_type, 'F');
    assert_eq!(day.end_type, 'F');

    let first = &day.lines[0];
    assert_eq!(first.additional, None);
    assert_eq!(first.addkey.as_deref(), Some("000"));
    assert_eq!(first.bdate.as_deref(), Some("2018-07-20"));
    assert_eq!(first.charge_value, None);
    assert_eq!(first.customerref.as_deref(), Some("NONREF"));
    assert_eq!(first.gvcode.as_deref(), Some("152"));
    assert_eq!(first.id.as_deref(), Some("2018-07-20-07.51.25.370057"));
    assert_eq!(first.instref, None);
    assert!(first.is_camt);
    assert!(first.is_sepa);
    assert!(!first.is_storno);
    assert_eq!(first.orig_value, None);
    assert_eq!(first.primanota.as_deref(), Some("9201"));
    assert_eq!(first.purposecode.as_deref(), Some("RINP"));
    assert_eq!(first.text.as_deref(), Some("DAUERAUFTRAG"));
    assert_eq!(first.usage, vec!["Verwendungszweck 1"]);
    assert_eq!(first.valuta.as_deref(), Some("2018-07-21"));
    assert_eq!(
        first.value.as_ref().map(ToString::to_string).as_deref(),
        Some("10.00 EUR")
    );
    assert_eq!(
        first.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 110.00 EUR")
    );
    let first_other = first.other.as_ref().expect("first counter account");
    assert_eq!(first_other.iban.as_deref(), Some("DE12345678901234567891"));
    assert_eq!(first_other.bic.as_deref(), Some("ABCDEFG2ABC"));
    assert_eq!(first_other.name.as_deref(), Some("Max Mustermann"));

    let second = &day.lines[1];
    assert_eq!(second.additional, None);
    assert_eq!(second.addkey.as_deref(), Some("000"));
    assert_eq!(second.bdate.as_deref(), Some("2018-07-20"));
    assert_eq!(second.charge_value, None);
    assert_eq!(second.customerref.as_deref(), Some("NONREF"));
    assert_eq!(second.gvcode.as_deref(), Some("152"));
    assert_eq!(second.id.as_deref(), Some("2018-07-20-07.51.28.370057"));
    assert_eq!(second.instref, None);
    assert!(second.is_camt);
    assert!(second.is_sepa);
    assert!(!second.is_storno);
    assert_eq!(second.orig_value, None);
    assert_eq!(second.primanota.as_deref(), Some("9201"));
    assert_eq!(second.purposecode.as_deref(), Some("DEPT"));
    assert_eq!(second.text.as_deref(), Some("EINZAHLUNG"));
    assert_eq!(second.usage, vec!["Verwendungszweck 2"]);
    assert_eq!(second.valuta.as_deref(), Some("2018-07-22"));
    assert_eq!(
        second.value.as_ref().map(ToString::to_string).as_deref(),
        Some("0.50 EUR")
    );
    assert_eq!(
        second.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2018-07-20 110.50 EUR")
    );
    let second_other = second.other.as_ref().expect("second counter account");
    assert_eq!(second_other.iban.as_deref(), Some("DE12345678901234567892"));
    assert_eq!(second_other.bic.as_deref(), Some("ABCDEFG3ABC"));
    assert_eq!(second_other.name.as_deref(), Some("Bert Bezahler"));
}

#[test]
fn camt_upstream_return_fixture_matches_original_test005_observable_fields() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-ruecklastschrift.xml");

    let version = SepaVersion::autodetect(xml)
        .expect("valid upstream fixture")
        .expect("known CAMT version");
    assert_eq!(version, SepaVersion::CAMT_052_001_02);

    let days = parse_camt_report_shell(xml, version).expect("upstream CAMT return fixture parses");

    assert_eq!(days.len(), 1);
    let day = &days[0];
    assert_eq!(day.lines.len(), 1);
    assert_eq!(
        day.start.as_ref().map(ToString::to_string).as_deref(),
        Some("2021-03-26 100.00 EUR")
    );
    assert_eq!(
        day.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2021-03-26 100.00 EUR")
    );

    let line = &day.lines[0];
    assert_eq!(
        line.value.as_ref().map(ToString::to_string).as_deref(),
        Some("-53.00 EUR")
    );
    assert_eq!(
        line.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2021-03-26 47.00 EUR")
    );
    assert_eq!(
        line.orig_value.as_ref().map(ToString::to_string).as_deref(),
        Some("50.00 EUR")
    );
    assert_eq!(
        line.additional.as_deref(),
        Some("RUECKLASTSCHRIFT Sonstige Gruende")
    );
    assert_eq!(line.mandate_id.as_deref(), Some("TEST1234"));
    assert_eq!(line.id.as_deref(), Some("2021-03-26-09-12345"));
    assert_eq!(line.customerref.as_deref(), Some("NONREF"));
    assert_eq!(line.text.as_deref(), Some("LS RUECKBELASTUNG"));
    assert_eq!(line.usage, vec!["RUECKLASTSCHRIFT Sonstige Gruende"]);
    assert_eq!(line.bdate.as_deref(), Some("2021-03-26"));
    assert_eq!(line.valuta.as_deref(), Some("2021-03-26"));

    let other = line.other.as_ref().expect("return counter account");
    assert_eq!(other.iban.as_deref(), Some("DES1234567890"));
    assert_eq!(other.bic.as_deref(), Some("TESTS1234"));
    assert_eq!(other.name.as_deref(), Some("Sven Schuldner"));
}

#[test]
fn camt_upstream_05200108_fixture_matches_original_test006_observable_fields() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-parse-05200108.xml");

    let version = SepaVersion::autodetect(xml)
        .expect("valid upstream fixture")
        .expect("known CAMT version");
    assert_eq!(version, SepaVersion::CAMT_052_001_08);

    let days = parse_camt_report_shell(xml, version).expect("upstream CAMT 052.001.08 parses");

    assert_eq!(days.len(), 1);
    let day = &days[0];
    assert_eq!(day.lines.len(), 1);
    assert_eq!(
        day.start.as_ref().map(ToString::to_string).as_deref(),
        Some("2023-11-08 100.00 EUR")
    );
    assert_eq!(
        day.end.as_ref().map(ToString::to_string).as_deref(),
        Some("2023-11-10 66.00 EUR")
    );
    assert_eq!(day.my.iban.as_deref(), Some("DE12345678901234567890"));
    assert_eq!(day.my.bic.as_deref(), Some("ABCDEFG1ABC"));
    assert_eq!(day.my.curr.as_deref(), Some("EUR"));
    assert_eq!(day.start_type, 'F');
    assert_eq!(day.end_type, 'F');

    let line = &day.lines[0];
    assert_eq!(line.additional, None);
    assert_eq!(line.addkey.as_deref(), Some("992"));
    assert_eq!(line.bdate.as_deref(), Some("2023-11-10"));
    assert_eq!(line.charge_value, None);
    assert_eq!(
        line.customerref.as_deref(),
        Some("2023-11-10-00.06.42.329883")
    );
    assert_eq!(line.gvcode.as_deref(), Some("105"));
    assert_eq!(line.id.as_deref(), Some("2023-11-10-00.06.42.329883"));
    assert_eq!(line.instref, None);
    assert!(line.is_camt);
    assert!(line.is_sepa);
    assert!(!line.is_storno);
    assert_eq!(line.orig_value, None);
    assert_eq!(line.primanota.as_deref(), Some("9200"));
    assert_eq!(line.purposecode, None);
    assert_eq!(line.text.as_deref(), Some("FOLGELASTSCHRIFT"));
    assert_eq!(line.usage, vec!["Verwendungszweck"]);
    assert_eq!(line.valuta.as_deref(), Some("2023-11-10"));
    assert_eq!(
        line.value.as_ref().map(ToString::to_string).as_deref(),
        Some("-34.00 EUR")
    );
    assert_eq!(
        line.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("2023-11-10 66.00 EUR")
    );
    let other = line.other.as_ref().expect("counter account");
    assert_eq!(other.iban.as_deref(), Some("DE12345678901234567892"));
    assert_eq!(other.bic.as_deref(), Some("ABCDEFG1CBA"));
    assert_eq!(other.name.as_deref(), Some("Beispiel AG"));
    assert_eq!(other.creditorid.as_deref(), Some("DE46ZZZ00000012345"));
}

#[test]
fn camt_upstream_05200108_missing_balance_dates_parse_like_original_test007() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-parse-5200108-missing-date.xml");

    let version = SepaVersion::autodetect(xml)
        .expect("valid upstream fixture")
        .expect("known CAMT version");
    assert_eq!(version, SepaVersion::CAMT_052_001_08);

    parse_camt_report_shell(xml, version).expect("upstream missing-date CAMT parses");
}

#[test]
fn camt_upstream_05200108_invalid_saldo_amounts_parse_like_original_test008() {
    let xml = include_str!("fixtures/hbci4java/sepa/test-camt-parse-5200108-invalid-saldo.xml");

    let version = SepaVersion::autodetect(xml)
        .expect("valid upstream fixture")
        .expect("known CAMT version");
    assert_eq!(version, SepaVersion::CAMT_052_001_08);

    parse_camt_report_shell(xml, version).expect("upstream invalid-saldo CAMT parses");
}
