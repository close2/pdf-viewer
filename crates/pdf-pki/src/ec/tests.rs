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

/// SEC 1 section 3.4's primitive, run by the static-only party of SP 800-56A's One-Pass MQV,
/// against NIST's own validation vectors for that scheme.
///
/// The Cryptographic Algorithm Validation Program's `KASTestVectorsECC2016` holds, under "ECC One
/// Pass MQV Scheme", `KASValidityTest_ECCOnePassMQV_KDFConcat_KC_resp_rcpt_ulat.fax`: the
/// implementation under test is the responder, which "has only a static key pair" (SP 800-56A
/// section 5.7.2.3.2), and each vector gives its static key `dsIUT`, the initiator's static and
/// ephemeral public keys `QsCAVS` and `QeCAVS`, and `Z`, the shared secret the primitive yields. The
/// first vector each of the P-256, P-384 and P-521 sections marks `Result = P` is taken (`COUNT =
/// 3`, `0` and `4`); the P-521 values are printed there in 68 octets, and the two leading zero
/// octets are dropped to the field's 66. `Z` is SEC 1's `z`.
#[test]
fn the_one_pass_mqv_primitive_reproduces_nist_s_responder_vectors() {
    let vectors = [
        (
            EcCurve::P256,
            "81115bc5afe90c273b5f0896a09cdd6368305a4c80ca3bf1564b57b622e712eb",
            (
                "4f123bb72966aeb537a382480821eaec5b0b51cdfa06d659f59cbbe7b94fb399",
                "e71b8f0e851f1e431f419c51592ce60863ebc074c1b7c384b25621dc31fb5821",
            ),
            (
                "142892998ee2bef5d07bbacf2351ab6a656d6d933fa39f036f010565bc7205e2",
                "94a766e25784bf23d428c2f4bc1c834f7c80102d63781b867207fa62efc5dd6b",
            ),
            "018aaf3d643486ff0d087da13c7c86dba89c7d5b59a8c196d9aa63274c207b91",
        ),
        (
            EcCurve::P384,
            "1ea8a7725ec4ec72849c68e79f4f57ee8571a9f313b54eb93dfad941f0b7350eb8c70efcb9c4df50669074ccaf5546ca",
            (
                "2c64a811859a0b9503a4a3569bef2849d6c1beae8b50d001708f689c9b777ab567a42dcf8930ba0e2fc332ec8a660ad1",
                "bf6c2fb8f407d36adeeab42d7dbb05f4ef5c7b5fd39ba3dc351fded981d09b5e4919d3a375c1f41e23c12e7b62d5ffd6",
            ),
            (
                "5d3e89cb6bf30ed027e2d33167046fd7155a54c23f6e748948b18911c5a49150b45508bada3f95852750d819184045e9",
                "b25ccfb6c6be546d2b1394d231b1149faca17d71cb7a7fdcebea7a5a4fc35ba1814e5e8e6503d6e8691e63448af181da",
            ),
            "b82bdd3bd7b397bf2cccfdd057012bb5f62f6c89beaca5339e2449e3cdb553c65b1e5734f2bc64accc2f9488f2f173af",
        ),
        (
            EcCurve::P521,
            "014d72457709aeb639d142940115624f959b9b429251fed66ab153315efe7eeb2432a5bfcaecd3a75c107502f9768b0a1601641536d79d18a86f723e7b68ecf962f6",
            (
                "01f6f0b3a795b819590cdb4d8fe6d684ec967e6d61a9598b025fd0a2c25ffdcf219adda284c74219cb23af15ad876429c860927214060008f9b117e22b2d6d0619fa",
                "01649a7733f1f5112a0b72449d7971fc205e31bbb16ff040cad222fb7d3cdbd20dae82b5639888f2c5c14dd1e4d733aeb3372a150fbdf70363bf75c64f56ea734b7d",
            ),
            (
                "0179b7b6878ab57daf872fef53b4e1d8fcdde2ef47380141fc291672428e460a30cd278b46e874f7cff2983aae226cf560571e02632395c87269f6c674a93688eaba",
                "004db3ccfbbf736fa118a01bfa77dfc0c3d13da5b2b601b96db51679a9b2f14e6e49554211f830f02850d3c4d5ed62a245b2bf4010818568ff7fc069b7fc0793e16e",
            ),
            "00292c3faee0083f014c221fb259bc0ce209fd664ed0e4ba845477220e098ebc4743f5269321eae10e76e62317e10a240c35e5c1bf68356f2a3e9aa59b4f0efbbf81",
        ),
    ];
    for (curve, scalar, (static_x, static_y), (ephemeral_x, ephemeral_y), z) in vectors {
        let key = PrivateKey::from_der(&ec_private_key(curve, &hex(scalar))).unwrap();
        let shared = key
            .agree_one_pass_mqv(
                &uncompressed(static_x, static_y),
                &uncompressed(ephemeral_x, ephemeral_y),
            )
            .unwrap();
        assert_eq!(shared, hex(z), "{}", curve.name());
        // The two public keys play different parts in step 5, so exchanging them is another
        // product: the primitive is not symmetric in the sender's keys.
        assert_ne!(
            key.agree_one_pass_mqv(
                &uncompressed(ephemeral_x, ephemeral_y),
                &uncompressed(static_x, static_y),
            )
            .unwrap(),
            hex(z),
            "{}",
            curve.name()
        );
    }
}

