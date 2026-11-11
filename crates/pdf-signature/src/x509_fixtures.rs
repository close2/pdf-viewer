//! Certificates built once with `openssl` and pasted in, shared with `signature.rs`'s tests.
//!
//! **Test vectors rather than oracles**, on the footing `pkcs1`'s constants are on: RFC 5280
//! defines the structure and RFC 8017 the arithmetic, and what a vector pins is that this tree
//! walks and computes them. No corpus document can stand in for the first: the certificates in
//! the corpus's signatures are real ones whose *private* keys nobody here holds, so a positive
//! verification over bytes this tree chose needs a key this tree made.

/// A self-signed 2048-bit certificate over the key `pkcs1`'s tests verify a signature with.
///
/// **A test vector rather than an oracle**, on the same footing as `pkcs1`'s: RFC 5280 defines
/// the structure, and what this pins is that these two hundred lines walk it. No corpus
/// document could stand in — the certificates in the corpus's signatures are real ones whose
/// keys nobody here holds, which is exactly why the *corpus* gate measures verification and
/// this test measures parsing.
pub(crate) const CERTIFICATE: &str = "\
    30820315308201fda0030201020214776792c587fa13475f8a307103e320f377\
    2065cc300d06092a864886f70d01010b0500301a3118301606035504030c0f70\
    64662d7669657765722074657374301e170d3236303830383033333534375a17\
    0d3336303830353033333534375a301a3118301606035504030c0f7064662d76\
    6965776572207465737430820122300d06092a864886f70d0101010500038201\
    0f003082010a0282010100c56a39fbe4fd00ac43c8080e81b5b2a314c57647dd\
    5317854109d621e44713cafabd7fbe5275f933f5956fd158c8fe6dab37447594\
    9366675f4feff22689459fd676925b9b55a1dec6274debe37905a3f843d322bf\
    4495164ec6e626f8c0f198f538d93e9ab8be31250ce1af107a53415c663ddcef\
    d8cef220613c58e1a9870ea2f67576e85d6457019c9b86422b3df59a664089e0\
    d5a9f97f921940eab4d95132a1b19870635d6372e4275c06d39f8943d6a971c4\
    6fa86199e5acbc24ed0a9f7c8aa50a5e57ef9df109c0bac20be6022abede06cc\
    603b1bed0e4d08e7c836af378d310edb7a752f2a9541f6da47b06291daaa0e25\
    ff4b29f6e2fc926f4789b70203010001a3533051301d0603551d0e04160414f9\
    7893bb0d13501fdd2770f997e124d19cc94f33301f0603551d23041830168014\
    f97893bb0d13501fdd2770f997e124d19cc94f33300f0603551d130101ff0405\
    30030101ff300d06092a864886f70d01010b050003820101009a997e66c47d6f\
    e8ed31eb9e3043ab13085fac7e118c1d1e4525847f1fdc8db7f8d2398b1fa786\
    93d541d4387874a3d2bc715ebba5420849e83ed9ebca6fa246834fd568ead25d\
    e1a3bbe9d6f89104b211e759f81c7d7545a2b3b95a9e0834bca043805aad2224\
    8ded832ca711f32c3063253c7b116bd789f730d0738a5be9af168fcb569da0bc\
    bc84b77c05f2a97d45c8475246ef543222fc6379b62de33f6f2d0fa50533b432\
    61c30c8f2c64533ed5474ba7ce8c8aae96ca3b128bec075dbe29f63be385ed57\
    0573cd49e8d892d706e10a8d1ff54dec1f258d676c4363d028839b8f6b5040bd\
    5168122013babed7b4b558679467ed2e7ca42a41e3deda7d01";

/// A self-signed P-256 certificate, for the one `/SubFilter` that may not carry such a key.
///
/// RFC 5480's `id-ecPublicKey`. This program verifies that family through CMS
/// ([`crate::ecdsa`], ADR 0532), and Table 260's `adbe.x509.rsa_sha1`
/// column still says **No** to it — so this fixture is what carries that "No" into a test.
/// [`crate::ecdsa::fixtures`] holds the certificates that are meant to verify.
pub(crate) const EC_CERTIFICATE: &str = "\
    3082019030820135a003020102021402b9a053ffb599e1d104c767b0ac9516e1\
    6a2827300a06082a8648ce3d040302301d311b301906035504030c127064662d\
    7669657765722065632074657374301e170d3236303830383033343930365a17\
    0d3336303830353033343930365a301d311b301906035504030c127064662d76\
    696577657220656320746573743059301306072a8648ce3d020106082a8648ce\
    3d03010703420004008481356aeb1a6299f333e9e81ebdf2616a4a0618f4ff28\
    6b314b3f849e086b96fd50fcf8b02568b9cb67b003b732130449f0ad0c4542e0\
    ecd6831ab775bc70a3533051301d0603551d0e0416041449a6344e98d3f2463c\
    150455fe60ddd0c08642b5301f0603551d2304183016801449a6344e98d3f246\
    3c150455fe60ddd0c08642b5300f0603551d130101ff040530030101ff300a06\
    082a8648ce3d04030203490030460221008976ebbd9a912abfad19d1f907ce18\
    0c7ff17039ff46c70e836f1c8dc9b073c0022100c2865a409279b22cf83ce663\
    acf64f6a0f1f8b52d1ccbcdb198d60ad5941bd31";

/// The PKCS #1 v1.5 signature of `SHA-256(b"the signed bytes")` under the RSA key above.
pub(crate) const PKCS1_SIGNATURE: &str = "\
    a8051d60075059a973c9845268a3c7848a60e36a1904180dd95d4226dad15bea\
    4050dd59907bbb865d915611e55ae1fa1017a71fe084b90695f6ed5ca75fdc91\
    7caf751663b994ed3f6f7cac36729a87b237668eec74aeb6040a682ff5c87e75\
    1763e17a5d010383187a73af9ee5e8e4f890802882baba7dbb0873847dfa55a1\
    db7bd5998ddd9456b79eb269184ac578e1451d29ea8748f6b8d2bfe4c9ec834f\
    9c9f917089d8210aec49c8538e72f05ed93fe4d4b9145acd94db022a4d16b93e\
    7ca648f59a45bcc30786e72cec2ce3147700cd6cebf2e48ff8eb1c777802003b\
    7869fdaa3f4f11773f78f2002bf1a66c019e62946c3c7aed327940e7e0121204";

/// Hexadecimal to bytes.
pub(crate) fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}
