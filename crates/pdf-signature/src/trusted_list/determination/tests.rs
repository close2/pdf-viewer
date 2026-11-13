//! TS 119 615 clause 4.4 run requirement by requirement, and TS 119 612 clause 5.5.9.2's
//! qualifications read and asked of a certificate.
//!
//! The certificates are this round's own (`r2024/gen/gen.sh`): one CA, `C=AT, O=Example Trust
//! Provider`, and three signers it issued, with P-256 keys generated for them. The lists are
//! written by `crate::trusted_list::tests::list_xml` with the CA as a `CA/QC` service's identity.
//! Every expected value is read off the clause's text or its Tables 1 to 5 by the row and column
//! the comment names; no published list or other validator's output decided one.

use super::*;
use crate::trusted_list::qualifications::{Assert, Assertion, CriteriaList, Field};
use crate::trusted_list::tests::{Written, list_xml};
use crate::trusted_list::{FOR_TYPES, GRANTED};
use crate::x509_fixtures::hex;

/// The self-signed CA, `C=AT, O=Example Trust Provider, CN=Example QC CA`, 2010 to 2040.
pub(crate) const CA: &str = "\
    308201bc30820163a003020102020101300a06082a8648ce3d0403023046310b3009060355040613024154311f30\
    1d060355040a0c164578616d706c652054727573742050726f76696465723116301406035504030c0d4578616d70\
    6c65205143204341301e170d3130303130313030303030305a170d3430303130313030303030305a3046310b3009\
    060355040613024154311f301d060355040a0c164578616d706c652054727573742050726f766964657231163014\
    06035504030c0d4578616d706c652051432043413059301306072a8648ce3d020106082a8648ce3d030107034200\
    041552220881034b42043b34b2495f8602e6a6c3f8023dfbce4f3235f522f83d746166d7b957963657cd05e72012\
    9af58f511567db8001dc189d82f6be6e7aa2c6a3423040300f0603551d130101ff040530030101ff300e0603551d\
    0f0101ff040403020106301d0603551d0e041604147106fd7aa8e2fb73f6035e7bc7803d075ff72ede300a06082a\
    8648ce3d04030203470030440220484dcda48f7d7931001b20d103b1fa21c1b95e21f7759869f9dd6e11297fcba5\
    02207140150f1a5e320c283103f0fcc1d88d2c517ce5cd8ef80429c19edf5a8d9ae3";

/// Issued by [`CA`] on 2025-01-01: `QcCompliance` and `QcType` `esign` (EN 319 412-5 clauses 4.2.1 and 4.2.3), `keyUsage` `nonRepudiation`.
pub(crate) const ESIG: &str = "\
    308201f43082019aa003020102020102300a06082a8648ce3d0403023046310b3009060355040613024154311f30\
    1d060355040a0c164578616d706c652054727573742050726f76696465723116301406035504030c0d4578616d70\
    6c65205143204341301e170d3235303130313030303030305a170d3335303130313030303030305a302e310b3009\
    060355040613024154311f301d06035504030c165369676e657220576974682053746174656d656e747330593013\
    06072a8648ce3d020106082a8648ce3d03010703420004619ba2fea9c451761a171ede4a9281abded794054e7dbc\
    5fc8f9c3d3964aec1035d1c4fff86fb2151794f7162d2a9a46bddbc616561023465593dcc485f4d5a5a381903081\
    8d300c0603551d130101ff04023000300e0603551d0f0101ff040403020640301f0603551d230418301680147106\
    fd7aa8e2fb73f6035e7bc7803d075ff72ede302d06082b060105050701030421301f3008060604008e4601013013\
    060604008e4601063009060704008e46010601301d0603551d0e041604148f480570d9a43a8408737ff26ae82add\
    66dc3e7d300a06082a8648ce3d0403020348003045022100bd9a0fad96e68cd446100aacec061484efdeeb878936\
    8bd6d6cee94b31c67a46022001fc6f10c956cf98151e0e2b439f33292a22397a2e1fa49ace8a8df2f7b3bfe3";

/// Issued by [`CA`] on 2025-01-01: no `qcStatements`, `keyUsage` `nonRepudiation`.
pub(crate) const PLAIN: &str = "\
    308201c73082016ca003020102020103300a06082a8648ce3d0403023046310b3009060355040613024154311f30\
    1d060355040a0c164578616d706c652054727573742050726f76696465723116301406035504030c0d4578616d70\
    6c65205143204341301e170d3235303130313030303030305a170d3335303130313030303030305a3031310b3009\
    0603550406130241543122302006035504030c195369676e657220576974686f75742053746174656d656e747330\
    59301306072a8648ce3d020106082a8648ce3d030107034200044c096ef931fa1ed804434e9de75d78866dd291c7\
    3f97b0c49ae0d15c99ae90e1d1361ed1c1fe3c2a17e3b11096c685b548d2e69a8f38e988531c771bfb4575b0a360\
    305e300c0603551d130101ff04023000300e0603551d0f0101ff040403020640301f0603551d2304183016801471\
    06fd7aa8e2fb73f6035e7bc7803d075ff72ede301d0603551d0e041604145edeae4e1333d816ac457565e84f2be4\
    57f9b51e300a06082a8648ce3d0403020349003046022100de2793c885e326f0c18bb5d568c08f4613a760c91ad5\
    5f971cf146e688421a70022100cd581592db068ed3749fab0fb92eb3db371bce25a8ce62e570501e7d043993f4";

