//! EN 319 412-5's statements read one clause at a time.
//!
//! **Every fixture is written here from the clause's ASN.1**, octet by octet, so each expected
//! value is the clause's syntax read back rather than anything this reader computed. The one
//! certificate was made with `openssl req -x509` under the round's scratch directory with the
//! extension given as those same octets (`1.3.6.1.5.5.7.1.3 = DER:...`), so it is the end-to-end
//! case: `pdf_pki::x509` finds the extension, and this module reads it.

use super::{
    QC_CC_LEGISLATION, QC_COMPLIANCE, QC_SSCD, QC_TYPE, QcStatementError, QcStatements, QcType,
    Stated,
};
use crate::x509_fixtures::hex;
use pdf_pki::x509::{QC_STATEMENTS, dotted, parse};

/// A self-signed certificate whose `qcStatements` are [`STATED`].
const QC_CERT: &str = "\
    308203763082025ea00302010202010b300d06092a864886f70d01010b0500303a310b3009060355040613024154\
    31173015060355040a0c0e54657374205143204973737565723112301006035504030c095143207369676e657230\
    1e170d3236313131333034313532335a170d3336313131303034313532335a303a310b3009060355040613024154\
    31173015060355040a0c0e54657374205143204973737565723112301006035504030c095143207369676e657230\
    820122300d06092a864886f70d01010105000382010f003082010a0282010100aae12b4fb3b597c5921d1238c526\
    1febc6791123ecf194bb29cdb6108fd4f9dee959fd1552ada4c18a19b057fb2f90443453960570c2f5a0d91afd14\
    30c839476c642940465633bd20b39b8c54bb5d53e8637dce59010e12823cd6fb7848b56a89276eca699153e4a57b\
    5fcdffd78ae0433c8b742d495ad45564a8ac901717cb09815472a48014a476ea5eeb1dd68f395e42b54091e377a5\
    64f0ccd4dd3fbd2b2f10cb8d80477381cc21057d69703a7f9df855d73c372e10c794aca5ad825e77ade4073cd502\
    5385f71dc3d915e6a9ee02936ee9ba8d93690fc127ef0058d91fc6488ccf390b09653220e64ae247632d30eea983\
    4aaa5fb5fc662882dc4873af0203010001a38186308183300c0603551d130101ff04023000300e0603551d0f0101\
    ff040403020640304406082b06010505070103043830363008060604008e4601013008060604008e460104301306\
    0604008e4601063009060704008e46010601300b060604008e46010302010a301d0603551d0e041604144b92368e\
    030c136d39a1b3cf3ee1088a0ffd726b300d06092a864886f70d01010b0500038201010051920b7c8f6fea947a29\
    3f757367c72ef767c0cb841b27b9b36d30235d886617257ee872bd5bc72eed9cf8c1aff06957d9c1f40beb1977ae\
    4ddc38a22a2f67e889ddc91b96174aa87f659267e9bc513bc3c55eac9ecf5ec9ba574e305a598ac9914eef43b6ef\
    c284420332a75246f90cd49f283765594dda9974e121d769e6b93770ecc4878990ee11b8519992ff474963e8d0a7\
    26a2db0b394cb81100122980ea63ab4501b03c6cdba7eb6f34e3b6c06a7fa0efe895439aa1d779e97311e03f6d2e\
    865a8f4bc4613520f70d2170f580a0040a55e01fcff5893f72d136b4786b480d3206143e82d080ad7b0e09688b83\
    62d1b3f81c356e95e8885b3f76be57ac\
";

/// `QcCompliance`, `QcSSCD`, `QcType { id-etsi-qct-esign }` and clause 4.3.3's
/// `QcRetentionPeriod` of 10 years — the extension's `extnValue` in [`QC_CERT`].
const STATED: &str = "\
    30363008060604008e4601013008060604008e4601043013060604008e4601063009060704008e46010601300b\
    060604008e46010302010a\
";

/// One DER value, short form.
fn tlv(tag: u8, contents: &[u8]) -> Vec<u8> {
    let length = u8::try_from(contents.len()).expect("a short fixture");
    [&[tag, length][..], contents].concat()
}

/// One `QCStatement ::= SEQUENCE { statementId, statementInfo OPTIONAL }`.
fn statement(id: &[u8], info: Option<Vec<u8>>) -> Vec<u8> {
    tlv(0x30, &[tlv(0x06, id), info.unwrap_or_default()].concat())
}

/// The identifiers are the Annex B module's, as dotted decimal.
#[test]
fn the_identifiers_are_annex_bs() {
    assert_eq!(dotted(QC_STATEMENTS).as_deref(), Some("1.3.6.1.5.5.7.1.3"));
    assert_eq!(dotted(QC_COMPLIANCE).as_deref(), Some("0.4.0.1862.1.1"));
    assert_eq!(dotted(QC_SSCD).as_deref(), Some("0.4.0.1862.1.4"));
    assert_eq!(dotted(QC_TYPE).as_deref(), Some("0.4.0.1862.1.6"));
    assert_eq!(dotted(QC_CC_LEGISLATION).as_deref(), Some("0.4.0.1862.1.7"));
}

