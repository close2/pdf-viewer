//! RFC 5280 section 6.1 exercised over a hierarchy this project issued.
//!
//! **Why a hierarchy of our own and not the corpus's.** Every certificate in the corpus's ten
//! signature values is a real one issued by a real authority, and this project holds none of
//! those authorities' private keys — so no corpus document can furnish a path whose *positive*
//! outcome is known in advance, and none can furnish the negatives below at all: a certificate
//! that is not a CA signing another, a `keyUsage` that forbids it, a path one certificate longer
//! than a `pathLenConstraint` permits. Each of those has to be *issued* to exist. `openssl` issued
//! them once and the DER is pasted in, the way [`crate::x509::fixtures`] already does.
//!
//! **Every negative here is calibrated against the positive beside it** (trap 13): the first test
//! validates the whole chain, and each test after it changes exactly one thing — one bit of a
//! signature, one instant, one extension, one certificate in the middle — and names the refusal
//! that change must produce. A sweep that only ever saw green would be a sentence about the test
//! rather than about the algorithm.

use super::{Material, PathRefusal, Revocation, Trust, TrustAnchor, TrustAnchors, validate};
use crate::x509::{Certificate, Instant, parse};

/// 2026-06-01T00:00:00Z, inside every fixture certificate's validity period.
const WITHIN: Instant = Instant::from_unix_seconds(1_780_272_000);

/// 2027-06-01T00:00:00Z — past [`fixtures::SIGNER`]'s `notAfter` and inside its issuer's.
const AFTER_THE_SIGNER_EXPIRED: Instant = Instant::from_unix_seconds(1_811_808_000);

/// 2025-06-01T00:00:00Z, before every fixture certificate's `notBefore`.
const BEFORE_ANY_OF_THEM: Instant = Instant::from_unix_seconds(1_748_736_000);

/// Hexadecimal to bytes, as `x509`'s fixtures spell it.
fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .filter_map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// A certificate out of a fixture, panicking with the fixture's name where it will not parse.
fn certificate(bytes: &[u8]) -> Certificate<'_> {
    parse(bytes).expect("a fixture certificate this tree issued parses")
}

pub(crate) mod fixtures {
    /// The trust anchor of the fixture hierarchy: a self-signed CA, 2026 to 2036.
    ///
    /// Built once with `openssl` and pasted in, on the footing [`crate::x509::fixtures`] states:
    /// these are **test vectors rather than oracles**. RFC 5280 defines the structure and section
    /// 6.1 the algorithm, and what a vector pins is that this tree walks and computes them. No
    /// corpus document can stand in — a positive path validation needs a hierarchy whose *private*
    /// keys somebody here held, and the certificates in the corpus's signatures are real ones
    /// nobody here can issue under.
    pub(crate) const ROOT: &str = "\
        30820306308201eea003020102021470871fda503ba52cebf609b193a5742a90\
        a86b02300d06092a864886f70d01010b0500301b3119301706035504030c1071\
        756f727261207465737420726f6f74301e170d3236303130313030303030305a\
        170d3336303130313030303030305a301b3119301706035504030c1071756f72\
        7261207465737420726f6f7430820122300d06092a864886f70d010101050003\
        82010f003082010a0282010100c674b42054e8a690c47751c122315dfaa2363b\
        46ec3e35cad492d7ce965cdb90e7ff36cdcac21ef22b821d4077a3636c27f57d\
        2507ec3d08ed494383bfc5fb7285c440a7eb9b95d92e122fad743e78eab8105f\
        d4c6969330b7c2f6369408e61252936912fbbecb6092d00c107848f9de60948c\
        ad3a52b8391266668803119393348f60e1e49e34c3e0ad4356079bc07966c2c8\
        42dd4889ea3ca9a2f69966f391f7f4fe42a6abe93b40d71ae9de871a19834f0d\
        c0c98ab93c87b5ab4f1d8e287a043b687870e2738405fb634e32efe845fa8522\
        bec84ea28b8774ed3a3be2c022aceaeba2dc220287ea3347a9bfd9be06a0fd95\
        82bc5519f7f7f68f07f95aa07f0203010001a3423040300f0603551d130101ff\
        040530030101ff300e0603551d0f0101ff040403020106301d0603551d0e0416\
        0414ff1994173b886516adc9392824711115acd878ab300d06092a864886f70d\
        01010b050003820101004a2e20ba34acbe2f9b32bb065be7be5a450bc7581404\
        07ae9aaf2c193c41ec415b5ce2529e0e8c46373e480900de14aa37f2425dff19\
        fc748db98877fc0030bc7bf41a20f8518cb58f6610dea613c2fb93865a2e70fc\
        87775ffb31561b487529223a726716f01dc9258ba0ded5eeae9f8c30c32e9fd8\
        833375a54b9663cdf4d0edaef4b7f0661594a53ea3e591ec7d0f29c205ce21f7\
        750e3668e0df76fa1dc60040d710a8833dc5118926cbe86c77129b27452f065d\
        7a2b858f2b25016e7d7fd7602350e4e1ecafc509fb505b9fcfb87c549e64af75\
        39e770979c65bc80a860cc87b71bbeb3f353e9fad5c17a7a009e55b1569ee3dc\
        5cc5e674a65ee35e5e13";