/// Issued by [`CA`] on 2015-01-01, under Directive 1999/93/EC: no `qcStatements`, the certificate policy QCP+ (`0.4.0.1456.1.1`).
pub(crate) const OLD: &str = "\
    308201dc30820182a003020102020104300a06082a8648ce3d0403023046310b3009060355040613024154311f30\
    1d060355040a0c164578616d706c652054727573742050726f76696465723116301406035504030c0d4578616d70\
    6c65205143204341301e170d3135303130313030303030305a170d3335303130313030303030305a3032310b3009\
    0603550406130241543123302106035504030c1a5369676e657220556e6465722054686520446972656374697665\
    3059301306072a8648ce3d020106082a8648ce3d030107034200046a4dec4f55d4507dcb4026c0ade83c6339b3b7\
    722d890c206f2d2ae258b58688f518576e68fd82b5a86e8480d46c4281175dc1fc98832714ca9ad8bd7821b494a3\
    753073300c0603551d130101ff04023000300e0603551d0f0101ff040403020640301f0603551d23041830168014\
    7106fd7aa8e2fb73f6035e7bc7803d075ff72ede30130603551d20040c300a3008060604008b300101301d060355\
    1d0e04160414f403e9e6db4d3690a20c3b0d3260d7a407bb9845300a06082a8648ce3d0403020348003045022100\
    8054cd14e2212b7a39f0be8ef8c182c7e1929fee3b3ef2cd2d71592171bd39f302204e34e8063668f69a003fe25d\
    fbdfe1e5f87528b825a94f560c9a172a3a2daba1";

/// 2026-06-01T00:00:00Z, the best signature time the tests ask about.
const AT: Instant = Instant::from_unix_seconds(1_780_272_000);

/// The CA's service, granted since the Regulation applies, for electronic signatures, under
/// supervision before that.
fn service(identity: &[u8]) -> Written<'_> {
    Written {
        service_type: CA_QC,
        status: GRANTED,
        since: "2016-07-01T00:00:00Z",
        history: &[(
            "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/undersupervision",
            "2010-01-01T00:00:00Z",
        )],
        for_type: Some(FOR_TYPES[0]),
        identity: Some(identity),
    }
}

