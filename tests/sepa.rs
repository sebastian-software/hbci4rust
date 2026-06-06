use hbci4rust::sepa::{
    CAMT_052_001_01_URN, CAMT_052_001_04_URN, CAMT_052_001_07_URN, CAMT_052_001_08_URN, SepaKind,
    SepaVersion, parse_camt_report_shell,
};

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
fn camt_version_autodetects_root_namespace() {
    let version = SepaVersion::autodetect(&camt_document(CAMT_052_001_08_URN)).expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_08));
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
