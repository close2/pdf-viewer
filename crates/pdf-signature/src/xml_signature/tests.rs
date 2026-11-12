//! XML Signature's core validation, one requirement at a time, over lists signed here.
//!
//! **Every expected value is derived by hand from the Recommendations, not computed by the code
//! under test.** Each fixture is written already in its exclusive canonical form (Exclusive XML
//! Canonicalization 1.0 section 3: the root declares the one prefix it uses, no element is empty
//! in the short form, attributes are in order), so the octets a reference digests are the
//! fixture's own text with the `ds:Signature` element cut out (XML Signature 1.1 section 6.6.4),
//! and `ds:SignedInfo`'s canonical form is its text with the `ds` declaration it inherits written
//! on it. The digest and the signature are computed over those literal strings; the
//! canonicaliser's output is never an input to an expected value.
//!
//! The two keys were made once with `openssl genrsa 2048` and `openssl req -x509` under the round's
//! scratch directory; the signature is RFC 8017 section 8.2.1's RSASSA-PKCS1-v1_5 computed here
//! (EMSA-PKCS1-v1_5 of section 9.2, then RSASP1 of section 5.2.1), so a positive verification
//! needs no other implementation's output.

use super::*;
use crate::trusted_list::tests::base64_of;
use crate::x509_fixtures::hex;
use pdf_pki::bigint::{Integer, Modulus, modpow};

/// The list-of-trusted-lists signer's modulus.
pub(crate) const LOTL_N: &str = "\
    95dc3d7013bcf8e347642300420c3bf176ebcf88da1c826d0e246287fb98f1c49bc8252456447e9fde770c573a7f\
    f626a9ccc0a0dbd8ccfa531c051f9df054ac04c34243c7137ffb9954c1d1fb5f1a8b64288fa9911c4a2426ab09b0\
    096e26168a0cebec1b91e0becbf11419d734987d38f5042bdd3338965a695d1cee13979718c48afbbd8d3159cdf7\
    3fbde09a75bc2cf9ea8d2eb6a81a51487e13a95f3659df8a22dddaf1545d9e5ca9fac3cd2213f418be748d58b3fb\
    394056cea9ffc42f87a9ddfd02eeac9b4eca946c33480e33d8e343c7ca7c1be659897a2103b0442f03c9925fdd55\
    83502e49432e177a5e6a46348a11600ae5cbd1c728d3cfaeb811\
";

/// Its private exponent.
pub(crate) const LOTL_D: &str = "\
    23f3c7c986480fc3219d79c82b6599513f4bd368ec40155f946fa633cebdcd42c8722676fbe5f2d03d2427bc7037\
    1816ae7b30bb66ad872a319b71f48e84d9cb6b03c277558b42be59a011350f2eac6f99d7cbf4ccf0bc836fc36e24\
    e2c8f4da48c450d53ec8008dc58a50b2e258fbe70ed863e7ccbee7bf2a719ca51423747a57f5c801ef0f9ab253a9\
    90162b593844a403d0acefbbdff08880f3f268990722a79e1cf8c6561e49094e409bcfd7e247cc6865648f39ba7a\
    76f3da723aee06ff665849d143129ba7971d82a92d6162b86352aa2d223b4df6a136d102059b35d3d4eb2e607f02\
    3a158fc33e775935503d5a6e9550447a1bd5986bc4d35a1f71\
";