/// Austria's list naming `provider` with `services`, each service's extensions preceded by
/// `extension` (a whole `tsl:Extension`, or nothing).
fn list(provider: &str, services: &[Written<'_>], extension: &str) -> TrustedList {
    let xml = list_xml("AT", &[(provider, services)]).replace(
        "<tsl:ServiceInformationExtensions>",
        &format!("<tsl:ServiceInformationExtensions>{extension}"),
    );
    TrustedList::read(xml.as_bytes()).expect("the written list reads")
}

/// A `Qualifications` extension of one element: `qualifiers`, under a criteria list asserting
/// `assert` over `criteria`.
fn qualifications(critical: bool, qualifiers: &[&str], assert: &str, criteria: &str) -> String {
    let qualifiers = qualifiers.iter().fold(String::new(), |mut written, uri| {
        written.push_str("<sie:Qualifier uri=\"");
        written.push_str(uri);
        written.push_str("\"/>");
        written
    });
    format!(
        "<tsl:Extension Critical=\"{critical}\">\
         <sie:Qualifications xmlns:sie=\"{SIE}\"><sie:QualificationElement>\
         <sie:Qualifiers>{qualifiers}</sie:Qualifiers>\
         <sie:CriteriaList assert=\"{assert}\">{criteria}</sie:CriteriaList>\
         </sie:QualificationElement></sie:Qualifications></tsl:Extension>",
        SIE = super::super::qualifications::SIE_NAMESPACE,
    )
}

const NON_REPUDIATION: &str =
    "<sie:KeyUsage><sie:KeyUsageBit name=\"nonRepudiation\">true</sie:KeyUsageBit></sie:KeyUsage>";

/// Clause 4.4 for `der` (issued by [`CA`]) over `lists` at [`AT`].
fn determined(der: &str, lists: &[TrustedList]) -> Qualification {
    let der = hex(der);
    let certificate = x509::parse(&der).expect("the certificate parses");
    let ca = hex(CA);
    let pool = [x509::parse(&ca).expect("the CA parses")];
    determine(lists, &certificate, &pool, AT)
}

fn outputs(qualification: Qualification) -> Determination {
    match qualification {
        Qualification::Determined {
            territory,
            determination,
            ..
        } => {
            assert_eq!(territory, "AT", "PRO-4.4.4-01: the issuer's countryName");
            determination
        }
        other => panic!("the procedure should have run: {other:?}"),
    }
}

/// Tables 1 to 3 as printed: the three EXAMPLEs (row 3, column 4 of each), NOTE 3's merged row 1,
/// and the cells where the three tables differ because each purpose has its own `QcType`.
#[test]
fn the_tables_are_the_clauses() {
    use Indication::*;
    // EXAMPLE 2, 3 and 4.
    assert_eq!(Purpose::ESig.cell(3, 4), Some(QcForESig));
    assert_eq!(Purpose::ESeal.cell(3, 4), Some(QcForESeal));
    assert_eq!(Purpose::Wsa.cell(3, 4), Some(Qwac));
    // Row 1, column 1: QcCompliance with no type or with type 1 is a signature certificate.
    assert_eq!(Purpose::ESig.cell(1, 1), Some(QcForESig));
    assert_eq!(Purpose::ESeal.cell(1, 1), Some(NotQualifiedForESeal));
    assert_eq!(Purpose::Wsa.cell(1, 1), Some(NotQwac));
    // Each purpose's own type, alone, under QcCompliance.
    assert_eq!(Purpose::ESeal.cell(2, 1), Some(QcForESeal));
    assert_eq!(Purpose::Wsa.cell(3, 1), Some(Qwac));
    assert_eq!(Purpose::ESig.cell(2, 1), Some(NotQualifiedForESig));
    // Its type beside another: indeterminate; another pair: not qualified.
    assert_eq!(Purpose::ESig.cell(4, 1), Some(IndetQcForESig));
    assert_eq!(Purpose::ESeal.cell(5, 1), Some(NotQualifiedForESeal));
    assert_eq!(Purpose::ESeal.cell(6, 1), Some(IndetQcForESeal));
    assert_eq!(Purpose::Wsa.cell(4, 1), Some(NotQwac));
    // Row 8, column 3: neither compliance nor type, only QCStatement — not enough to tell.
    for purpose in [Purpose::ESig, Purpose::ESeal, Purpose::Wsa] {
        assert_eq!(purpose.cell(8, 3), Some(purpose.indeterminate()));
        for row in 1..=15 {
            // Column 2, NotQualified, and column 5, QCStatement with the purpose's qualifier,
            // are the same down every table.
            assert_eq!(purpose.cell(row, 2), Some(purpose.not_qualified()));
            assert_eq!(purpose.cell(row, 5), Some(purpose.qualified()));
        }
        assert_eq!(purpose.cell(0, 1), None);
        assert_eq!(purpose.cell(16, 1), None);
    }
    // Row 9, column 3: no compliance, type 1, QCStatement.
    assert_eq!(Purpose::ESig.cell(9, 3), Some(QcForESig));
    assert_eq!(Purpose::ESeal.cell(10, 3), Some(QcForESeal));
    assert_eq!(Purpose::Wsa.cell(11, 3), Some(Qwac));
}

/// The row is the statements' (NOTE 3: `QcCompliance` alone is row 1 with type 1), the column the
/// qualifiers' (NOTE 3: none of the three is column 1, `NotQualified` column 2 with or without the
/// purpose's qualifier).
#[test]
fn row_and_column_are_chosen_as_the_notes_say() {
    use QcType::{Eseal, Esign, Web};
    assert_eq!(row(true, &[]), 1);
    assert_eq!(row(true, &[Esign]), 1);
    assert_eq!(row(true, &[Eseal]), 2);
    assert_eq!(row(true, &[Web]), 3);
    assert_eq!(row(true, &[Esign, Eseal]), 4);
    assert_eq!(row(true, &[Web, Esign]), 5);
    assert_eq!(row(true, &[Eseal, Web]), 6);
    assert_eq!(row(true, &[Esign, Eseal, Web]), 7);
    assert_eq!(row(false, &[]), 8);
    assert_eq!(row(false, &[Esign]), 9);
    assert_eq!(row(false, &[Web, Eseal]), 14);
    assert_eq!(row(false, &[Esign, Eseal, Web]), 15);
    assert_eq!(column(false, false, false), 1);
    assert_eq!(column(true, false, false), 2);
    assert_eq!(column(true, false, true), 2);
    assert_eq!(column(false, true, false), 3);
    assert_eq!(column(false, false, true), 4);
    assert_eq!(column(false, true, true), 5);
}

/// Table 4: two purposes qualified at once is an error, an indeterminate check beside anything a
/// warning, and the rest ok. Table 5: rows 1 to 4 alike, row 5 none of the three.
#[test]
fn tables_4_and_5_are_the_clauses() {
    use Indication::*;
    assert_eq!(combination(QcForESig, QcForESeal), Combined::Error);
    assert_eq!(combination(Qwac, QcForESig), Combined::Error);
    assert_eq!(combination(QcForESig, NotQualifiedForESeal), Combined::Ok);
    assert_eq!(combination(NotQualifiedForESig, Qwac), Combined::Ok);
    assert_eq!(
        combination(IndetQcForESig, NotQualifiedForESeal),
        Combined::Warning
    );
    assert_eq!(combination(QcForESeal, IndetQwac), Combined::Warning);
    assert_eq!(table_5(true, false, false), QcForESig);
    assert_eq!(table_5(true, true, false), NotQualifiedForESig);
    assert_eq!(table_5(false, false, false), NotQualifiedForESig);
    assert_eq!(table_5(false, false, true), QcForESig);
}

/// TS 119 612 clause 5.5.9.2: the extension read as Annex C's schema writes it — its criticality,
/// the qualifiers, the indicator, a key usage bit, a policy set as an `urn:oid:` URN, clause
/// 5.5.9.2.2.3's extended key usage, and a criterion the clause does not define kept by name.
#[test]
fn a_qualifications_extension_reads_as_the_schema_writes_it() {
    let ca = hex(CA);
    let criteria = format!(
        "{NON_REPUDIATION}<sie:PolicySet><sie:PolicyIdentifier>\
         <xades:Identifier xmlns:xades=\"http://uri.etsi.org/01903/v1.3.2#\" Qualifier=\"OIDAsURN\">\
         urn:oid:0.4.0.194112.1.2</xades:Identifier></sie:PolicyIdentifier></sie:PolicySet>\
         <sie:Description>Seals</sie:Description>\
         <sie:otherCriteriaList><tslx:ExtendedKeyUsage xmlns:tslx=\"{}\"><tslx:KeyPurposeId>\
         <xades:Identifier xmlns:xades=\"http://uri.etsi.org/01903/v1.3.2#\">1.3.6.1.5.5.7.3.4\
         </xades:Identifier></tslx:KeyPurposeId></tslx:ExtendedKeyUsage>\
         <ex:Mine xmlns:ex=\"urn:example\"/></sie:otherCriteriaList>",
        crate::trusted_list::ADDITIONAL_TYPES
    );
    let read = list(
        "Example Trust Provider",
        &[service(&ca)],
        &qualifications(true, &[QC_STATEMENT, QC_FOR_ESIG], "all", &criteria),
    );
    let state = &read.providers[0].services[0].current;
    let [qualifications] = state.qualifications.as_slice() else {
        panic!("one extension: {:?}", state.qualifications);
    };
    assert!(qualifications.critical);
    let [element] = qualifications.elements.as_slice() else {
        panic!("one element");
    };
    assert_eq!(element.qualifiers, [QC_STATEMENT, QC_FOR_ESIG]);
    assert_eq!(
        element.criteria,
        CriteriaList {
            assert: Assert::All,
            assertions: vec![
                Assertion::KeyUsage(vec![(Some(1), Some(true))]),
                Assertion::PolicySet(vec![Some("0.4.0.194112.1.2".to_owned())]),
                Assertion::ExtendedKeyUsage(vec![Some("1.3.6.1.5.5.7.3.4".to_owned())]),
                Assertion::Unrecognised("{urn:example}Mine".to_owned()),
            ],
            description: Some("Seals".to_owned()),
        }
    );
    // Clause 5.4.1: the provider's name, every language, for PRO-4.4.4-06.
    assert_eq!(read.providers[0].names, ["Example Trust Provider"]);
}

/// Clause 5.5.9.2.2.0's three indicators and clauses 5.5.9.2.2.1 to 5.5.9.2.2.3's assertions over
/// the signer [`ESIG`] (`keyUsage` `nonRepudiation` only, no policies, no extended key usage):
/// an assertion this reader cannot evaluate decides only where the answer turns on it.
#[test]
fn criteria_identify_by_their_indicator() {
    use Match::{No, Undecided, Yes};
    let der = hex(ESIG);
    let examined = Examined::of(&x509::parse(&der).expect("parses"));
    assert_eq!(examined.policies, Field::Absent);
    let list = |assert: Assert, assertions: Vec<Assertion>| CriteriaList {
        assert,
        assertions,
        description: None,
    };
    let non_repudiation = Assertion::KeyUsage(vec![(Some(1), Some(true))]);
    let digital_signature = Assertion::KeyUsage(vec![(Some(0), Some(true))]);
    let not_digital_signature = Assertion::KeyUsage(vec![(Some(0), Some(false))]);
    let policy = Assertion::PolicySet(vec![Some("0.4.0.194112.1.2".to_owned())]);
    let unknown = Assertion::Unrecognised("{urn:example}Mine".to_owned());
    let subject_country = Assertion::CertSubjectDnAttribute(vec![Some("2.5.4.6".to_owned())]);
    let subject_serial = Assertion::CertSubjectDnAttribute(vec![Some("2.5.4.5".to_owned())]);
    let cases = [
        (
            Assert::All,
            vec![non_repudiation.clone(), not_digital_signature],
            Yes,
        ),
        (
            Assert::All,
            vec![non_repudiation.clone(), digital_signature.clone()],
            No,
        ),
        (
            Assert::AtLeastOne,
            vec![digital_signature.clone(), non_repudiation.clone()],
            Yes,
        ),
        // Clause 5.5.9.2.2.2: an absent extension does not hold the identifier.
        (
            Assert::AtLeastOne,
            vec![digital_signature.clone(), policy.clone()],
            No,
        ),
        (Assert::None, vec![digital_signature.clone(), policy], Yes),
        (Assert::None, vec![non_repudiation.clone()], No),
        (
            Assert::All,
            vec![subject_country, non_repudiation.clone()],
            Yes,
        ),
        (Assert::AtLeastOne, vec![subject_serial], No),
        (
            Assert::AtLeastOne,
            vec![unknown.clone(), non_repudiation.clone()],
            Yes,
        ),
        (
            Assert::AtLeastOne,
            vec![unknown.clone(), digital_signature.clone()],
            Undecided,
        ),
        (Assert::All, vec![unknown.clone(), digital_signature], No),
        (
            Assert::All,
            vec![unknown.clone(), non_repudiation.clone()],
            Undecided,
        ),
        (
            Assert::Unrecognised("some".to_owned()),
            vec![non_repudiation.clone()],
            Undecided,
        ),
        (
            Assert::All,
            vec![
                Assertion::CriteriaList(list(Assert::None, vec![unknown])),
                non_repudiation,
            ],
            Undecided,
        ),
    ];
    for (at, (assert, assertions, expected)) in cases.into_iter().enumerate() {
        assert_eq!(
            list(assert, assertions).identifies(&examined),
            expected,
            "case {at}"
        );
    }
}

/// PRO-4.4.4-15 with no qualifications extension: [`ESIG`] states `QcCompliance` and `QcType`
/// `esign` (Table 1 row 1, column 1: `QC_For_eSig`); the service is not for seals or web sites
/// (PRO-4.4.4-18 and -26: the not-qualified values); Table 4 has nothing to say and the run at
/// `notBefore` agrees (PRO-4.4.4-36).
#[test]
fn a_qualified_signers_certificate_is_confirmed_for_signatures() {
    let ca = hex(CA);
    let lists = [list("Example Trust Provider", &[service(&ca)], "")];
    let determination = outputs(determined(ESIG, &lists));
    assert_eq!(determination.status, QcStatus::Passed);
    assert_eq!(determination.sub_status, []);
    assert_eq!(
        determination.results,
        [
            Indication::QcForESig,
            Indication::NotQualifiedForESeal,
            Indication::NotQwac
        ]
    );
}

/// The qualifiers of an element whose criteria identify the certificate change the column: the
/// plain signer [`PLAIN`] claims nothing (row 8), so with no qualifier it is column 1,
/// `Not_Qualified_For_eSig`; `QCStatement` with `QCForESig` over `nonRepudiation` is column 5,
/// `QC_For_eSig`; the same element under `none` identifies nobody with that bit, and it is column
/// 1 again.
#[test]
fn the_qualifiers_of_identifying_criteria_choose_the_column() {
    let ca = hex(CA);
    let results = |extension: &str| {
        outputs(determined(
            PLAIN,
            &[list("Example Trust Provider", &[service(&ca)], extension)],
        ))
        .results
    };
    assert_eq!(results("").first(), Some(&Indication::NotQualifiedForESig));
    let applies = qualifications(
        false,
        &[QC_STATEMENT, QC_FOR_ESIG],
        "atLeastOne",
        NON_REPUDIATION,
    );
    assert_eq!(results(&applies).first(), Some(&Indication::QcForESig));
    let excludes = qualifications(false, &[QC_STATEMENT, QC_FOR_ESIG], "none", NON_REPUDIATION);
    assert_eq!(
        results(&excludes).first(),
        Some(&Indication::NotQualifiedForESig)
    );
    // Row 8, column 3 (QCStatement alone): indeterminate, and PRO-4.4.4-15 d)'s warning.
    let statement_only = qualifications(false, &[QC_STATEMENT], "all", NON_REPUDIATION);
    let determination = outputs(determined(
        PLAIN,
        &[list(
            "Example Trust Provider",
            &[service(&ca)],
            &statement_only,
        )],
    ));
    assert_eq!(
        determination.results.first(),
        Some(&Indication::IndetQcForESig)
    );
    assert!(
        determination
            .sub_status
            .contains(&SubStatus::NotEnoughInfoOnQcType(Purpose::ESig))
    );
    // Table 4: an indeterminate check beside the others is a warning.
    assert_eq!(determination.status, QcStatus::PassedWithWarning);
}

/// PRO-4.4.4-12 a) 7): `QCForLegalPerson` makes the signature branch indeterminate with its
/// warning, whatever the certificate claims.
#[test]
fn a_legal_person_qualifier_is_inconsistent_for_signatures() {
    let ca = hex(CA);
    let extension = qualifications(false, &[QC_FOR_LEGAL_PERSON], "all", NON_REPUDIATION);
    let determination = outputs(determined(
        ESIG,
        &[list("Example Trust Provider", &[service(&ca)], &extension)],
    ));
    assert_eq!(
        determination.results.first(),
        Some(&Indication::IndetQcForESig)
    );
    assert!(
        determination
            .sub_status
            .contains(&SubStatus::InconsistentQualifiers(Purpose::ESig))
    );
}

/// TS 119 612 clause 5.5.9.2.0: criteria this reader cannot evaluate under a critical extension
/// discard the certificate — INDETERMINATE, failed; under a non-critical one the element is set
/// aside, said as a warning, and the answer is the certificate's own (Table 1 row 1, column 1).
#[test]
fn undecidable_criteria_discard_the_certificate_only_when_critical() {
    let ca = hex(CA);
    let unknown =
        "<sie:otherCriteriaList><ex:Mine xmlns:ex=\"urn:example\"/></sie:otherCriteriaList>";
    let run = |critical: bool| {
        let extension = qualifications(critical, &[NOT_QUALIFIED], "all", unknown);
        outputs(determined(
            ESIG,
            &[list("Example Trust Provider", &[service(&ca)], &extension)],
        ))
    };
    let critical = run(true);
    assert_eq!(critical.status, QcStatus::Failed);
    assert_eq!(critical.results, [Indication::Indeterminate]);
    assert_eq!(critical.sub_status, [SubStatus::CriticalCriteriaUndecided]);
    let set_aside = run(false);
    assert_eq!(set_aside.results.first(), Some(&Indication::QcForESig));
    assert_eq!(set_aside.status, QcStatus::PassedWithWarning);
    assert!(set_aside.sub_status.contains(&SubStatus::CriteriaUndecided));
}

/// PRO-4.4.4-06 a): the issuer's `organizationName` is not one of the provider's names, so the
/// process fails with `ERROR_TSP_NAME_INCONSISTENCY_BETWEEN_CERT_AND_TL` and INDETERMINATE; RFC
/// 5280 section 7.1's case folding still matches a name that differs only in case.
#[test]
fn an_issuer_that_is_not_the_listed_provider_is_refused() {
    let ca = hex(CA);
    let refused = outputs(determined(
        ESIG,
        &[list("Another Provider", &[service(&ca)], "")],
    ));
    assert_eq!(refused.status, QcStatus::Failed);
    assert_eq!(refused.results, [Indication::Indeterminate]);
    assert_eq!(refused.sub_status, [SubStatus::TspNameInconsistency]);
    let folded = outputs(determined(
        ESIG,
        &[list("EXAMPLE  trust provider", &[service(&ca)], "")],
    ));
    assert_eq!(folded.status, QcStatus::Passed);
}

/// PRO-4.4.4-05: a list of the issuer's territory naming no service that issued the certificate
/// is `Not_Qualified` with `No_confirmation_found_in_EUMSTL_CC`; with no list of that territory
/// at all the procedure has nothing to read, and says so.
#[test]
fn no_service_and_no_list_are_different_answers() {
    let other = hex(PLAIN);
    let unrelated = outputs(determined(
        ESIG,
        &[list("Example Trust Provider", &[service(&other)], "")],
    ));
    assert_eq!(unrelated.status, QcStatus::Passed);
    assert_eq!(unrelated.results, [Indication::NotQualified]);
    assert_eq!(unrelated.sub_status, [SubStatus::NoConfirmationFound]);
    let ca = hex(CA);
    let xml = list_xml("DE", &[("Example Trust Provider", &[service(&ca)])]);
    let german = TrustedList::read(xml.as_bytes()).expect("reads");
    assert_eq!(
        determined(ESIG, &[german]),
        Qualification::NoListForTerritory("AT".to_owned())
    );
}

/// PRO-4.4.4-36 a): the service was withdrawn when [`ESIG`] was issued and granted again later, so
/// the run at `notBefore` says `Not_Qualified_For_eSig` (PRO-4.4.4-10) where the run at the best
/// signature time says `QC_For_eSig`: the sets differ and the process fails.
#[test]
fn a_different_answer_at_issue_fails_the_process() {
    let ca = hex(CA);
    let regranted = Written {
        since: "2025-06-01T00:00:00Z",
        history: &[(WITHDRAWN, "2024-06-01T00:00:00Z")],
        ..service(&ca)
    };
    let determination = outputs(determined(
        ESIG,
        &[list("Example Trust Provider", &[regranted], "")],
    ));
    assert_eq!(determination.status, QcStatus::Failed);
    assert_eq!(
        determination.sub_status,
        [SubStatus::ResultsDifferAtIssue(vec![
            Indication::NotQualifiedForESig,
            Indication::NotQualifiedForESeal,
            Indication::NotQwac
        ])]
    );
}

/// PRO-4.4.4-07 and -33: [`OLD`] was issued in 2015, under the Directive. Table 5 row 3 (the QCP+
/// policy), column 1: `QC_For_eSig`; seals and web sites are not qualified (a) and b)). A
/// `NotQualified` qualifier is column 2: `Not_Qualified_For_eSig`. The run at 2015-01-01 reads the
/// service's history instance under supervision, which f) 1) does not end, and agrees.
#[test]
fn a_certificate_under_the_directive_is_decided_by_table_5() {
    let ca = hex(CA);
    let determination = outputs(determined(
        OLD,
        &[list("Example Trust Provider", &[service(&ca)], "")],
    ));
    assert_eq!(determination.status, QcStatus::Passed);
    assert_eq!(determination.sub_status, []);
    assert_eq!(
        determination.results,
        [
            Indication::QcForESig,
            Indication::NotQualifiedForESeal,
            Indication::NotQwac
        ]
    );
    let extension = qualifications(false, &[NOT_QUALIFIED], "all", NON_REPUDIATION);
    let not = outputs(determined(
        OLD,
        &[list("Example Trust Provider", &[service(&ca)], &extension)],
    ));
    assert_eq!(not.results.first(), Some(&Indication::NotQualifiedForESig));
}

/// PRO-4.4.4-10 and -18: each branch reads only the services whose `additionalServiceInformation`
/// names its purpose. A service for seals alone says nothing of signatures (`Not_Qualified_For_eSig`),
/// and [`ESIG`]'s `QcType` `esign` is Table 2 row 1, column 1: `Not_Qualified_For_eSeal`.
#[test]
fn each_branch_reads_the_services_for_its_purpose() {
    let ca = hex(CA);
    let for_seals = Written {
        for_type: Some(FOR_TYPES[1]),
        ..service(&ca)
    };
    let determination = outputs(determined(
        ESIG,
        &[list("Example Trust Provider", &[for_seals], "")],
    ));
    assert_eq!(
        determination.results,
        [
            Indication::NotQualifiedForESig,
            Indication::NotQualifiedForESeal,
            Indication::NotQwac
        ]
    );
}

/// Clause 4.5 for `der` over Austria's list naming the CA with `extension`, at `at`.
fn qscd_of(der: &str, provider: &str, extension: &str, at: Instant) -> qscd::Qscd {
    let der = hex(der);
    let certificate = x509::parse(&der).expect("the certificate parses");
    let ca = hex(CA);
    let pool = [x509::parse(&ca).expect("the CA parses")];
    match determine(
        &[list(provider, &[service(&ca)], extension)],
        &certificate,
        &pool,
        at,
    ) {
        Qualification::Determined { qscd, .. } => qscd,
        other => panic!("the procedure should have run: {other:?}"),
    }
}

/// Tables 6 and 7 as printed: the list's "with" column is `QSCD_YES` and its "no" column `QSCD_NO`
/// on both rows; between them (the "as in the certificate" qualifier, or none) the certificate's
/// `QcSSCD` statement — and under the Directive the QCP+ policy — decides.
#[test]
fn tables_6_and_7_are_the_clauses() {
    use qscd::{QscdIndication::*, table_6, table_7};
    for table in [table_6, table_7] {
        assert_eq!(table(true, 1), Yes);
        assert_eq!(table(false, 1), Yes);
        assert_eq!(table(true, 2), Yes);
        assert_eq!(table(false, 2), No);
        assert_eq!(table(true, 3), No);
        assert_eq!(table(false, 3), No);
    }
}

/// PRO-4.5.4-04 over [`ESIG`], confirmed `QC_For_eSig` and stating no `QcSSCD`: no QSCD qualifier
/// is Table 7's second row, second column, `QSCD_NO`; `QCWithQSCD` or `QCQSCDManagedOnBehalf` is
/// the first column, `QSCD_YES`; c) 5) and c) 1)'s combinations are `QSCD_INDETERMINATE` with their
/// warning.
#[test]
fn the_regulations_qualifiers_choose_table_7s_column() {
    use qscd::{QC_QSCD_MANAGED_ON_BEHALF, QC_QSCD_STATUS_AS_IN_CERT, QC_WITH_QSCD, QC_WITH_SSCD};
    use qscd::{QscdIndication, QscdSubStatus};
    let with = |qualifiers: &[&str]| {
        qscd_of(
            ESIG,
            "Example Trust Provider",
            &qualifications(false, qualifiers, "atLeastOne", NON_REPUDIATION),
            AT,
        )
    };
    let none = qscd_of(ESIG, "Example Trust Provider", "", AT);
    assert_eq!(none.result, Some(QscdIndication::No));
    assert_eq!(none.status, QcStatus::Passed);
    assert_eq!(with(&[QC_WITH_QSCD]).result, Some(QscdIndication::Yes));
    assert_eq!(
        with(&[QC_QSCD_MANAGED_ON_BEHALF]).result,
        Some(QscdIndication::Yes)
    );
    for inconsistent in [
        &[QC_QSCD_STATUS_AS_IN_CERT, QC_WITH_QSCD][..],
        &[QC_WITH_SSCD],
    ] {
        let answer = with(inconsistent);
        assert_eq!(
            answer.result,
            Some(QscdIndication::Indeterminate),
            "{inconsistent:?}"
        );
        assert_eq!(answer.status, QcStatus::PassedWithWarning);
        assert_eq!(answer.sub_status, [QscdSubStatus::InconsistentQscd]);
    }
}

/// PRO-4.5.4-04 b): a qualifier TS 119 612 clause 5.5.9.2.3 does not define fails the process
/// under a critical extension and is a warning under another, whose answer is then the table's.
#[test]
fn an_unknown_qualifier_fails_only_when_critical() {
    use qscd::{QscdIndication, QscdSubStatus};
    let run = |critical: bool| {
        qscd_of(
            ESIG,
            "Example Trust Provider",
            &qualifications(critical, &["urn:example:qualifier"], "all", NON_REPUDIATION),
            AT,
        )
    };
    let critical = run(true);
    assert_eq!(critical.status, QcStatus::Failed);
    assert_eq!(
        critical.sub_status,
        [QscdSubStatus::UnknownCriticalQualifiers]
    );
    assert_eq!(critical.result, None);
    let warned = run(false);
    assert_eq!(warned.sub_status, [QscdSubStatus::UnknownQualifiers]);
    assert_eq!(warned.result, Some(QscdIndication::No));
}

/// PRO-4.5.4-02 and -05: clause 4.4 failing fails clause 4.5 with its sub-status, and a
/// certificate confirmed for neither signatures nor seals ([`PLAIN`], `Not_Qualified_For_eSig`) is
/// `QSCD_INDETERMINATE`.
#[test]
fn qscd_follows_the_qualification() {
    use qscd::{QscdIndication, QscdSubStatus};
    let failed = qscd_of(ESIG, "Another Provider", "", AT);
    assert_eq!(failed.status, QcStatus::Failed);
    assert_eq!(
        failed.sub_status,
        [QscdSubStatus::QualificationFailed(vec![
            SubStatus::TspNameInconsistency
        ])]
    );
    let plain = qscd_of(PLAIN, "Example Trust Provider", "", AT);
    assert_eq!(plain.result, Some(QscdIndication::Indeterminate));
    assert_eq!(plain.status, QcStatus::Passed);
}

/// PRO-4.5.4-03 at 2015-06-01, strictly before the Regulation: [`OLD`] is `QC_For_eSig` by Table
/// 5, and Table 6's first row (QCP+ present), second column (no qualifier) is `QSCD_YES`;
/// `QCNoSSCD` is the third column, `QSCD_NO`; `QCWithSSCD` beside `QCNoSSCD` is a) 2) i).
#[test]
fn the_directives_qualifiers_choose_table_6s_column() {
    use qscd::{QC_NO_SSCD, QC_WITH_SSCD, QscdIndication, QscdSubStatus};
    let before = Instant::from_unix_seconds(1_433_116_800);
    let with = |qualifiers: &[&str]| {
        qscd_of(
            OLD,
            "Example Trust Provider",
            &qualifications(false, qualifiers, "all", NON_REPUDIATION),
            before,
        )
    };
    assert_eq!(
        qscd_of(OLD, "Example Trust Provider", "", before).result,
        Some(QscdIndication::Yes)
    );
    assert_eq!(with(&[QC_NO_SSCD]).result, Some(QscdIndication::No));
    let both = with(&[QC_WITH_SSCD, QC_NO_SSCD]);
    assert_eq!(both.result, Some(QscdIndication::Indeterminate));
    assert_eq!(both.sub_status, [QscdSubStatus::InconsistentSscd]);
}