/// RFC 5480 section 2.1.2: "The id-ecDH and id-ecMQV algorithm identifiers are used to indicate
/// the key is restricted to the respective algorithm". A key whose `OneAsymmetricKey` names
/// `id-ecMQV` runs the MQV primitive and refuses Diffie–Hellman, and one naming `id-ecDH` the other
/// way round; the bare `ECPrivateKey` and `id-ecPublicKey` run both.
#[test]
fn a_restricted_key_runs_only_its_own_scheme() {
    let scalar = hex("81115bc5afe90c273b5f0896a09cdd6368305a4c80ca3bf1564b57b622e712eb");
    let bare = ec_private_key(EcCurve::P256, &scalar);
    let wrapped = |algorithm: &[u8]| {
        let mut identifier = tlv(0x06, algorithm);
        identifier.extend(tlv(0x06, EcCurve::P256.oid().as_bytes()));
        let mut body = tlv(0x02, &[0]);
        body.extend(tlv(0x30, &identifier));
        body.extend(tlv(0x04, &bare));
        tlv(0x30, &body)
    };
    let point = uncompressed(
        "4f123bb72966aeb537a382480821eaec5b0b51cdfa06d659f59cbbe7b94fb399",
        "e71b8f0e851f1e431f419c51592ce60863ebc074c1b7c384b25621dc31fb5821",
    );
    let mqv_only = PrivateKey::from_der(&wrapped(super::ID_EC_MQV)).unwrap();
    assert_eq!(mqv_only.restriction(), super::Restriction::Mqv);
    assert_eq!(mqv_only.agree(&point), Err(EcError::Restricted("ECMQV")));
    assert!(mqv_only.agree_one_pass_mqv(&point, &point).is_ok());
    let dh_only = PrivateKey::from_der(&wrapped(super::ID_EC_DH)).unwrap();
    assert_eq!(dh_only.restriction(), super::Restriction::DiffieHellman);
    assert!(dh_only.agree(&point).is_ok());
    assert_eq!(
        dh_only.agree_one_pass_mqv(&point, &point),
        Err(EcError::Restricted("ECDH"))
    );
    for key in [
        PrivateKey::from_der(&bare).unwrap(),
        PrivateKey::from_der(&wrapped(super::ID_EC_PUBLIC_KEY)).unwrap(),
    ] {
        assert_eq!(key.restriction(), super::Restriction::Unrestricted);
        assert!(key.agree(&point).is_ok());
        assert!(key.agree_one_pass_mqv(&point, &point).is_ok());
    }
}