/// Its self-signed certificate: `C=EU`, `O=European Commission`, key usage `digitalSignature`,
/// extended key usage `id-tsl-kp-tslSigning`, `cA` false (TS 119 612 clause 5.7.1).
pub(crate) const LOTL_CERT: &str = "\
    3082035930820241a003020102020101300d06092a864886f70d01010b05003046310b3009060355040613024555\
    311c301a060355040a0c134575726f7065616e20436f6d6d697373696f6e3119301706035504030c105465737420\
    4c4f544c207369676e6572301e170d3236313131323134333732345a170d3436313130373134333732345a304631\
    0b3009060355040613024555311c301a060355040a0c134575726f7065616e20436f6d6d697373696f6e31193017\
    06035504030c1054657374204c4f544c207369676e657230820122300d06092a864886f70d01010105000382010f\
    003082010a028201010095dc3d7013bcf8e347642300420c3bf176ebcf88da1c826d0e246287fb98f1c49bc82524\
    56447e9fde770c573a7ff626a9ccc0a0dbd8ccfa531c051f9df054ac04c34243c7137ffb9954c1d1fb5f1a8b6428\
    8fa9911c4a2426ab09b0096e26168a0cebec1b91e0becbf11419d734987d38f5042bdd3338965a695d1cee139797\
    18c48afbbd8d3159cdf73fbde09a75bc2cf9ea8d2eb6a81a51487e13a95f3659df8a22dddaf1545d9e5ca9fac3cd\
    2213f418be748d58b3fb394056cea9ffc42f87a9ddfd02eeac9b4eca946c33480e33d8e343c7ca7c1be659897a21\
    03b0442f03c9925fdd5583502e49432e177a5e6a46348a11600ae5cbd1c728d3cfaeb8110203010001a352305030\
    0e0603551d0f0101ff04040302078030110603551d25040a30080606040091370300301d0603551d0e0416041409\
    766e79ca200aa394685f5a713e2eaf3b60ce2f300c0603551d130101ff04023000300d06092a864886f70d01010b\
    05000382010100370e6db563eb0d4fc24ebb53fa60cf413eba703671eca4bfc74e1912a01559f319c19f48c8e38c\
    2cd3ea5e28520bfaea4ac9d3e90100b6bcc625fb6ef88214f9d26d5d50021e80067b1027f60580e35006fc6b3b2c\
    2795f6e35b06347df44770533d4b6eacb1a58c59d6adc5f5e8164d052a6900c03189d4d97be6406efc67ed859dc1\
    2bcaa81c7e259f8fdd52ca2caa095cf53b00fc1f0bf2fa18244024b1a656b4a96cbd6600caacf87b1900a4fcb511\
    4f5660824e9139146f6a4f74dc1c76546466ea827e4a98104e9ac3f6936adaf0de49ac3bc72759871534068561cc\
    7f235aa12ed61e96e060c2b4ce257d615cb5857b373c324a9ed29abf207c792a93\
";

/// A Member State scheme operator's modulus.
pub(crate) const MEMBER_N: &str = "\
    c9fd122bb6f9be5e7fce4d3939e736dfdd2b82cc32f7b76e72d501ed44030bd383955f925a2ded47af55c6165db8\
    13e44db69ca8f5722218c68eb6e92e91928cb43caec5ac54da812ffd1c510424079ed034830af0289a108f6890c6\
    79b3aa14f43b6af6b33942e25ae55d9ab3c8f6bf2a112cb0c0dc87ed15319a27012fecb93b2ab1614941b4c68df1\
    4b841f320cb90c886ffd71644116d1e11c38a250839b3e33aa85493e5f84eb834c14fb3ec562a111aa8eb4de6841\
    4a63aacba5294b6f6d27396407b63aecfc22b4d95f5dbc8563f4aab9554ee5901423d72c36252a4de9a39e38e5a2\
    6cd8db9d28232920c70e2d9b5198f5bbcd28295ad210769ebfdd\
";

/// Its private exponent.
pub(crate) const MEMBER_D: &str = "\
    03b897cc6fb32b75f8e0c7676b2653f5227bef62ecfac749a4efeba32aacebb0da81b6a059979dac1912241e115d\
    421b389b4948e0e20362cacaf79a55734a6c867d7fb2c08ea79fd84f24bb115a75e6cfc0729191af0dccdef776d8\
    a686a576d15baedd709bce1c4c1461a4674a27d189bd85ec93aba70d65108eddc7c07c44923d77721b9e99289bd3\
    21be8c10400f640b1d51abf64cdec8866f2f82daba2349944a21588bfd0984ebfc9dd811276df0a9d067edd63adc\
    aa288423e7dca768c669ec226a8a91384c99ed2028f361826a07bdda99aaae066cfe15a0f4ec8559508d0c44b7b5\
    5ee8bb46543a73da338c8b24db5ca9e25cbf7547377a354b8e79\
";