    /// A CA issued by [`ROOT`], `pathLenConstraint` 0, `keyCertSign` asserted.
    pub(super) const INTERMEDIATE: &str = "\
        308203323082021aa00302010202144155d6fc8b62c46b802c029c0f9b469b10\
        49c579300d06092a864886f70d01010b0500301b3119301706035504030c1071\
        756f727261207465737420726f6f74301e170d3236303130313030303030305a\
        170d3336303130313030303030305a30233121301f06035504030c1871756f72\
        7261207465737420696e7465726d65646961746530820122300d06092a864886\
        f70d01010105000382010f003082010a0282010100b90e77a84485ce8a3d2c3d\
        7a85341dcb8a8cc46ed2417dde95990de9396a5d6fecb58fe088995b43e49b68\
        1e936f83458e40470ff38d4b40a6e951f354dbe5f853d407c48116c7392612a0\
        5a6969c95e676e86af52c71897ec9f867c792b332594761ad9168c90e4b57dc9\
        c136e0c8f0dd5d3a8af0a25fdae08670b0acc7c4f08c475f62f8d2bb790452d4\
        6240dd892307266fd0cb8bd5f25f058f06c5672e23949b23d5d688078675c1ff\
        542ac4c113a047f40f523f273d68dee0916d1bc86ecf89949171788ab7a354e6\
        49eac2c2daf06bf094796b32f69ee27e3838857bdf3e652e94448aaca3ed47ed\
        20b54c80b0e62128825c2787790ff806efc1b6c8db0203010001a36630643012\
        0603551d130101ff040830060101ff020100300e0603551d0f0101ff04040302\
        0106301d0603551d0e0416041477cc24d2bc83e87ebe82315d66abd565a41bf5\
        5f301f0603551d23041830168014ff1994173b886516adc9392824711115acd8\
        78ab300d06092a864886f70d01010b05000382010100b17e206ec169a00332be\
        bbba20b3128c33f7e263e5f1968355c5b70b4ae98fc9f4ea5b5085d4442c781b\
        7bef2b2358ce3c785039f3e9e2d94c933d6f72ab7600587d66e2fdaaee8911cd\
        ea04a68d4efc2701a8b2e99410e014b5aa233ff455a71b08e5db96ada883bc21\
        2a774113fbe331bcfeb14cb93cd9bb05dd87397b870dced129549965840d49c3\
        c1441e01e3654543eaa44a8155a5f0c7a0b6933c383630dc5d97518204087b7c\
        e7a5eec3acda8f04e930a084c3ac9219841eb95702bc3103f889bc596d185f05\
        499958049c5e84536d518693f43752cddb54ad79a4de7c5d30af47302c6b9ab7\
        f3c13544555b71a8f44a4bdccceda61bdf55c3a8806d";

    /// An end-entity certificate issued by [`INTERMEDIATE`], 2026 to **2027**.
    ///
    /// The narrower validity window is deliberate: it is what makes
    /// `a_path_outside_the_validity_period_is_refused` a test of RFC 5280 section 6.1.3 (a)(2)
    /// on one certificate of a path rather than on all of them.
    pub(super) const SIGNER: &str = "\
        3082032e30820216a00302010202142098d48ac135f5264c4317afd4dd554de2\
        d88312300d06092a864886f70d01010b050030233121301f06035504030c1871\
        756f727261207465737420696e7465726d656469617465301e170d3236303130\
        313030303030305a170d3237303130313030303030305a301d311b3019060355\
        04030c1271756f7272612074657374207369676e657230820122300d06092a86\
        4886f70d01010105000382010f003082010a0282010100bac37188c6fa919a49\
        c2cd8c8929b79b0a8da88aab078ff0c5927ca2d97f0f85cf037079853256e9f6\
        8f8a85b316e18c736a99689d986dbe9c935840498b414ae3e61436c32b04452f\
        ea6cd2be234db866c3557fabd3e708c2c5abe557bbcbebd07dddc81b880366c6\
        83f3667d6c324ba84e69e50d7fb88ee9b995d8693bdfa1ed2f1621100e2a449d\
        8920b5bd170d0484979513c1c26c10792cebf7d4910cad2735d7af8c2d117e05\
        422d4d8d87ae9d34432df265805440191ac3b160e190e62cf705b52bb164ed2d\
        e37774d826756577bed49523167ea00edaa7d86ef6806976aad7f850b530a3ed\
        23ec1736b538c2415d8da2338b845eaf210396e72a5fe50203010001a360305e\
        300c0603551d130101ff04023000300e0603551d0f0101ff0404030206c0301d\
        0603551d0e04160414a5971f9142fda632e595545a0803cad92c771837301f06\
        03551d2304183016801477cc24d2bc83e87ebe82315d66abd565a41bf55f300d\
        06092a864886f70d01010b0500038201010085f224610fe597b19d2fbf560f77\
        b51f5e4be0fee3ce66676cbcc589e04ba7a6aa66da3e3779832931e99b47d0d0\
        6c829858cd9a1699f16997386b6c65711afe999bfca97695f77f6986dac4971e\
        01c5b1e1131d31cb9d4c1e589387a534358c715135f85b0132667bb0f5dce47d\
        c94c624e54e964ff91a609d1a2a6749c17b8171ed33fa9edfe009531ab34da82\
        d38f6d8b7f83dbb1829ce6cf447e1374a9f205154372b6fa2235e56b52e5baeb\
        b206199dcdbbac0d45d1171a7c7678fb98a08a994550125fa7769f4544292f3f\
        d681a96afac676ab2272582cd64259a65b1dd0852f4f47e542f6f52aa62d45e9\
        87de04d59028917e4cfe891e174ce86dfddb";

