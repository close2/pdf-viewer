//! RFC 5755's attribute certificate read from fixtures signed for the purpose, and the profile's
//! violations refused from hand-built ones.

use super::*;
use crate::der::Reader;
use crate::x509::Instant;

/// Attribute certificates and their authority's hierarchy.
///
/// Built once with `openssl` (P-256 keys, `ecdsa-with-SHA256`) and a short script that writes each
/// `AttributeCertificateInfo` by RFC 5755 section 4.1 under the module's `IMPLICIT TAGS` and signs
/// it, then pasted in: **test vectors rather than oracles**, on the footing
/// [`crate::trust::tests::fixtures`] states — a valid attribute certificate needs an authority
/// whose private key somebody here held. Every certificate below is valid from 2026-01-01 to
/// 2027-01-01 and carries one `id-at-role` attribute; each holder names
/// [`crate::trust::tests::fixtures::SIGNER`] unless its name says otherwise.
pub(crate) mod fixtures {
    /// Issued by [`AA`], its holder the signer by `baseCertificateID`.
    pub(crate) const AC: &str = "\
            308201323081d90201013041a03f3027a42530233121301f06035504030c1871756f727261207465737420696e746572\
            6d65646961746502142098d48ac135f5264c4317afd4dd554de2d88312a030302ea42c302a3128302606035504030c1f\
            71756f72726120746573742061747472696275746520617574686f72697479300a06082a8648ce3d0403020201113022\
            180f32303236303130313030303030305a180f32303237303130313030303030305a302c302a060355044831233021a1\
            1f861d75726e3a71756f7272613a746573743a726f6c653a617070726f766572300a06082a8648ce3d04030203480030\
            45022100cefa3650aeaa32ed1bdfac8e2dc3a890d22e53e9d9bfe0daae3f087e7b0149ae0220692c71bec84eff6e2eca\
            2267fc13c64c4c09149edd85231fe98ed7de5f0a43ee";

    /// Issued by [`AA`], its holder serial number 1 of the signer's issuer: somebody else.
    pub(crate) const AC_OTHER_HOLDER: &str = "\
            308201203081c6020101302ea02c3027a42530233121301f06035504030c1871756f727261207465737420696e746572\
            6d656469617465020101a030302ea42c302a3128302606035504030c1f71756f72726120746573742061747472696275\
            746520617574686f72697479300a06082a8648ce3d0403020201123022180f32303236303130313030303030305a180f\
            32303237303130313030303030305a302c302a060355044831233021a11f861d75726e3a71756f7272613a746573743a\
            726f6c653a617070726f766572300a06082a8648ce3d0403020349003046022100b47f195500a851b92dc3464c3117bb\
            4c4e02a5e1d6164dfb776fbce93da0dcf8022100a621144c569be1c58ea2b3178c7e800ba23034698b4e7083cb2285a3\
            ba1bc13b";

    /// Issued by [`AA_CA`], whose certificate is a CA's.
    pub(crate) const AC_BY_A_CA: &str = "\
            3082013f3081e60201013041a03f3027a42530233121301f06035504030c1871756f727261207465737420696e746572\
            6d65646961746502142098d48ac135f5264c4317afd4dd554de2d88312a03d303ba43930373135303306035504030c2c\
            71756f72726120746573742061747472696275746520617574686f7269747920746861742069732061204341300a0608\
            2a8648ce3d0403020201133022180f32303236303130313030303030305a180f32303237303130313030303030305a30\
            2c302a060355044831233021a11f861d75726e3a71756f7272613a746573743a726f6c653a617070726f766572300a06\
            082a8648ce3d0403020348003045022100af6d2c2b1dc808ff71d8cd0d26456c67d5511f5da1d732b4fdefa4a7f501cf\
            d602207558c1a779f558cde37d1fd367a77f165b7ff9317ed31ffac1d048d8f7321c38";

    /// Issued by [`AA`], with a critical extension of the private arc `1.3.6.1.4.1.39857.1.1`.
    pub(crate) const AC_UNSUPPORTED_CRITICAL: &str = "\
            3082014a3081f00201013041a03f3027a42530233121301f06035504030c1871756f727261207465737420696e746572\
            6d65646961746502142098d48ac135f5264c4317afd4dd554de2d88312a030302ea42c302a3128302606035504030c1f\
            71756f72726120746573742061747472696275746520617574686f72697479300a06082a8648ce3d0403020201143022\
            180f32303236303130313030303030305a180f32303237303130313030303030305a302c302a060355044831233021a1\
            1f861d75726e3a71756f7272613a746573743a726f6c653a617070726f76657230153013060a2b0601040182b7310101\
            0101ff04020500300a06082a8648ce3d0403020349003046022100e9b6b43c39e6ec6e1747ea4832295074e996ee1748\
            a1ad29f47e4c8fb0789785022100d9ee7778a634de7592275744290dc18564e67d8fa2491ba1c505870c8069e1bf";

