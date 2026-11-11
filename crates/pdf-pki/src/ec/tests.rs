#![expect(
    clippy::arithmetic_side_effects,
    reason = "test code: offsets into hexadecimal the test itself wrote"
)]

use super::{EcError, PrivateKey};
use crate::x509::EcCurve;

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

fn tlv(identifier: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![identifier];
    if contents.len() < 0x80 {
        out.push(u8::try_from(contents.len()).unwrap());
    } else {
        out.extend([0x81, u8::try_from(contents.len()).unwrap()]);
    }
    out.extend_from_slice(contents);
    out
}

/// RFC 5915 section 3's `ECPrivateKey { version 1, privateKey, parameters [0] namedCurve }`.
fn ec_private_key(curve: EcCurve, scalar: &[u8]) -> Vec<u8> {
    let mut body = tlv(0x02, &[1]);
    body.extend(tlv(0x04, scalar));
    body.extend(tlv(0xA0, &tlv(0x06, curve.oid().as_bytes())));
    tlv(0x30, &body)
}

/// SEC 1 section 2.3.3's uncompressed encoding, `04 || x || y`.
fn uncompressed(x: &str, y: &str) -> Vec<u8> {
    let mut point = vec![0x04];
    point.extend(hex(x));
    point.extend(hex(y));
    point
}

/// SEC 1 section 3.3.1's primitive against two published Diffie–Hellman vectors per curve family.
///
/// RFC 5903 section 8 gives, for each NIST curve, the initiator's private key `i`, the
/// responder's public key `g^r = (grx, gry)`, and "the Diffie-Hellman shared secret value is
/// girx". RFC 7027 appendix A gives the same for the Brainpool curves as `dA`, `qB = (x_qB,
/// y_qB)` and `Z = (x_Z, y_Z)`. Each `x` is SEC 1's `z`.
#[test]
fn the_primitive_agrees_with_rfc_5903_and_rfc_7027_s_vectors() {
    let vectors = [
        (
            EcCurve::P256,
            "c88f01f510d9ac3f70a292daa2316de544e9aab8afe84049c62a9c57862d1433",
            "d12dfb5289c8d4f81208b70270398c342296970a0bccb74c736fc7554494bf63",
            "56fbf3ca366cc23e8157854c13c58d6aac23f046ada30f8353e74f33039872ab",
            "d6840f6b42f6edafd13116e0e12565202fef8e9ece7dce03812464d04b9442de",
        ),
        (
            EcCurve::P384,
            "099f3c7034d4a2c699884d73a375a67f7624ef7c6b3c0f160647b67414dce655e35b538041e649ee3faef896783ab194",
            "e558dbef53eecde3d3fccfc1aea08a89a987475d12fd950d83cfa41732bc509d0d1ac43a0336def96fda41d0774a3571",
            "dcfbec7aacf3196472169e838430367f66eebe3c6e70c416dd5f0c68759dd1fff83fa40142209dff5eaad96db9e6386c",
            "11187331c279962d93d604243fd592cb9d0a926f422e47187521287e7156c5c4d603135569b9e9d09cf5d4a270f59746",
        ),
        (
            EcCurve::P521,
            "0037ade9319a89f4dabdb3ef411aaccca5123c61acab57b5393dce47608172a095aa85a30fe1c2952c6771d937ba9777f5957b2639bab072462f68c27a57382d4a52",
            "00d0b3975ac4b799f5bea16d5e13e9af971d5e9b984c9f39728b5e5739735a219b97c356436adc6e95bb0352f6be64a6c2912d4ef2d0433ced2b6171640012d9460f",
            "015c68226383956e3bd066e797b623c27ce0eac2f551a10c2c724d9852077b87220b6536c5c408a1d2aebb8e86d678ae49cb57091f4732296579ab44fcd17f0fc56a",
            "01144c7d79ae6956bc8edb8e7c787c4521cb086fa64407f97894e5e6b2d79b04d1427e73ca4baa240a34786859810c06b3c715a3a8cc3151f2bee417996d19f3ddea",
        ),
        (
            EcCurve::BrainpoolP256r1,
            "81DB1EE100150FF2EA338D708271BE38300CB54241D79950F77B063039804F1D",
            "8D2D688C6CF93E1160AD04CC4429117DC2C41825E1E9FCA0ADDD34E6F1B39F7B",
            "990C57520812BE512641E47034832106BC7D3E8DD0E4C7F1136D7006547CEC6A",
            "89AFC39D41D3B327814B80940B042590F96556EC91E6AE7939BCE31F3A18BF2B",
        ),
        (
            EcCurve::BrainpoolP384r1,
            "1E20F5E048A5886F1F157C74E91BDE2B98C8B52D58E5003D57053FC4B0BD65D6F15EB5D1EE1610DF870795143627D042",
            "4D44326F269A597A5B58BBA565DA5556ED7FD9A8A9EB76C25F46DB69D19DC8CE6AD18E404B15738B2086DF37E71D1EB4",
            "62D692136DE56CBE93BF5FA3188EF58BC8A3A0EC6C1E151A21038A42E9185329B5B275903D192F8D4E1F32FE9CC78C48",
            "0BD9D3A7EA0B3D519D09D8E48D0785FB744A6B355E6304BC51C229FBBCE239BBADF6403715C35D4FB2A5444F575D4F42",
        ),
    ];
    for (curve, private, x, y, shared) in vectors {
        let key = PrivateKey::from_der(&ec_private_key(curve, &hex(private))).unwrap();
        assert_eq!(key.curve(), curve);
        assert_eq!(
            key.agree(&uncompressed(x, y)).unwrap(),
            hex(shared),
            "{}",
            curve.name()
        );
    }
}