    /// The same end-entity key, with a critical extension nobody recognises.
    ///
    /// `1.3.6.1.4.1.99999.1`, in a private enterprise arc that assigns nothing, marked critical.
    /// RFC 5280 section 4.2's `MUST` is what this fixture exists to exercise.
    pub(super) const SIGNER_CRITICAL: &str = "\
        308203433082022ba00302010202146854c1534929998ffe9e828b91db37e828\
        1a1375300d06092a864886f70d01010b050030233121301f06035504030c1871\
        756f727261207465737420696e7465726d656469617465301e170d3236303130\
        313030303030305a170d3237303130313030303030305a301d311b3019060355\
        04030c1271756f7272612074657374207369676e657230820122300d06092a86\
        4886f70d01010105000382010f003082010a0282010100bac37188c6fa919a49\
        c2cd8c8929b79b0a8da88aab078ff0c5927ca2d97f0f85cf037079853256e9f6\
        8f8a85b316e18c736a99689d986dbe9c935840498b414ae3e61436c32b04452f\
        ea6cd2be234db866c3557fabd3e708c2c5abe557bbcbebd07dddc81b880366c6\
        83f3667d6c324ba84e69e50d7fb88ee9b995d8693bdfa1ed2f1621100e2a449d\
        8920b5bd170d0484979513c1c26c10792cebf7d4910cad2735d7af8c2d117e05\
        422d4d8d87ae9d34432df265805440191ac3b160e190e62cf705b52bb164ed2d\
        e37774d826756577bed49523167ea00edaa7d86ef6806976aad7f850b530a3ed\
        23ec1736b538c2415d8da2338b845eaf210396e72a5fe50203010001a3753073\
        300c0603551d130101ff04023000300e0603551d0f0101ff0404030207803013\
        06092b06010401868d1f010101ff04030101ff301d0603551d0e04160414a597\
        1f9142fda632e595545a0803cad92c771837301f0603551d2304183016801477\
        cc24d2bc83e87ebe82315d66abd565a41bf55f300d06092a864886f70d01010b\
        05000382010100a9327ee43c6699c0d771392f7e944258a8f00c10779338d440\
        59bba05b5910d5de9da62f12b09cef9684d312d718c576dbc1cfbb56ba65d293\
        3419c28a4e056bd1a002232310e7f13d080724feae032df64c71a089a730a965\
        92af558f1d9d496bbbb337204ec7744485d1a19ba6c473580df023e3d5e142a8\
        b50f6947c0a6b7a654f8b3d116fc731ec577acaaabdb5e42a5ee5fbeb55e55fe\
        1b37438d20d6f661e309da5854079e00a77cfcc7d8c526a6698b11e46047b39f\
        4dd962f640290ddf0807ef879296f4ac68f5b5743c3800740edae040bb31f609\
        4a02a5a5111018f46ad35b81166c39a117ab262d1d71e30f5c58a709ad231503\
        06d9d5c7bc9e25";

    /// A certificate issued by [`ROOT`] that states `basicConstraints` with `cA` **false**.
    pub(super) const NOT_A_CA: &str = "\
        3082031c30820204a003020102021437fa45736da09ddfff24d2bbb4f2a4f044\
        ba7eb0300d06092a864886f70d01010b0500301b3119301706035504030c1071\
        756f727261207465737420726f6f74301e170d3236303130313030303030305a\
        170d3336303130313030303030305a30233121301f06035504030c1871756f72\
        7261207465737420696e7465726d65646961746530820122300d06092a864886\
        f70d01010105000382010f003082010a0282010100b90e77a84485ce8a3d2c3d\
        7a85341dcb8a8cc46ed2417dde95990de9396a5d6fecb58fe088995b43e49b68\
        1e936f83458e40470ff38d4b40a6e951f354dbe5f853d407c48116c7392612a0\
        5a6969c95e676e86af52c71897ec9f867c792b332594761ad9168c90e4b57dc9\
        c136e0c8f0dd5d3a8af0a25fdae08670b0acc7c4f08c475f62f8d2bb790452d4\
        6240dd892307266fd0cb8bd5f25f058f06c5672e23949b23d5d688078675c1ff\
        542ac4c113a047f40f523f273d68dee0916d1bc86ecf89949171788ab7a354e6\
        49eac2c2daf06bf094796b32f69ee27e3838857bdf3e652e94448aaca3ed47ed\
        20b54c80b0e62128825c2787790ff806efc1b6c8db0203010001a350304e300c\
        0603551d130101ff04023000301d0603551d0e0416041477cc24d2bc83e87ebe\
        82315d66abd565a41bf55f301f0603551d23041830168014ff1994173b886516\
        adc9392824711115acd878ab300d06092a864886f70d01010b05000382010100\
        76836e4a4db6d0421002c59654d89121f23045b08d7e2eedb7faf96500d5355e\
        c178223e8bf719b5856a1ad9702ad46b292961e0c2ac7afca1f9860de98d2a03\
        7c17c83c3db14222136605a28791e18e328354a3f5e301e40116e4f27cacf22c\
        a5acd44e6ad370209fdc5f69ab90f27623d44a62eb2146d763e98088a53b1426\
        008bb900523671a98f6e924509adf582abcb3b98abb71f8e8d69e65b5155c6b6\
        cba2d160443414bd38bfb1be16440624315f1ce4dacc73dbc6dcc7d1e914ab91\
        82994a24b2f320406cf1e3034f6612c47c6b49b2cb29096d0689e0da671da918\
        3ffa0a6f176a21836c38f0797b60159f688207a8900cc69a5a2bbaca37e48d3d";