/// Its self-signed certificate: `C=AT`, `O=Test Scheme Operator`, the same extensions.
pub(crate) const MEMBER_CERT: &str = "\
    308203573082023fa003020102020102300d06092a864886f70d01010b05003045310b3009060355040613024154\
    311d301b060355040a0c145465737420536368656d65204f70657261746f723117301506035504030c0e54657374\
    204154207369676e6572301e170d3236313131323134333732345a170d3436313130373134333732345a3045310b\
    3009060355040613024154311d301b060355040a0c145465737420536368656d65204f70657261746f7231173015\
    06035504030c0e54657374204154207369676e657230820122300d06092a864886f70d01010105000382010f0030\
    82010a0282010100c9fd122bb6f9be5e7fce4d3939e736dfdd2b82cc32f7b76e72d501ed44030bd383955f925a2d\
    ed47af55c6165db813e44db69ca8f5722218c68eb6e92e91928cb43caec5ac54da812ffd1c510424079ed034830a\
    f0289a108f6890c679b3aa14f43b6af6b33942e25ae55d9ab3c8f6bf2a112cb0c0dc87ed15319a27012fecb93b2a\
    b1614941b4c68df14b841f320cb90c886ffd71644116d1e11c38a250839b3e33aa85493e5f84eb834c14fb3ec562\
    a111aa8eb4de68414a63aacba5294b6f6d27396407b63aecfc22b4d95f5dbc8563f4aab9554ee5901423d72c3625\
    2a4de9a39e38e5a26cd8db9d28232920c70e2d9b5198f5bbcd28295ad210769ebfdd0203010001a3523050300e06\
    03551d0f0101ff04040302078030110603551d25040a30080606040091370300301d0603551d0e041604143056a6\
    1cf98268ebaba1c276b69bfa4659d7751c300c0603551d130101ff04023000300d06092a864886f70d01010b0500\
    0382010100329da96c76bd7aba2a04f70af7c351faa04aeff5298a2ce16df0cd114d04f967d26e35da85e47f4996\
    5577bb1d35924291f5ca404062f6ecdb03aba4b61b8aaebc16c8797fbde2d6d23c8fa74576bb0cf150128c36615b\
    1ccab6ff05d8913ffdfd40ef1892f4105c79dd6dcc211be6de7b04644010cd93c3b34eed2a6f5c2bc619b7910b4d\
    9f0cfa3fc025f9b6b3edff574107d90c51c3914a994c47799cbf06be5ee2a2b64eec80e65b7aab29190480e703e9\
    4cf1a8507124cddfa1dab070f376907a11a444f0b1d02f8f2ad2f9a469df785bfa31cbe295c0ae7961ed11c6b885\
    4e7f9f809be175f4391aef48cc9bdf0baa63b668a915c9c629aeb2f924aa92\
";

/// One of the two test keys.
#[derive(Clone, Copy)]
pub(crate) struct Key {
    n: &'static str,
    d: &'static str,
    certificate: &'static str,
}

/// The list-of-trusted-lists signer.
pub(crate) const LOTL_KEY: Key = Key {
    n: LOTL_N,
    d: LOTL_D,
    certificate: LOTL_CERT,
};

/// The Member State's scheme operator.
pub(crate) const MEMBER_KEY: Key = Key {
    n: MEMBER_N,
    d: MEMBER_D,
    certificate: MEMBER_CERT,
};

impl Key {
    pub(crate) fn certificate(self) -> Vec<u8> {
        hex(self.certificate)
    }

