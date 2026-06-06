use hbci4rust::sepa::{
    CAMT_052_001_01_URN, CAMT_052_001_02_URN, CAMT_052_001_04_URN, CAMT_052_001_07_URN,
    CAMT_052_001_08_URN, SepaKind, SepaVersion, parse_camt_report_shell,
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