    /// An end-entity certificate [`NOT_A_CA`] nevertheless signed — RFC 5280 section 6.1.4 (k).
    pub(super) const NOT_A_CA_SIGNER: &str = "\
        3082032e30820216a003020102021420c81f9208e90e0f5665fd17176ec94268\
        62131b300d06092a864886f70d01010b050030233121301f06035504030c1871\
        756f727261207465737420696e7465726d656469617465301e170d3236303130\
        313030303030305a170d3237303130313030303030305a301d311b3019060355\
        04030c1271756f7272612074657374207369676e657230820122300d06092a86\
        4886f70d01010105000382010f003082010a0282010100bac37188c6fa919a49\
        c2cd8c8929b79b0a8da88aab078ff0c5927ca2d97f0f85cf037079853256e9f6\
        8f8a85b316e18c736a99689d986dbe9c935840498b414ae3e61436c32b04452f\
        ea6cd2be234db866c3557fabd3e708c2c5abe557bbcbebd07dddc81b880366c6\
        83f3667d6c324ba84e69e50d7fb88ee9b995d8693bdfa1ed2f1621100e2a449d\
        8920b5bd170d0484979513c1c26c10792cebf7d4910cad2735d7af8c2d117e05\
        422d4d8d87ae9d34432df265805440191ac3b160e190e62cf705b52bb164ed2d\
        e37774d826756577bed49523167ea00edaa7d86ef6806976aad7f850b530a3ed\
        23ec1736b538c2415d8da2338b845eaf210396e72a5fe50203010001a360305e\
        300c0603551d130101ff04023000300e0603551d0f0101ff040403020780301d\
        0603551d0e04160414a5971f9142fda632e595545a0803cad92c771837301f06\
        03551d2304183016801477cc24d2bc83e87ebe82315d66abd565a41bf55f300d\
        06092a864886f70d01010b05000382010100b2889d798a0e2638c1ff19bdfed9\
        1184436534b36b51a4ef533e58276bdaf923e0b7d82afa4622a7060c1d5861d0\
        df9c5c25b0ff77c1bf43de2567bdfb82114f6a564bea0cb4256b2576f8690fa3\
        86d716bcfe133b72bbc3c7b60b578b5cb1f4eec0f22a54a8e5f213baf3011bd5\
        ae60bdfbacc58bfc0a2bdb4ad3c24c9eaa23fdae41cb409758a594c084a608dc\
        e82130f867a685a7aa49ce2a683eb8f6b0ddd1023ed78b1f576f202e36044544\
        28b58609dec8b5eb06806e6ce8f858f2cc97037b1729b8baa12f477e7199dd80\
        38f791c9689db833ff7434763d31e253fae352d0400889920dab321a3b5c00fa\
        b0d3111e0d8839ab871d3dfa5a672e38ebca";

    /// A CA issued by [`ROOT`] whose `keyUsage` omits `keyCertSign`.
    pub(super) const NO_CERT_SIGN: &str = "\
        3082032f30820217a00302010202142920acfc26facdeb1f2b177b27202fe6a7\
        24f8ba300d06092a864886f70d01010b0500301b3119301706035504030c1071\
        756f727261207465737420726f6f74301e170d3236303130313030303030305a\
        170d3336303130313030303030305a30233121301f06035504030c1871756f72\
        7261207465737420696e7465726d65646961746530820122300d06092a864886\
        f70d01010105000382010f003082010a0282010100b90e77a84485ce8a3d2c3d\
        7a85341dcb8a8cc46ed2417dde95990de9396a5d6fecb58fe088995b43e49b68\
        1e936f83458e40470ff38d4b40a6e951f354dbe5f853d407c48116c7392612a0\
        5a6969c95e676e86af52c71897ec9f867c792b332594761ad9168c90e4b57dc9\
        c136e0c8f0dd5d3a8af0a25fdae08670b0acc7c4f08c475f62f8d2bb790452d4\
        6240dd892307266fd0cb8bd5f25f058f06c5672e23949b23d5d688078675c1ff\
        542ac4c113a047f40f523f273d68dee0916d1bc86ecf89949171788ab7a354e6\
        49eac2c2daf06bf094796b32f69ee27e3838857bdf3e652e94448aaca3ed47ed\
        20b54c80b0e62128825c2787790ff806efc1b6c8db0203010001a3633061300f\
        0603551d130101ff040530030101ff300e0603551d0f0101ff04040302018230\
        1d0603551d0e0416041477cc24d2bc83e87ebe82315d66abd565a41bf55f301f\
        0603551d23041830168014ff1994173b886516adc9392824711115acd878ab30\
        0d06092a864886f70d01010b050003820101000fc2e3661932b62aafc2024820\
        c4a06881a0815c7f2f8a1769cddea50d28ad0da093817a3edae7330aba09d50e\
        6ac6d658c756aa1bc7e7eb91c2c87ced9633e3a00604311b55012c5ecccc5186\
        5b0eeae5bb6a326fc69860711cf816acdc7c439d34f72afde0c0c570f0b72fef\
        81085f8fa15366c010ebbfd3fe6760b8fb1dc0b8fedb84e738631321b9afc6bd\
        76babf202e583938644048af238dae9b412b0f1ec8e85c8333b3a0600b0b02d0\
        035f00996a8d8dcd70f5fe3144dd5417c048864ba14a1878387bdeda1d0f1770\
        e87e733d21578d33c0e65827b1181660475752fe243d9d9dc3785329fd9f2262\
        8f5b4cd7ba1e0e9cd6fddbd5ab8a587d881eda";