    /// Issued by [`AA`], its holder the signer by `entityName`, the signer's subject.
    pub(crate) const AC_BY_SUBJECT: &str = "\
            308201143081bb0201013023a121a41f301d311b301906035504030c1271756f7272612074657374207369676e6572a0\
            30302ea42c302a3128302606035504030c1f71756f72726120746573742061747472696275746520617574686f726974\
            79300a06082a8648ce3d0403020201153022180f32303236303130313030303030305a180f3230323730313031303030\
            3030305a302c302a060355044831233021a11f861d75726e3a71756f7272613a746573743a726f6c653a617070726f76\
            6572300a06082a8648ce3d040302034800304502200347361d4ad94f649ee0e53a16c35614334728c63967ed530a31b5\
            556674c39e0221009c8f1465191d4806aa642d1088582c058dab2ab62c8226bb25b808a545537572";

    /// The attribute authorities' self-signed root, 2026 to 2036.
    pub(crate) const AA_ROOT: &str = "\
            3082018f30820134a00302010202144657bfa2911beb06206d1cea915555ecb3286bb6300a06082a8648ce3d04030230\
            253123302106035504030c1a71756f72726120746573742061747472696275746520726f6f74301e170d323630313031\
            3030303030305a170d3336303130313030303030305a30253123302106035504030c1a71756f72726120746573742061\
            747472696275746520726f6f743059301306072a8648ce3d020106082a8648ce3d030107034200040e680f5732ab4aa0\
            02fb06cc382aea18d38a1f85b845332adb00a199c576b96581cb4fb818321b6968c94d706996ac39345bae4065c5beb5\
            2cdbb37c6c39515fa3423040300f0603551d130101ff040530030101ff300e0603551d0f0101ff040403020106301d06\
            03551d0e04160414b0d5dc4be2edffc109f56863ecf2c4dc456b8717300a06082a8648ce3d0403020349003046022100\
            feced912ab3993215bc893d8941407be08f9478120748652c275cf6f0895985a022100db8720c74be0bdcecdef67786b\
            1573a933e9cc980305f1101c5e15adbef14f94";

    /// An attribute authority issued by [`AA_ROOT`]: no basic constraints, `digitalSignature` only.
    pub(crate) const AA: &str = "\
            308201983082013ea003020102020900a00bbd3db5e090b6300a06082a8648ce3d04030230253123302106035504030c\
            1a71756f72726120746573742061747472696275746520726f6f74301e170d3236303130313030303030305a170d3330\
            303130313030303030305a302a3128302606035504030c1f71756f727261207465737420617474726962757465206175\
            74686f726974793059301306072a8648ce3d020106082a8648ce3d03010703420004ba1f7a7f9d5bae3a0d446e958046\
            60f48eb953f5352d0ca973ca6dad75c7e5abb3890f340eb9f93ce6feb857c0b1174b674d970aebb4f81877baf6e5d4ab\
            77baa3523050300e0603551d0f0101ff040403020780301d0603551d0e0416041467797203d0dc7847365c7b177e2e03\
            7fccd03348301f0603551d23041830168014b0d5dc4be2edffc109f56863ecf2c4dc456b8717300a06082a8648ce3d04\
            03020348003045022100c55702a5ec9a4d09d251ada05b98ec79f12a3fbc7c955aab3ceb46f6ff0e7bfb022028d6ff0f\
            57f61cfe19607d4ff86fdd0aad63dd752562a3f8d8898f9663e3f7a1";

    /// An attribute authority issued by [`AA_ROOT`] whose certificate asserts `cA`.
    pub(crate) const AA_CA: &str = "\
            308201b53082015ba00302010202081f215794f77f43c0300a06082a8648ce3d04030230253123302106035504030c1a\
            71756f72726120746573742061747472696275746520726f6f74301e170d3236303130313030303030305a170d333030\
            3130313030303030305a30373135303306035504030c2c71756f72726120746573742061747472696275746520617574\
            686f72697479207468617420697320612043413059301306072a8648ce3d020106082a8648ce3d03010703420004c64e\
            8ee560e7894660fb48aa390885517e0c72481b5d421f3d83afd4ffe940fca18be3e6aaacc176a92f9012c14b218fce8e\
            681635863de483bac4a696d61324a3633061300f0603551d130101ff040530030101ff300e0603551d0f0101ff040403\
            020284301d0603551d0e041604143202ca9020cde3d925b7fe25be4f8ad1bb4e37d0301f0603551d23041830168014b0\
            d5dc4be2edffc109f56863ecf2c4dc456b8717300a06082a8648ce3d0403020348003045022100b018725a9c9bea97ac\
            322e3880df21969845943b8d633374cc6c3f6fd1c837190220504ced70ff8bb6c49e5d0ddfe60d2b9b9029741a4adca5\
            4d93b5b4f0e0f96b3d";
}