    /// RFC 8017 section 8.2.1 with SHA-256: `EM = 0x00 01 FF..FF 00 || DigestInfo || H`
    /// (section 9.2, the `DigestInfo` prefix of its note 1), then `s = m^d mod n`.
    pub(crate) fn sign(self, message: &[u8]) -> Vec<u8> {
        const SHA256_PREFIX: &str = "3031300d060960864801650304020105000420";
        let k: usize = 256;
        let mut tail = hex(SHA256_PREFIX);
        tail.extend(Digest::Sha256.compute(&[message]));
        let mut encoded = vec![0x00, 0x01];
        encoded.resize(k.saturating_sub(tail.len()).saturating_sub(1), 0xff);
        encoded.push(0x00);
        encoded.extend(tail);
        let modulus = Modulus::new(&Integer::from_be_bytes(&hex(self.n)).expect("n")).expect("n");
        let m = Integer::from_be_bytes(&encoded).expect("EM");
        let d = Integer::from_be_bytes(&hex(self.d)).expect("d");
        modpow(&m, &d, &modulus).be_bytes(k)
    }
}

/// The root's start tag, in canonical form: its one namespace declaration, then its `Id`.
pub(crate) const ROOT: &str =
    "<tsl:TrustServiceStatusList xmlns:tsl=\"http://uri.etsi.org/02231/v2#\" Id=\"TSL\">";

/// How a fixture's signature is shaped; [`Shape::ANNEX_B`] is TS 119 612 Annex B.1.0's.
#[derive(Clone, Copy)]
pub(crate) struct Shape<'a> {
    pub(crate) uri: &'a str,
    pub(crate) transforms: &'a [&'a str],
    pub(crate) canonicalization: &'a str,
    pub(crate) certificates: usize,
}

impl Shape<'static> {
    pub(crate) const ANNEX_B: Self = Self {
        uri: "",
        transforms: &[ENVELOPED_SIGNATURE, EXC_C14N],
        canonicalization: EXC_C14N,
        certificates: 1,
    };
}

/// A list whose content after the root's start tag is `body` (written canonically), signed by
/// `key` in `shape`, the signature last among the root's children.
pub(crate) fn signed_with(body: &str, key: Key, shape: Shape<'_>) -> String {
    let close = "</tsl:TrustServiceStatusList>";
    // Section 6.6.4 removes the signature; section 4.4.3.3 the comments, of which there are none.
    let covered = format!("{ROOT}{body}{close}");
    let digest = base64_of(&Digest::Sha256.compute(&[covered.as_bytes()]));
    let transforms = shape
        .transforms
        .iter()
        .map(|uri| format!("<ds:Transform Algorithm=\"{uri}\"></ds:Transform>"))
        .collect::<Vec<_>>()
        .concat();
    let inner = format!(
        "<ds:CanonicalizationMethod Algorithm=\"{}\"></ds:CanonicalizationMethod>\
         <ds:SignatureMethod Algorithm=\"http://www.w3.org/2001/04/xmldsig-more#rsa-sha256\">\
         </ds:SignatureMethod>\
         <ds:Reference URI=\"{}\"><ds:Transforms>{transforms}</ds:Transforms>\
         <ds:DigestMethod Algorithm=\"http://www.w3.org/2001/04/xmlenc#sha256\"></ds:DigestMethod>\
         <ds:DigestValue>{digest}</ds:DigestValue></ds:Reference>",
        shape.canonicalization, shape.uri
    );
    // Exclusive canonicalisation renders on the apex the `ds` prefix it visibly utilizes.
    let canonical_info = format!("<ds:SignedInfo xmlns:ds=\"{DSIG}\">{inner}</ds:SignedInfo>");
    let value = base64_of(&key.sign(canonical_info.as_bytes()));
    let certificate = format!(
        "<ds:X509Certificate>{}</ds:X509Certificate>",
        base64_of(&key.certificate())
    );
    let certificates = certificate.repeat(shape.certificates);
    format!(
        "{ROOT}{body}<ds:Signature xmlns:ds=\"{DSIG}\"><ds:SignedInfo>{inner}</ds:SignedInfo>\
         <ds:SignatureValue>{value}</ds:SignatureValue><ds:KeyInfo><ds:X509Data>{certificates}\
         </ds:X509Data></ds:KeyInfo></ds:Signature>{close}"
    )
}