    /// An end-entity certificate [`NO_CERT_SIGN`] signed — RFC 5280 section 6.1.4 (n).
    pub(super) const NO_CERT_SIGN_SIGNER: &str = "\
        3082032e30820216a0030201020214470fa0b3b1e91002e22f34ed1bbaf3491a\
        fd7cb4300d06092a864886f70d01010b050030233121301f06035504030c1871\
        756f727261207465737420696e7465726d656469617465301e170d3236303130\
        313030303030305a170d3237303130313030303030305a301d311b3019060355\
        04030c1271756f7272612074657374207369676e657230820122300d06092a86\
        4886f70d01010105000382010f003082010a0282010100bac37188c6fa919a49\
        c2cd8c8929b79b0a8da88aab078ff0c5927ca2d97f0f85cf037079853256e9f6\
        8f8a85b316e18c736a99689d986dbe9c935840498b414ae3e61436c32b04452f\
        ea6cd2be234db866c3557fabd3e708c2c5abe557bbcbebd07dddc81b880366c6\
        83f3667d6c324ba84e69e50d7fb88ee9b995d8693bdfa1ed2f1621100e2a449d\
        8920b5bd170d0484979513c1c26c10792cebf7d4910cad2735d7af8c2d117e05\
        422d4d8d87ae9d34432df265805440191ac3b160e190e62cf705b52bb164ed2d\
        e37774d826756577bed49523167ea00edaa7d86ef6806976aad7f850b530a3ed\
        23ec1736b538c2415d8da2338b845eaf210396e72a5fe50203010001a360305e\
        300c0603551d130101ff04023000300e0603551d0f0101ff040403020780301d\
        0603551d0e04160414a5971f9142fda632e595545a0803cad92c771837301f06\
        03551d2304183016801477cc24d2bc83e87ebe82315d66abd565a41bf55f300d\
        06092a864886f70d01010b05000382010100b361fa68d846f56be93c079ac8a9\
        cbeb2d2fc4ff9a7af38c9ad1f00a3d28878bef58e82de01501920b3060e2fe21\
        c4fe7cdcc87dfa7451a3f778428f2d5e4381adc0e3d14fc2cf20fe9e907b8552\
        f8d79045674d07cefa60a4598932cbd251b3f3878e66b01adc94483af34c326b\
        a345fb27090b3c64580c6ee44037c8cc094e7a5211286fc4d35d05194e38cf25\
        055d095d52395bba9af11185da4e7fa112ab26168080dddf5160fb7e6f9d9917\
        d9d264b8ffd6cc9710ec13276f6e6eddeb3fa7eb1a14c708215ce550ec202bda\
        3142e01c29650e02bbb78ce892031620a3f52b4620fdba365cf70ab0967dcd0e\
        cdbe0e554e16fdf65995147dc9f5e2193f43";