/// A fixture's hexadecimal, whitespace ignored, as octets.
pub(crate) fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("ASCII digits");
            u8::from_str_radix(text, 16).expect("a hexadecimal pair")
        })
        .collect()
}

/// One DER value, its length in the short form or in two octets.
fn tlv(tag: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    match u8::try_from(contents.len()) {
        Ok(length) if length < 0x80 => out.push(length),
        _ => {
            let length = u16::try_from(contents.len()).expect("a hand-built value");
            out.push(0x82);
            out.extend_from_slice(&length.to_be_bytes());
        }
    }
    out.extend_from_slice(contents);
    out
}

fn seq(parts: &[&[u8]]) -> Vec<u8> {
    tlv(SEQUENCE, &parts.concat())
}

fn read_from(bytes: &[u8]) -> Result<AttributeCertificate<'_>, Unreadable> {
    let value = Reader::new(bytes)
        .expect("a fixture")
        .next_value()
        .expect("well formed")
        .expect("one value");
    read(value)
}

/// `id-at-role`, `2.5.4.72`.
const ROLE: &[u8] = &[0x55, 0x04, 0x48];
/// `id-at-title`, `2.5.4.12`.
const TITLE: &[u8] = &[0x55, 0x04, 0x0C];

/// A `directoryName` holding `CN=<common>`: `[4]` wraps the `Name`, a `CHOICE` (X.680 clause
/// 31.2.7 makes its tag explicit even in an `IMPLICIT TAGS` module).
fn directory_name(common: &[u8]) -> Vec<u8> {
    let rdn = tlv(
        SET,
        &seq(&[
            &tlv(OBJECT_IDENTIFIER, &[0x55, 0x04, 0x03]),
            &tlv(0x0C, common),
        ]),
    );
    tlv(0xA4, &seq(&[&rdn]))
}

/// `Attribute ::= SEQUENCE { type, values SET OF }` with one `UTF8String` value.
fn attribute(kind: &[u8]) -> Vec<u8> {
    seq(&[&tlv(OBJECT_IDENTIFIER, kind), &tlv(SET, &tlv(0x0C, b"x"))])
}

/// The members of a hand-built `AttributeCertificateInfo`, each whole.
struct Info {
    version: Vec<u8>,
    holder: Vec<u8>,
    issuer: Vec<u8>,
    attributes: Vec<u8>,
}

impl Info {
    /// Section 4.1's syntax under section 4.2's profile: version v2, the holder an `[0]
    /// IssuerSerial` (implicitly tagged), the issuer a `v2Form` holding one `directoryName`, and
    /// one role.
    fn conforming() -> Self {
        Self {
            version: tlv(0x02, &[1]),
            holder: seq(&[&tlv(
                0xA0,
                &[seq(&[&directory_name(b"issuer")]), tlv(0x02, &[7])].concat(),
            )]),
            issuer: tlv(0xA0, &seq(&[&directory_name(b"authority")])),
            attributes: seq(&[&attribute(ROLE)]),
        }
    }

    /// The certificate, its signature an empty placeholder: the reader reads and never verifies.
    fn encode(&self) -> Vec<u8> {
        let algorithm = seq(&[&tlv(
            OBJECT_IDENTIFIER,
            &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02],
        )]);
        let period = seq(&[
            &tlv(0x18, b"20260101000000Z"),
            &tlv(0x18, b"20270101000000Z"),
        ]);
        let info = seq(&[
            &self.version,
            &self.holder,
            &self.issuer,
            &algorithm,
            &tlv(0x02, &[9]),
            &period,
            &self.attributes,
        ]);
        seq(&[&info, &algorithm, &tlv(0x03, &[0])])
    }
}