/// A list signed in Annex B.1.0's shape.
pub(crate) fn signed(body: &str, key: Key) -> String {
    signed_with(body, key, Shape::ANNEX_B)
}

/// A body that is not a whole list: what this module verifies is the signature, not clause 5.
const BODY: &str = "<tsl:SchemeInformation><tsl:SchemeTerritory>AT</tsl:SchemeTerritory>\
                    </tsl:SchemeInformation>";

fn verify(list: &str) -> Result<Verified, XmlSignatureError> {
    verify_enveloped(list.as_bytes(), 4096)
}

/// Section 3.2: the reference's digest over the list without its signature, and the signature
/// over the canonical `ds:SignedInfo`, both hold, and the signer is the `ds:KeyInfo` certificate.
#[test]
fn a_list_signed_as_annex_b_requires_verifies_under_its_keyinfo_certificate() {
    assert_eq!(
        verify(&signed(BODY, MEMBER_KEY)),
        Ok(Verified {
            signer: MEMBER_KEY.certificate()
        })
    );
}

/// The plant: one character of the list changed after signing, and section 3.2.1's digest fails.
#[test]
fn a_list_changed_after_signing_fails_its_reference_digest() {
    let list = signed(BODY, MEMBER_KEY).replace(">AT<", ">DE<");
    assert_eq!(
        verify(&list),
        Err(XmlSignatureError::DigestDiffers(String::new()))
    );
}

/// Section 3.2.2: a signature value made by another key does not verify under the certificate.
#[test]
fn a_signature_by_another_key_fails_under_the_keyinfo_certificate() {
    let theirs = signed(BODY, LOTL_KEY);
    let value_of = |list: &str| {
        let start = list.find("<ds:SignatureValue>").expect("a value");
        let end = list.find("</ds:SignatureValue>").expect("a value");
        list.get(start..end).expect("a value").to_owned()
    };
    let ours = signed(BODY, MEMBER_KEY);
    let forged = ours.replace(&value_of(&ours), &value_of(&theirs));
    assert_eq!(verify(&forged), Err(XmlSignatureError::SignatureDiffers));
}

/// The same signature verifies with the root named by its identifier (section 4.4.3.3's bare
/// name), and both forms satisfy Annex B.1.0 rule 2.
#[test]
fn the_root_named_by_its_identifier_is_the_list_too() {
    let shape = Shape {
        uri: "#TSL",
        ..Shape::ANNEX_B
    };
    assert!(verify(&signed_with(BODY, MEMBER_KEY, shape)).is_ok());
}

/// Annex B.1.0 rule 3: `ds:SignedInfo` canonicalised by inclusive Canonical XML is refused by
/// name, whatever the arithmetic would say.
#[test]
fn a_signed_info_under_another_canonicalisation_is_refused_by_name() {
    let shape = Shape {
        canonicalization: canonical_xml::C14N,
        ..Shape::ANNEX_B
    };
    assert_eq!(
        verify(&signed_with(BODY, MEMBER_KEY, shape)),
        Err(XmlSignatureError::Canonicalization(
            canonical_xml::C14N.to_owned()
        ))
    );
}

/// Annex B.1.0 rule 2: a reference that digests the list without the enveloped signature
/// transform does not cover it. The transform's absence means the signature is digested too, so
/// the digest computed over the list without it differs first — which is the section 3.2.1 failure
/// a list signed this way would meet.
#[test]
fn a_reference_without_the_enveloped_transform_does_not_cover_the_list() {
    let shape = Shape {
        transforms: &[EXC_C14N],
        ..Shape::ANNEX_B
    };
    assert_eq!(
        verify(&signed_with(BODY, MEMBER_KEY, shape)),
        Err(XmlSignatureError::DigestDiffers(String::new()))
    );
}

/// TS 119 612 clause 5.7.1: `ds:KeyInfo` carries the operator's certificate and no other.
#[test]
fn two_certificates_in_keyinfo_are_refused() {
    let shape = Shape {
        certificates: 2,
        ..Shape::ANNEX_B
    };
    assert_eq!(
        verify(&signed_with(BODY, MEMBER_KEY, shape)),
        Err(XmlSignatureError::Certificates(2))
    );
}