    /// A CA issued by [`INTERMEDIATE`], which states `pathLenConstraint` 0.
    pub(super) const DEEP: &str = "\
        3082032f30820217a003020102021409f2dfb158a6738b0974e6d4f21c34abc4\
        74d458300d06092a864886f70d01010b050030233121301f06035504030c1871\
        756f727261207465737420696e7465726d656469617465301e170d3236303130\
        313030303030305a170d3336303130313030303030305a301b31193017060355\
        04030c1071756f7272612074657374206465657030820122300d06092a864886\
        f70d01010105000382010f003082010a0282010100c700ee1ed4e238a497b42d\
        feb6618dbf2140f97e33ae49ad5d37864b895ccb619ed740534fa33a32c95edb\
        fe84f9953ef0d2e8b6a41c861266e2395a1e96413df81b114122dfe4388f2d56\
        66dab1bbbdc5ec84562cad756fbbe35a9d15b7c23617162885d6ece286a8aae7\
        7775938897fedb0288803e9baba6c39e5775687740a6051071297bc5eff26510\
        f2642220ba4c2b7fa38ea3246b69c975835281c678ddc159a6e20fee02b4fbde\
        2994c5de6c173dd21c8ba3550479951324fd0f9908dcf58ddfecc6c36f0deb1a\
        31126b4429909fed47c1c7cd1c729ca191a42b73dba588779796bb3ffbd5f549\
        88c6ab3b911fd1d42c4646362cd8edd1856ebf4fad0203010001a3633061300f\
        0603551d130101ff040530030101ff300e0603551d0f0101ff04040302020430\
        1d0603551d0e04160414e83f98f5e7b1a9d87f9777c8cc8e9e2643c89059301f\
        0603551d2304183016801477cc24d2bc83e87ebe82315d66abd565a41bf55f30\
        0d06092a864886f70d01010b050003820101009ef7ab4e98e266b5cc62c62357\
        cd7ae85e26468baf5fa20b1cf8300e2c90692c0ed5e4cff30df77dbb325df518\
        6a40c8acbb582f821cfede2bf21b18f478c914f662dbef88545135bb87986626\
        782e25439ca9b43a900e8de94d1190ed4ae98b30804b1da2a002f2728b0af847\
        7ae2507db1d65673a45d7e32e41ce0bac009bf591c69454ae522147179489cd6\
        d72f070833be9b88a04835ff5727e44672e9891ce38c5f26a923cdaf7141709a\
        0447675969674f4e2a09f8c5f90d920d659854d9d83db7c501996660a90aeb4d\
        ce64d88f87db3aad3157bcc228458caa674aa9ccef2d76c07ceeb351e78dec3c\
        6dd2992d366abfafb2f9e3eb5d3e30d1a6a3d5";

    /// An end-entity certificate issued by [`DEEP`] — one certificate past what
    /// [`INTERMEDIATE`]'s `pathLenConstraint` permits, so RFC 5280 section 6.1.4 (l) refuses it.
    pub(super) const DEEP_SIGNER: &str = "\
        308203263082020ea0030201020214553e70e8fd1d04469c7ad9332f1272c43b\
        19bcdb300d06092a864886f70d01010b0500301b3119301706035504030c1071\
        756f72726120746573742064656570301e170d3236303130313030303030305a\
        170d3237303130313030303030305a301d311b301906035504030c1271756f72\
        72612074657374207369676e657230820122300d06092a864886f70d01010105\
        000382010f003082010a0282010100bac37188c6fa919a49c2cd8c8929b79b0a\
        8da88aab078ff0c5927ca2d97f0f85cf037079853256e9f68f8a85b316e18c73\
        6a99689d986dbe9c935840498b414ae3e61436c32b04452fea6cd2be234db866\
        c3557fabd3e708c2c5abe557bbcbebd07dddc81b880366c683f3667d6c324ba8\
        4e69e50d7fb88ee9b995d8693bdfa1ed2f1621100e2a449d8920b5bd170d0484\
        979513c1c26c10792cebf7d4910cad2735d7af8c2d117e05422d4d8d87ae9d34\
        432df265805440191ac3b160e190e62cf705b52bb164ed2de37774d826756577\
        bed49523167ea00edaa7d86ef6806976aad7f850b530a3ed23ec1736b538c241\
        5d8da2338b845eaf210396e72a5fe50203010001a360305e300c0603551d1301\
        01ff04023000300e0603551d0f0101ff040403020780301d0603551d0e041604\
        14a5971f9142fda632e595545a0803cad92c771837301f0603551d2304183016\
        8014e83f98f5e7b1a9d87f9777c8cc8e9e2643c89059300d06092a864886f70d\
        01010b050003820101007420a40b0b4606d125d103bd11845329f8022716dfbe\
        b7aa06107f0c441990bd78ae4dc4602069e658b0fd22d13504eb283b0d4c9a92\
        f74f395287546b75feca7f04678b81528e76f0c12aa7e46c8cbae30689bf0bd1\
        61d0b11b2cd999c7122032ee42414ec7a129f67b39bb0511d9b982f3dcd6364e\
        3cd08d9def962e7c530267e4b7de0e5f9f38f9e2ea2752afa586a1f26f070a09\
        9047263bbc5315eba8c458f028f5448f9c24f576e5ad8e53d7b9f0dfa69b145c\
        b24cdca93757d123e085b7be9d4db8aacb0a7e6cb8ab2c0abad46fb6a645d3c8\
        4566211221c88367dc14c4a551a8de2f071e3ecdf7d68aa7cf32198ee9dc4fd4\
        608f6568306bb7309fa6";
}

#[test]
fn a_chain_to_a_supplied_anchor_validates() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::SIGNER),
    );
    let (root, mid, leaf) = (certificate(&root), certificate(&mid), certificate(&leaf));
    let anchors = TrustAnchors::of(&[root]);
    assert_eq!(
        validate(&leaf, &[mid], &anchors, &Material::none(), WITHIN),
        Trust::Anchored {
            length: 2,
            revocation: Revocation::NotChecked,
        },
        "the signer and its one intermediate are RFC 5280 section 6.1's certificates 1 and 2"
    );
}