#[test]
fn a_signed_attribute_certificate_is_read_by_section_4_1() {
    // RFC 5755 section 4.1: the holder's `baseCertificateID` is the issuer and serial number of
    // the holder's certificate, the fixture signer's; the issuer is the authority's name; the
    // validity is the two `GeneralizedTime`s the fixture states; one `id-at-role` attribute.
    let signer_bytes = hex(crate::trust::tests::fixtures::SIGNER);
    let signer = x509::parse(&signer_bytes).expect("the fixture signer");
    let authority_bytes = hex(fixtures::AA);
    let authority = x509::parse(&authority_bytes).expect("the fixture authority");
    let bytes = hex(fixtures::AC);
    let read = read_from(&bytes).expect("a conforming attribute certificate");
    let base = read.holder.base.expect("a baseCertificateID");
    assert_eq!(base.issuer, Some(signer.issuer));
    assert!(signer.is_named_by(signer.issuer, base.serial));
    assert!(read.holder.entity.is_empty() && !read.holder.digest);
    assert_eq!(read.issuer, authority.subject);
    assert_eq!(read.types, vec![ROLE]);
    assert_eq!(
        read.validity.not_before,
        Instant::from_unix_seconds(1_767_225_600)
    );
    assert_eq!(
        read.validity.not_after,
        Instant::from_unix_seconds(1_798_761_600)
    );
    assert!(!read.targeted && read.unsupported_critical.is_none() && !read.indefinite);
    // The signature covers `acinfo`'s whole encoding, the first member of the outer `SEQUENCE`.
    assert_eq!(read.info.first(), Some(&SEQUENCE));
    assert!(
        bytes
            .windows(read.info.len())
            .any(|window| window == read.info)
    );

    let bytes = hex(fixtures::AC_BY_SUBJECT);
    let read = read_from(&bytes).expect("a conforming attribute certificate");
    assert!(read.holder.base.is_none());
    assert_eq!(read.holder.entity.len(), 1);
    assert_eq!(read.holder.entity[0].identifier, 0xA4);

    // Section 4.2.9: a critical extension this reader does not support is reported by identifier.
    let bytes = hex(fixtures::AC_UNSUPPORTED_CRITICAL);
    let read = read_from(&bytes).expect("a conforming attribute certificate");
    assert_eq!(
        read.unsupported_critical.and_then(x509::dotted).as_deref(),
        Some("1.3.6.1.4.1.39857.1.1")
    );
}

#[test]
fn the_profiles_violations_are_refused_by_member() {
    assert!(read_from(&Info::conforming().encode()).is_ok());
    let refused = |info: Info| read_from(&info.encode()).map(|_| ()).unwrap_err();
    // Section 4.2.1: the version is v2, encoded 1.
    let mut info = Info::conforming();
    info.version = tlv(0x02, &[0]);
    assert_eq!(refused(info), Unreadable::Malformed("version"));
    // Section 4.2.3: `v1Form`, a bare `GeneralNames`, is not the profile's issuer.
    let mut info = Info::conforming();
    info.issuer = seq(&[&directory_name(b"authority")]);
    assert_eq!(refused(info), Unreadable::Malformed("issuer"));
    // Section 4.2.3: one name, not two.
    let mut info = Info::conforming();
    info.issuer = tlv(
        0xA0,
        &seq(&[&directory_name(b"authority"), &directory_name(b"other")]),
    );
    assert_eq!(refused(info), Unreadable::Malformed("issuer"));
    // Section 4.2.7: at least one attribute, and each type once.
    let mut info = Info::conforming();
    info.attributes = seq(&[]);
    assert_eq!(refused(info), Unreadable::Malformed("attributes"));
    let mut info = Info::conforming();
    info.attributes = seq(&[&attribute(ROLE), &attribute(TITLE), &attribute(ROLE)]);
    assert_eq!(refused(info), Unreadable::Malformed("attributes"));
    // The module is `IMPLICIT TAGS`: a `baseCertificateID` whose `[0]` wraps an `IssuerSerial`
    // `SEQUENCE`, as an explicit tag would, is not the syntax.
    let mut info = Info::conforming();
    info.holder = seq(&[&tlv(
        0xA0,
        &seq(&[&seq(&[&directory_name(b"issuer")]), &tlv(0x02, &[7])]),
    )]);
    assert_eq!(refused(info), Unreadable::Malformed("holder"));
}

#[test]
fn a_list_past_the_bound_is_refused_rather_than_read() {
    // `MAX_ATTRIBUTES` distinct types read; one more is refused as past the bound.
    let types: Vec<Vec<u8>> = (0..=MAX_ATTRIBUTES)
        .map(|index| vec![0x2A, u8::try_from(index).expect("under 128")])
        .collect();
    let attributes = |count: usize| -> Vec<u8> {
        let list: Vec<u8> = types[..count]
            .iter()
            .flat_map(|kind| attribute(kind))
            .collect();
        tlv(SEQUENCE, &list)
    };
    let within = Info {
        attributes: attributes(MAX_ATTRIBUTES),
        ..Info::conforming()
    };
    let read = read_info_long(&within).expect("at the bound");
    assert_eq!(read, MAX_ATTRIBUTES);
    let past = Info {
        attributes: attributes(MAX_ATTRIBUTES + 1),
        ..Info::conforming()
    };
    assert_eq!(
        read_info_long(&past),
        Err(Unreadable::TooMany("attributes"))
    );
}

/// [`read_types`] over a hand-built attribute list.
fn read_info_long(info: &Info) -> Result<usize, Unreadable> {
    let value = Reader::new(&info.attributes)
        .expect("a list")
        .next_value()
        .expect("well formed")
        .expect("one value");
    read_types(&value).map(|types| types.len())
}