/// RFC 5753 section 7.2: implementations "MAY support compressed keys, and MUST NOT support hybrid
/// keys". SEC 1 section 2.3.3's compressed form of RFC 5903's P-256 point is `02` or `03` by
/// `y`'s parity (`gry` ends in `0xAB`, odd, so `03`) and agrees as the uncompressed one does; the
/// hybrid form `06`/`07 || x || y` is refused, and so is a point off the curve and the identity.
#[test]
fn a_compressed_point_agrees_and_a_hybrid_or_invalid_one_is_refused() {
    let key = PrivateKey::from_der(&ec_private_key(
        EcCurve::P256,
        &hex("c88f01f510d9ac3f70a292daa2316de544e9aab8afe84049c62a9c57862d1433"),
    ))
    .unwrap();
    let x = "d12dfb5289c8d4f81208b70270398c342296970a0bccb74c736fc7554494bf63";
    let y = "56fbf3ca366cc23e8157854c13c58d6aac23f046ada30f8353e74f33039872ab";
    let shared = hex("d6840f6b42f6edafd13116e0e12565202fef8e9ece7dce03812464d04b9442de");
    let mut compressed = vec![0x03];
    compressed.extend(hex(x));
    assert_eq!(key.agree(&compressed).unwrap(), shared);
    let mut hybrid = uncompressed(x, y);
    hybrid[0] = 0x07;
    assert_eq!(key.agree(&hybrid), Err(EcError::Point));
    let mut off_curve = uncompressed(x, y);
    off_curve[64] ^= 1;
    assert_eq!(key.agree(&off_curve), Err(EcError::Point));
    assert_eq!(key.agree(&[0x00]), Err(EcError::Point));
    assert_eq!(key.agree(&[]), Err(EcError::Point));
}

/// RFC 5915 section 3: `version` "SHALL be set to ecPrivkeyVer1"; the parameters "MUST always"
/// be included, and without them the scalar names no curve; a scalar of zero or not below the
/// order is no key; RFC 8017's `RSAPrivateKey` is another family's; brainpoolP512r1 is a curve
/// this program does not compute on, refused by name.
#[test]
fn what_is_not_a_usable_ec_private_key_is_refused_by_name() {
    let scalar = hex("c88f01f510d9ac3f70a292daa2316de544e9aab8afe84049c62a9c57862d1433");
    let mut version_two = ec_private_key(EcCurve::P256, &scalar);
    version_two[4] = 2;
    assert_eq!(
        PrivateKey::from_der(&version_two).unwrap_err(),
        EcError::Malformed("version is not ecPrivkeyVer1")
    );
    let mut no_parameters = tlv(0x02, &[1]);
    no_parameters.extend(tlv(0x04, &scalar));
    assert!(matches!(
        PrivateKey::from_der(&tlv(0x30, &no_parameters)),
        Err(EcError::Malformed(_))
    ));
    assert_eq!(
        PrivateKey::from_der(&ec_private_key(EcCurve::P256, &[0; 32])).unwrap_err(),
        EcError::Scalar
    );
    assert_eq!(
        PrivateKey::from_der(&ec_private_key(EcCurve::P256, &[0xFF; 32])).unwrap_err(),
        EcError::Scalar
    );
    assert_eq!(
        PrivateKey::from_der(&ec_private_key(EcCurve::P256, &[1; 33])).unwrap_err(),
        EcError::Scalar
    );
    let mut rsa = tlv(0x02, &[0]);
    rsa.extend(tlv(0x02, &[0x0F]));
    assert_eq!(
        PrivateKey::from_der(&tlv(0x30, &rsa)).unwrap_err(),
        EcError::NotEc
    );
    assert_eq!(
        PrivateKey::from_der(&ec_private_key(EcCurve::BrainpoolP512r1, &[1; 64])).unwrap_err(),
        EcError::Curve("brainpoolP512r1".to_owned())
    );
}