#[test]
fn an_anchored_path_asked_with_no_material_says_nothing_about_revocation() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::SIGNER),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    let Trust::Anchored { revocation, .. } = validate(
        &certificate(&leaf),
        &[certificate(&mid)],
        &anchors,
        &Material::none(),
        WITHIN,
    ) else {
        panic!("the fixture chain validates");
    };
    // RFC 5280 section 6.1.3 (a)(3) is answered from §12.8.4's material and there is none here, so
    // the one thing this assertion is for is that the success value says *that* rather than
    // something reassuring. `Revocation::Good` appearing here is the defect it exists to catch.
    assert_eq!(revocation, Revocation::NotChecked);
}

#[test]
fn with_no_anchor_there_is_no_question_to_answer() {
    let (mid, leaf) = (hex(fixtures::INTERMEDIATE), hex(fixtures::SIGNER));
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[certificate(&mid)],
            &TrustAnchors::none(),
            &Material::none(),
            WITHIN,
        ),
        Trust::NoAnchorSupplied,
        "the default state of this program, and not a defect in the document"
    );
}

#[test]
fn an_anchor_that_issued_nothing_here_yields_no_path() {
    let (mid, leaf) = (hex(fixtures::INTERMEDIATE), hex(fixtures::SIGNER));
    let leaf = certificate(&leaf);
    // The signer's *own* subject as an anchor name: a set of one that no certificate in the
    // hierarchy was issued by, which is the shape of a host supplying the wrong store.
    let mut anchors = TrustAnchors::none();
    anchors.push(TrustAnchor::of(&leaf));
    assert_eq!(
        validate(
            &leaf,
            &[certificate(&mid)],
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::NoPathToAnyAnchor { examined: 0 },
        "nothing was validated because no prospective path ever reached an anchor"
    );
}

#[test]
fn the_intermediate_is_needed_and_its_absence_is_not_a_refusal() {
    let (root, leaf) = (hex(fixtures::ROOT), hex(fixtures::SIGNER));
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[],
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::NoPathToAnyAnchor { examined: 0 },
        "a file that carries too few certificates says so as a missing path, not as a bad one"
    );
}

#[test]
fn one_turned_bit_in_an_intermediates_signature_breaks_the_path() {
    let (root, leaf) = (hex(fixtures::ROOT), hex(fixtures::SIGNER));
    let mut mid = hex(fixtures::INTERMEDIATE);
    // The last octet of the certificate is the last octet of `signatureValue`, which is the
    // issuer's RSA signature over the `tbsCertificate`. This is the calibration the positive test
    // above needs: with the bit as issued the chain validates, and with it turned over it does
    // not, so the verification is doing the work rather than the name comparison alone.
    let last = mid.len().saturating_sub(1);
    mid[last] ^= 0x01;
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[certificate(&mid)],
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::Refused {
            refusal: PathRefusal::SignatureNotUnderIssuersKey,
            examined: 1,
        },
        "RFC 5280 section 6.1.3 (a)(1) is what fails, and it fails loudly — on the first \
         certificate the walk looks at, because section 6.1 numbers them from the anchor down"
    );
}

#[test]
fn a_path_outside_the_validity_period_is_refused_at_either_end() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::SIGNER),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    // Section 6.1.3 (a)(2), "[t]he certificate validity period includes the current time", asked
    // at two instants the fixture chain is not current at. The later one is inside the
    // intermediate's period and outside the signer's, so it is the *target* that fails; the
    // earlier one is outside both.
    // The counts are the clause's order rather than a recorded output: the walk processes the
    // intermediate first, so at the later instant it passes and the signer is the second
    // certificate looked at, while at the earlier one the intermediate itself is already not
    // current and the walk stops at the first.
    for (at, examined, when) in [
        (AFTER_THE_SIGNER_EXPIRED, 2, "after the signer expired"),
        (BEFORE_ANY_OF_THEM, 1, "before any of them began"),
    ] {
        assert_eq!(
            validate(
                &certificate(&leaf),
                &[certificate(&mid)],
                &anchors,
                &Material::none(),
                at
            ),
            Trust::Refused {
                refusal: PathRefusal::NotCurrent,
                examined,
            },
            "a path validated {when} is not current"
        );
    }
}

#[test]
fn a_critical_extension_this_reader_does_not_know_is_a_refusal() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::SIGNER_CRITICAL),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    // RFC 5280 section 4.2: "A certificate-using system MUST reject the certificate if it
    // encounters a critical extension it does not recognize". This is also what makes the omitted
    // steps of section 6.1 safe, so the assertion names the identifier rather than only the
    // variant: a refusal that could not say *which* extension would not carry that argument.
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[certificate(&mid)],
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::Refused {
            refusal: PathRefusal::UnrecognisedCriticalExtension("1.3.6.1.4.1.99999.1".to_owned()),
            // The intermediate passes and the signer is the second certificate looked at.
            examined: 2,
        },
    );
}

#[test]
fn a_certificate_that_is_not_a_certification_authority_may_not_sign_one() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::NOT_A_CA),
        hex(fixtures::NOT_A_CA_SIGNER),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    // Section 6.1.4 (k): "verify that the basicConstraints extension is present and that cA is set
    // to TRUE". Everything else about this chain is in order — the signatures verify, the dates
    // are current — so the `cA FALSE` is the only thing that can produce this answer.
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[certificate(&mid)],
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::Refused {
            refusal: PathRefusal::NotACertificationAuthority,
            // Refused while preparing for the certificate below it, so one was looked at.
            examined: 1,
        },
    );
}