/// A second `ds:Signature` beside the root's children, or a second element with the identifier a
/// reference names, is a wrapping shape and is refused rather than resolved.
#[test]
fn a_second_signature_or_a_second_identifier_is_refused() {
    let list = signed(BODY, MEMBER_KEY);
    let start = list.find("<ds:Signature ").expect("a signature");
    let end = list.find("</ds:Signature>").expect("a signature") + "</ds:Signature>".len();
    let signature = list.get(start..end).expect("a signature");
    let doubled = list.replace(signature, &signature.repeat(2));
    assert_eq!(
        verify(&doubled),
        Err(XmlSignatureError::Ambiguous("ds:Signature".to_owned()))
    );
    let shape = Shape {
        uri: "#TSL",
        ..Shape::ANNEX_B
    };
    let wrapped = signed_with(
        "<tsl:SchemeInformation Id=\"TSL\"></tsl:SchemeInformation>",
        MEMBER_KEY,
        shape,
    );
    assert!(matches!(
        verify(&wrapped),
        Err(XmlSignatureError::Ambiguous(_))
    ));
}

/// An unsigned list, a transform outside the four this crate computes, and a reference outside
/// the document are each named.
#[test]
fn what_is_not_computed_is_named() {
    let unsigned = format!("{ROOT}{BODY}</tsl:TrustServiceStatusList>");
    assert_eq!(verify(&unsigned), Err(XmlSignatureError::Unsigned));
    let xpath = "http://www.w3.org/TR/1999/REC-xpath-19991116";
    let shape = Shape {
        transforms: &[ENVELOPED_SIGNATURE, xpath],
        ..Shape::ANNEX_B
    };
    assert_eq!(
        verify(&signed_with(BODY, MEMBER_KEY, shape)),
        Err(XmlSignatureError::Unsupported(format!(
            "the transform {xpath}"
        )))
    );
    let shape = Shape {
        uri: "https://example.org/list.xml",
        ..Shape::ANNEX_B
    };
    assert!(matches!(
        verify(&signed_with(BODY, MEMBER_KEY, shape)),
        Err(XmlSignatureError::Unsupported(_))
    ));
}

/// Section 6.4.3's value is `r || s` at the order's width; the ECDSA fixture's DER value,
/// re-encoded that way, verifies under the same key, and the DER form itself does not.
#[test]
fn an_ecdsa_value_is_the_concatenation_of_r_and_s() {
    use crate::ecdsa::fixtures::{MESSAGE, P256_CERTIFICATE, P256_SIGNATURE, hex};
    let certificate_der = hex(P256_CERTIFICATE);
    let certificate = x509::parse(&certificate_der).expect("a certificate");
    let PublicKey::Ec(key) = certificate.public_key else {
        panic!("an EC key");
    };
    let der = hex(P256_SIGNATURE);
    // X.690: SEQUENCE, short length, then two INTEGERs, each its tag, short length and contents,
    // a leading zero octet where the top bit would otherwise read as a sign.
    let mut at = 2;
    let mut halves = Vec::new();
    for _ in 0..2 {
        let length = usize::from(der[at + 1]);
        let contents = &der[at + 2..at + 2 + length];
        let significant: Vec<u8> = contents.iter().copied().skip_while(|&o| o == 0).collect();
        let mut half = vec![0_u8; 32 - significant.len()];
        half.extend(significant);
        halves.extend(half);
        at += 2 + length;
    }
    let digest = Digest::Sha256.compute(&[MESSAGE]);
    assert_eq!(ecdsa::verify_concatenated(key, &halves, &digest), Ok(true));
    let mut plant = halves.clone();
    plant[40] ^= 1;
    assert_eq!(ecdsa::verify_concatenated(key, &plant, &digest), Ok(false));
    assert!(ecdsa::verify_concatenated(key, &der, &digest).is_err());
}
