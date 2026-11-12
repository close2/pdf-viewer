//! Fuzzes ISO 32000-2 §7.6.5's recipient lists: RFC 5652 `EnvelopedData`, read and opened.
//!
//! A public-key encrypted document's `/Recipients` are byte strings a stranger wrote, and opening
//! the document reads every one of them with `pdf_pki::enveloped::parse` and opens the one that
//! names the reader's certificate. The owner's answers A66 and A800 asked for this target with
//! the reader's first commit (ADR 2801): the envelope is attacker-controlled even in a document
//! that decrypts.
//!
//! Three properties are under test.
//!
//! **Parsing and opening terminate and never panic**, over any byte sequence, opened with two
//! fixed test certificates and their keys — an RSA one for key transport and a P-256 one for key
//! agreement (ADR 2821) — so that the RSA decryption, the padding check, the Diffie–Hellman
//! primitive, the KDF, the key unwrap and the content cipher all see hostile input whenever a
//! recipient matches, which a fuzzer reaches by copying a certificate's issuer and serial or its
//! key identifier out of the dictionary below.
//!
//! **Nothing that comes back outgrows its input.** An opened envelope is the decryption of the
//! input's own octets, so it is never longer than the input.
//!
//! **A parse is idempotent**: the same bytes give the same answer twice.

#![no_main]
#![expect(
    clippy::expect_used,
    reason = "a fuzz target states its properties by failing: `expect` and `panic!` are how a violated one reaches libFuzzer, and each message here names the property rather than the call"
)]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use pdf_pki::enveloped::parse;
use pdf_pki::key::PrivateKey;
use pdf_pki::x509;

/// A 1024-bit test key, RFC 5958's `OneAsymmetricKey`, and a self-signed certificate for it
/// (issuer `CN=pdf-pki test recipient`, serial `0x1995`): the same pair `pdf-pki`'s tests use.
const PKCS8: &str = "\
30820276020100300d06092a864886f70d0101010500048202603082025c02010002818100eb365a97770f46d9cf516a\
dba156d86d4a45f4de743c1d0f42d5011695197f9905cc16e982af627712a36ebeecad2188874759f439475a4d5b7211\
9d28730c45684d2427652db31da8e8c4f295037635754eb1aed97fbb4172ae6272976cde84490032330c9f27bfae7697\
758d46cf0640538c525889a9cfb49518ac4fd4e4c1020301000102818100b59a79c7883d5b4ec02af47e2ecb86be9b21\
f835e938a0972fda95c481c9f62287bf53d166fd52e0ec9b12de868e05de0bd4a33755222a2e9ff42a7554cdbf223cb4\
031530bf60ede5a055940029bd022c048d0f2725f2ccc2535a23c1b7e8bd6e32edc4b66aaa19fdc60fb846c0b432b699\
f682860208a6b2c1b7444736ef81024100f97df577c47692c3370dfb8fdefc354ead53c94411461e1ce0b5dab4d8a5de\
4725e88ce446430d54a95d8b531e5180ffd90429e678cbbb155a1f4901f79def45024100f15909f842822d70b6820cff\
08c4ee31546456876aa9c21f676e87ddfbd5a3e3d0ce685aefba48c72e382f50a9ae75b68898c76c2cc82de7961e031e\
4fa8894d02406df163ca73790d13999f3fcc16a76d964e2b2df620e6953e84879ee932cf65345530356622d0497f9562\
f3929d39ad6f11631453cbc77b99dd2d0c2f21bf274102404de149924f03eadde046ff229d15cf3723e06cfbadb3b1ec\
a857c6881f67c547126e48c66f6d5166d4c7549d56e3f12041dbeb1e4125278affaa179f845d025102401b1387537ac7\
25cd2c47f7ac5328431c043e6d1ce2c546e5111abbe2801f0fa01426f251c8f7e71040755aa7b8cfb2a4e6a4978b5233\
237579efb9b3c6c245b7";

const CERTIFICATE: &str = "\
3082020e30820177a00302010202021995300d06092a864886f70d01010b05003021311f301d06035504030c16706466\
2d706b69207465737420726563697069656e743020170d3236313131313030353231315a180f32313236313031383030\
353231315a3021311f301d06035504030c167064662d706b69207465737420726563697069656e7430819f300d06092a\
864886f70d010101050003818d0030818902818100eb365a97770f46d9cf516adba156d86d4a45f4de743c1d0f42d501\
1695197f9905cc16e982af627712a36ebeecad2188874759f439475a4d5b72119d28730c45684d2427652db31da8e8c4\
f295037635754eb1aed97fbb4172ae6272976cde84490032330c9f27bfae7697758d46cf0640538c525889a9cfb49518\
ac4fd4e4c10203010001a3533051301d0603551d0e041604141b0c8fde8f30ccbdbee3b32c179bc0bad090417e301f06\
03551d230418301680141b0c8fde8f30ccbdbee3b32c179bc0bad090417e300f0603551d130101ff040530030101ff30\
0d06092a864886f70d01010b050003818100add86f3cf58951adf30d367ffedeb82821ec48eee8eaa9abb27194b2063d\
46f57cb2eccd9ac02a18b7f472eca608613c51d3ba0801056fde837b5e04acc27a4cf6d6b2d8853857f623125d95057a\
5dea8c763e86fc79fa0983cd14ac74bad770680ee568437be5a3f0df047441c0a9fb9028e36a454d2db4b452c093e065\
cffc";

fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII"), 16).expect("hex digits")
        })
        .collect()
}

/// `pdf-pki`'s generated P-256 recipient: the certificate and its key as `OneAsymmetricKey`.
const EC_CERTIFICATE: &[u8] =
    include_bytes!("../../crates/pdf-pki/tests/envelopes/kari/p256.cert.der");
const EC_KEY: &[u8] = include_bytes!("../../crates/pdf-pki/tests/envelopes/kari/p256.p8.der");

fn fixtures() -> &'static [(Vec<u8>, PrivateKey); 2] {
    static FIXTURES: OnceLock<[(Vec<u8>, PrivateKey); 2]> = OnceLock::new();
    FIXTURES.get_or_init(|| {
        let rsa = PrivateKey::from_der(&hex(PKCS8)).expect("the RSA test key reads");
        let ec = PrivateKey::from_der(EC_KEY).expect("the EC test key reads");
        [(hex(CERTIFICATE), rsa), (EC_CERTIFICATE.to_vec(), ec)]
    })
}

fuzz_target!(|data: &[u8]| {
    let first = parse(data);
    assert_eq!(first, parse(data), "a parse is idempotent");
    if let Ok(envelope) = first {
        for (certificate_bytes, key) in fixtures() {
            let certificate = x509::parse(certificate_bytes).expect("the test certificate reads");
            let _ = envelope.recipient(&certificate);
            if let Ok(opened) = envelope.open(&certificate, key) {
                assert!(
                    opened.len() <= data.len(),
                    "an opened envelope is not longer than its input"
                );
            }
        }
    }
});