/// Clauses 4.2.1 to 4.2.3, read off a certificate: the claim of compliance, the device, the one
/// purpose `esign`, and clause 4.3.3's statement named by its identifier. No `QcCClegislation`
/// is kept as none (EN 319 412-5 Table 1A's "Absent" row, the EU texts).
#[test]
fn a_certificates_statements_are_read_as_the_clauses_define_them() {
    let der = hex(QC_CERT);
    let certificate = parse(&der).expect("the certificate parses");
    assert_eq!(
        certificate.extensions.qc_statements,
        Some(hex(STATED).as_slice())
    );
    assert_eq!(
        Stated::of(&certificate),
        Stated::Read(QcStatements {
            compliance: true,
            sscd: true,
            types: Some(vec![QcType::Esign]),
            legislation: None,
            others: vec!["0.4.0.1862.1.3".to_owned()],
        })
    );
    // A certificate with no extension states nothing, which is not a statement of anything.
    let plain = hex(crate::x509_fixtures::CERTIFICATE);
    assert_eq!(
        Stated::of(&parse(&plain).expect("a certificate")),
        Stated::Nothing
    );
}

/// Clause 4.2.3's three purposes and the extension marker, and clause 4.2.4's countries; an empty
/// sequence of statements is read as stating none of them.
#[test]
fn types_and_countries_are_read_as_written() {
    let qct = |last: u8| tlv(0x06, &[QC_TYPE, &[last]].concat());
    let types = statement(QC_TYPE, Some(tlv(0x30, &[qct(2), qct(3), qct(9)].concat())));
    let countries = statement(
        QC_CC_LEGISLATION,
        Some(tlv(0x30, &[tlv(0x13, b"CH"), tlv(0x13, b"NO")].concat())),
    );
    let read = QcStatements::read(&tlv(0x30, &[types, countries].concat())).expect("read");
    assert_eq!(
        read.types,
        Some(vec![
            QcType::Eseal,
            QcType::Web,
            QcType::Other("0.4.0.1862.1.6.9".to_owned())
        ])
    );
    assert_eq!(
        read.legislation,
        Some(vec!["CH".to_owned(), "NO".to_owned()])
    );
    assert!(!read.compliance && !read.sscd);
    assert_eq!(
        QcStatements::read(&tlv(0x30, &[])),
        Ok(QcStatements::default())
    );
}

/// What does not have its clause's syntax is refused by the clause's name.
#[test]
fn a_statement_without_its_syntax_is_refused_by_its_clause() {
    // QcType's `statementInfo` is required by its `SYNTAX`; a bare identifier is not one.
    assert_eq!(
        QcStatements::read(&tlv(0x30, &statement(QC_TYPE, None))),
        Err(QcStatementError::Syntax {
            statement: "QcType",
            clause: "4.2.3"
        })
    );
    // `CountryName ::= PrintableString (SIZE (2))`: three letters, or a UTF8String, are not one.
    for country in [tlv(0x13, b"AUT"), tlv(0x0C, b"AT"), tlv(0x13, b"A1")] {
        let legislation = statement(QC_CC_LEGISLATION, Some(tlv(0x30, &country)));
        assert_eq!(
            QcStatements::read(&tlv(0x30, &legislation)),
            Err(QcStatementError::Syntax {
                statement: "QcCClegislation",
                clause: "4.2.4"
            })
        );
    }
    let twice = [statement(QC_SSCD, None), statement(QC_SSCD, None)].concat();
    assert_eq!(
        QcStatements::read(&tlv(0x30, &twice)),
        Err(QcStatementError::Repeated("QcSSCD"))
    );
    // RFC 3739's outer `SEQUENCE OF`, and nothing after it.
    assert_eq!(
        QcStatements::read(&tlv(0x31, &[])),
        Err(QcStatementError::Malformed)
    );
    assert_eq!(
        QcStatements::read(&[tlv(0x30, &[]), tlv(0x05, &[])].concat()),
        Err(QcStatementError::Malformed)
    );
    assert_eq!(QcStatements::read(&[]), Err(QcStatementError::Malformed));
}

/// The walk is bounded: one statement past [`super::MAX_ENTRIES`] is refused, not allocated.
#[test]
fn the_walk_is_bounded() {
    let unknown = statement(&[0x2A, 0x03], None);
    let many = unknown.repeat(super::MAX_ENTRIES + 1);
    // Long form: `0x82` and two octets of length.
    let length = u16::try_from(many.len()).expect("fits");
    let encoded = [&[0x30, 0x82][..], &length.to_be_bytes(), &many].concat();
    assert_eq!(QcStatements::read(&encoded), Err(QcStatementError::TooMany));
    let enough = unknown.repeat(super::MAX_ENTRIES);
    let length = u16::try_from(enough.len()).expect("fits");
    let encoded = [&[0x30, 0x82][..], &length.to_be_bytes(), &enough].concat();
    assert_eq!(
        QcStatements::read(&encoded).map(|read| read.others.len()),
        Ok(super::MAX_ENTRIES)
    );
}