/// A shorter `privateKey` is the same integer with its leading zero octets dropped: RFC 5903's
/// P-521 scalar starts `00 37`, and read from 65 octets it agrees as from 66.
#[test]
fn a_scalar_without_its_leading_zeros_is_the_same_scalar() {
    let full = hex(
        "0037ade9319a89f4dabdb3ef411aaccca5123c61acab57b5393dce47608172a095aa85a30fe1c2952c6771d937ba9777f5957b2639bab072462f68c27a57382d4a52",
    );
    let point = uncompressed(
        "00d0b3975ac4b799f5bea16d5e13e9af971d5e9b984c9f39728b5e5739735a219b97c356436adc6e95bb0352f6be64a6c2912d4ef2d0433ced2b6171640012d9460f",
        "015c68226383956e3bd066e797b623c27ce0eac2f551a10c2c724d9852077b87220b6536c5c408a1d2aebb8e86d678ae49cb57091f4732296579ab44fcd17f0fc56a",
    );
    let long = PrivateKey::from_der(&ec_private_key(EcCurve::P521, &full)).unwrap();
    let short = PrivateKey::from_der(&ec_private_key(EcCurve::P521, &full[1..])).unwrap();
    assert_eq!(long.agree(&point).unwrap(), short.agree(&point).unwrap());
}

/// The same key in RFC 5915's bare encoding and inside RFC 5958's `OneAsymmetricKey`, written by
/// this tree's generator (`tests/envelopes/kari/<curve>.ec.der` and `.p8.der`), reads as one key
/// on the curve its certificate states; and its `Debug` says the curve and not the scalar.
#[test]
fn both_encodings_of_a_generated_key_read_on_its_curve() {
    for (name, curve, bare, wrapped) in [
        (
            "p256",
            EcCurve::P256,
            &include_bytes!("../../tests/envelopes/kari/p256.ec.der")[..],
            &include_bytes!("../../tests/envelopes/kari/p256.p8.der")[..],
        ),
        (
            "p384",
            EcCurve::P384,
            include_bytes!("../../tests/envelopes/kari/p384.ec.der"),
            include_bytes!("../../tests/envelopes/kari/p384.p8.der"),
        ),
        (
            "p521",
            EcCurve::P521,
            include_bytes!("../../tests/envelopes/kari/p521.ec.der"),
            include_bytes!("../../tests/envelopes/kari/p521.p8.der"),
        ),
        (
            "bp256",
            EcCurve::BrainpoolP256r1,
            include_bytes!("../../tests/envelopes/kari/bp256.ec.der"),
            include_bytes!("../../tests/envelopes/kari/bp256.p8.der"),
        ),
        (
            "bp384",
            EcCurve::BrainpoolP384r1,
            include_bytes!("../../tests/envelopes/kari/bp384.ec.der"),
            include_bytes!("../../tests/envelopes/kari/bp384.p8.der"),
        ),
    ] {
        let bare = PrivateKey::from_der(bare).unwrap();
        let wrapped = PrivateKey::from_der(wrapped).unwrap();
        assert_eq!((bare.curve(), wrapped.curve()), (curve, curve), "{name}");
        let shown = format!("{wrapped:?}");
        assert!(
            shown.contains(curve.name()) && !shown.contains("scalar"),
            "{shown}"
        );
    }
}