#[test]
fn a_key_usage_that_forbids_signing_certificates_is_obeyed() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::NO_CERT_SIGN),
        hex(fixtures::NO_CERT_SIGN_SIGNER),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    // Section 6.1.4 (n): "If a key usage extension is present, verify that the keyCertSign bit is
    // set." This certificate asserts `digitalSignature` and `cRLSign` and not that one, which is
    // exactly the case that would pass a reader that looked only at `basicConstraints`.
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[certificate(&mid)],
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::Refused {
            refusal: PathRefusal::KeyUsageForbidsCertificateSigning,
            // Refused at the same step as the case above, and for the other of its two reasons.
            examined: 1,
        },
    );
}

#[test]
fn a_path_longer_than_a_path_length_constraint_permits_is_refused() {
    let (root, mid, deep, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::DEEP),
        hex(fixtures::DEEP_SIGNER),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    // Section 6.1.4 (m) lowers `max_path_length` to the intermediate's `pathLenConstraint` of
    // zero, and (l) then refuses the next non-self-issued certificate. Four certificates in the
    // hierarchy and one constraint decide it.
    assert_eq!(
        validate(
            &certificate(&leaf),
            &[certificate(&mid), certificate(&deep)],
            &anchors,
            &Material::none(),
            WITHIN,
        ),
        Trust::Refused {
            refusal: PathRefusal::PathLengthExceeded,
            // The intermediate passes; the CA below it is the second certificate looked at and is
            // one more than its `pathLenConstraint` of zero permits.
            examined: 2,
        },
    );
}

#[test]
fn the_same_certificate_cannot_be_walked_twice() {
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::SIGNER),
    );
    let anchors = TrustAnchors::of(&[certificate(&root)]);
    // RFC 5280 section 6.1: "A certificate MUST NOT appear more than once in a prospective
    // certification path." The same intermediate offered eight times is what a file would carry to
    // make the search do eight times the work; the answer and the count of what was examined are
    // both unchanged by the repetition.
    let repeated: Vec<Certificate<'_>> = std::iter::repeat_n(certificate(&mid), 8).collect();
    assert_eq!(
        validate(
            &certificate(&leaf),
            &repeated,
            &anchors,
            &Material::none(),
            WITHIN
        ),
        Trust::Anchored {
            length: 2,
            revocation: Revocation::NotChecked,
        },
    );
}

#[test]
fn the_search_stops_at_its_own_depth_bound() {
    // A self-issued CA offered as its own issuer would chain forever if nothing bounded it, and
    // the fixture root is exactly that shape: its subject and issuer are one name. Offered as a
    // *candidate* rather than as an anchor, it is a cycle the search has to decline — and it does,
    // because a certificate may not appear twice and the anchor set is empty of anyone it leads
    // to. The point of the test is that this returns at all.
    let (root, mid, leaf) = (
        hex(fixtures::ROOT),
        hex(fixtures::INTERMEDIATE),
        hex(fixtures::SIGNER),
    );
    let leaf = certificate(&leaf);
    let mut anchors = TrustAnchors::none();
    anchors.push(TrustAnchor::of(&leaf));
    assert_eq!(
        validate(
            &leaf,
            &[certificate(&mid), certificate(&root)],
            &anchors,
            &Material::none(),
            WITHIN,
        ),
        Trust::NoPathToAnyAnchor { examined: 0 },
    );
}

#[test]
fn the_fields_a_path_needs_are_read_off_a_certificate() {
    let bytes = hex(fixtures::INTERMEDIATE);
    let parsed = certificate(&bytes);
    // Everything `trust` asks of `x509`, asserted where it is read rather than only where it is
    // used: a path that failed for want of one of these would otherwise be indistinguishable from
    // a path that failed on its merits.
    assert_eq!(parsed.version, 3, "an X.509 v3 certificate");
    assert!(!parsed.indefinite_lengths, "openssl writes DER");
    let constraints = parsed
        .extensions
        .basic_constraints
        .expect("the intermediate states basicConstraints");
    assert!(constraints.ca, "cA is asserted");
    assert_eq!(constraints.path_len, Some(0), "pathLenConstraint is zero");
    let usage = parsed.extensions.key_usage.expect("it states keyUsage");
    assert!(usage.key_cert_sign() && usage.crl_sign());
    assert!(!usage.digital_signature());
    assert!(parsed.extensions.unrecognised_critical.is_none());
    assert!(!parsed.extensions.truncated);
    assert!(
        parsed.extensions.authority_key_identifier.is_some(),
        "it names its issuer's key"
    );
    let validity = parsed.validity.expect("both instants are readable");
    // 2026-01-01T00:00:00Z and 2036-01-01T00:00:00Z, which is what the fixture was issued with.
    assert_eq!(validity.not_before.unix_seconds(), 1_767_225_600);
    assert_eq!(validity.not_after.unix_seconds(), 2_082_758_400);
    assert!(validity.includes(WITHIN));
    assert!(!validity.includes(BEFORE_ANY_OF_THEM));
}
