//! RFC 5280 section 6.3.3 and RFC 6960 section 4.2, over a hierarchy this tree issued.
//!
//! **Test vectors rather than oracles, and here the distinction is sharper than usual.** A
//! positive revocation answer needs a CRL and an OCSP response signed by the key that issued the
//! certificate they are about, so it needs a hierarchy whose *private* keys somebody here held —
//! which no corpus document can supply and no real authority will. The RFCs define the structures
//! and the algorithm; what these vectors pin is that this tree walks and computes them.
//!
//! **And the corpus is the other half, not a substitute for it.** No document in `doc/pdf.js`
//! carries a document security store at all — `signatures.rs`'s census is the command that says so
//! — which is exactly the position trap 13 is about: a sweep that finds nothing reads identically
//! as "the reader is right" and as "the reader was never called. So the defect is planted here,
//! one case at a time, and the corpus gate asserts the rule the absence must obey — no `Good`
//! without material.

use super::{
    CertStatus, Evidence, Material, MaterialRefusal, Revocation, RevocationReason, Subject,
    Undetermined, certificate_list, ocsp_response, status, worst,
};
use crate::trust::{Trust, TrustAnchors, validate};
use crate::x509::{Certificate, Instant, parse};

/// 2026-10-01T00:00:00Z — inside every fixture's validity period, after the CRL's `thisUpdate`
/// and after the OCSP responses were produced.
const AT: Instant = Instant::from_unix_seconds(1_790_812_800);

/// 2046-10-01T00:00:00Z — past the CRL's `nextUpdate` and past every certificate's `notAfter`.
const LONG_AFTER: Instant = Instant::from_unix_seconds(2_422_281_600);

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

/// The subject one leaf makes, with the CA as its issuer — what [`crate::trust`] assembles as it
/// walks a path, built by hand here so that one certificate can be asked about at a time.
fn subject<'a>(leaf: &'a Certificate<'a>, ca: &'a Certificate<'a>) -> Subject<'a, 'a> {
    Subject {
        certificate: leaf,
        issuer_name: ca.subject_encoding,
        issuer_key_bits: ca.public_key_bits,
        issuer_key: ca.public_key,
        issuer_key_usage: ca.extensions.key_usage,
        position: 0,
    }
}

/// One byte of a signature moved, which is what a forgery looks like from here.
fn with_a_moved_signature(bytes: &[u8]) -> Vec<u8> {
    let mut moved = bytes.to_vec();
    let last = moved.len().saturating_sub(1);
    if let Some(byte) = moved.get_mut(last) {
        *byte ^= 0x01;
    }
    moved
}

mod fixtures {
    //! Built once with `openssl` and pasted in, on the footing this module's own comment
    //! states.

    /// The anchor of this module's hierarchy: a self-signed CA asserting `keyCertSign` and
    /// `cRLSign`, 2026 to 2036.
    pub(super) const CA: &str = "\
        308203273082020fa00302010202020100300d06092a864886f70d01010b0500\
        30243122302006035504030c1971756f727261207265766f636174696f6e2074\
        657374204341301e170d3236303130313030303030305a170d33363031303130\
        30303030305a30243122302006035504030c1971756f727261207265766f6361\
        74696f6e207465737420434130820122300d06092a864886f70d010101050003\
        82010f003082010a0282010100b7a6790d52847b96814f886a4c2e6ddc058272\
        8e2af44de3a631406bc2a728ec6d5b6890e0ef1c6e469e0169a95b073395575a\
        ea37f1b4099c9226312f841316e8ccd249e562f8a48428002d94e6348f9ff1e7\
        f04b3e4ef7ee8b8c6954a4d848e52a4720f91fd700e1c0bd97423124cb121ef3\
        b86204744a23b2004f88e9d8305a1a26f8e3431c5adddfe815f0c71e9bf04251\
        bf2271c18cb480d764082c9813aa081756e7a5a17bbe4c7124a8167e14f1fcea\
        71b900dba73beabb2809c4a5874c53d7f53ef99ac4070bfbbf1c256e6c983322\
        1e72a93345091332fe83c42a0f77463fd358e53352a685bda49a3e6bfb40e132\
        04f92df14d2a9120769033e3250203010001a3633061301d0603551d0e041604\
        149763c035db60226aab36f04fa7e841087c82b243301f0603551d2304183016\
        80149763c035db60226aab36f04fa7e841087c82b243300f0603551d130101ff\
        040530030101ff300e0603551d0f0101ff040403020106300d06092a864886f7\
        0d01010b050003820101003d6057f60aa336cfd10e2691642858121582a27d94\
        709ad8695e81b9cdc93d7d4abfd2e9888a66dc4845fa43952d98da760990077a\
        9b4ac14f528da091625a6668e23620dcd581d2afd48d9695c7e53402967ef78f\
        ab9893f09b168fbaaff997559973cb1c832fc61f1e384e7dcfc59892c7c22908\
        03259a7c2b893bfd972065427690d45165b8037adc6a6cfc7836e20cdbfb2b4a\
        5c56bac62acaeef852db6220b059ce695d6d5621ae25b07d78baeff1a937632d\
        6825b7b000b7542134e7b9bdfc4dfa19184392469d86d66d8cdd3ba90674452a\
        e59cd40df518a4cf45b40385d504c99ede4ed039eefd0478508c6daa2c2b03cd\
        31016d0d3e9199a79c24f2";

    /// A leaf this CA issued with serial `2A`, and revoked for `keyCompromise`.
    pub(super) const REVOKED_LEAF: &str = "\
        308202ff308201e7a00302010202012a300d06092a864886f70d01010b050030\
        243122302006035504030c1971756f727261207265766f636174696f6e207465\
        7374204341301e170d3236303130313030303030305a170d3336303130313030\
        303030305a301e311c301a06035504030c1371756f727261207265766f6b6564\
        206c65616630820122300d06092a864886f70d01010105000382010f00308201\
        0a0282010100be16bba9bfd099e7d09994196a1d45b7fc5a9cdecb9071d61401\
        a011dd4c034528079b6545933c46cecf1cf8021e3488105d296be4e3486c1d2d\
        56fbb368569a6c3bc0b13acbd38086c32bb096ccd15c2fd669a1b17aa465451e\
        b8f2ebd93eede86c4cb80042248133b192bec5efd59bad30b40dc567c8f106d4\
        a37c1dfaac6e7a5592ec42b97d41247e4c9c5a3ab994566e31c42e06d0dcec3e\
        dec4788d31a815bbd25c8ea2b8bc977058f6e45d8f977020dbe187ed19f78359\
        fd2fa1c3249ec8209f2e4eb7bdf1c2a078f8a16fa773cb2b9cc174e380ec0669\
        3f2f6e3d1f495a18d55e762a4e1b76a4361f79b62a1d1c09346f4425e3717c85\
        1c5fc2257ac90203010001a3423040301d0603551d0e0416041428152a4a936d\
        5a970b75cd03c9d160a71a8f19d3301f0603551d230418301680149763c035db\
        60226aab36f04fa7e841087c82b243300d06092a864886f70d01010b05000382\
        0101001e633c2e1275609eef5f06a4f12cef4aabacefaf4dab101c1ddc8ae624\
        673268d0fd5ca87267c8b0e44d266cd44006e26c3cae6525764b257f876cfef2\
        3faded5b80515ef7e85623847bcfd1d7131d2dc434701d7ec0d68da6cfdaf204\
        65093febfe85ebdfe8f21851dc1dee1ebfff8a60f42149607568f9b8c9792c7f\
        e7193bc74a6413e289a802abd52269a3ce4531251425575890bc6ff24550e19c\
        43ec3523964f3d0b864c430867ab8c611808ff7b85e2319aef174c42df70b0b8\
        da3e29e953dc0b6e4c7a27453941c8f7b6e7f707cd8572e6930bbc159fb00e7e\
        8bdd1d12aed5f367bb178809d66cf1e11fb6ab2a385c5dc5c88ebcbeda0bf359\
        6d7d04";

    /// A leaf this CA issued with serial `2B`, and never revoked.
    pub(super) const GOOD_LEAF: &str = "\
        308202fc308201e4a00302010202012b300d06092a864886f70d01010b050030\
        243122302006035504030c1971756f727261207265766f636174696f6e207465\
        7374204341301e170d3236303130313030303030305a170d3336303130313030\
        303030305a301b3119301706035504030c1071756f72726120676f6f64206c65\
        616630820122300d06092a864886f70d01010105000382010f003082010a0282\
        010100b64a451fa0d1de400b66977d580592e93eaa8a404b74ed26dc22d6b56a\
        3c2f724144b72c33d7a1d2935fcbe1ddda2f7c2c0be58872dcea62eb1fc28382\
        4d121be3a5d7571fcb9f347d5cef723bbbe364b7b267de9353558f4c89c92848\
        fd2f0b3ae553f76d2933925737564e4fd4b6a98f5d303a0846e8f6a7f4b060ec\
        968eac308a9b2d941122ba10648b6c979b9448d0da9e68a944e084f6a9695bd3\
        5e17693dbea90707f89f9c081425e0324744ec71144f299cbfa4c1ea46c35049\
        58e3ddb55ace15f91b04d76f7ea49b1b14d14d7079705ad968af045f4a387cd8\
        b124100a02652cc8b1a2f309d41888e45dd4bbbd0b87eacea556455f5a07a7f9\
        850fd30203010001a3423040301d0603551d0e04160414e403a936b46ffc6602\
        7e10c046d4a965766b9acc301f0603551d230418301680149763c035db60226a\
        ab36f04fa7e841087c82b243300d06092a864886f70d01010b05000382010100\
        6ec3bdc1a1ed3cf66a28b64ed87b60645ba0df1f2ca0cca4fc7530baec1cbb8e\
        ad0c388b7271a6e60a7b73157c5bd56368ccc1516945e4362f9025f77ae49987\
        3237a695ce19097438cd08bbeb2b15983f79446b6996141323194e32dbd520d6\
        9aef6ff3ef26bc86466b676bb509fe435e9d6c294a310c8335403bd074e759bb\
        bd56ee64bfd91b615c84b6effd04d00cecafd71cb82e21b36001fd4df77db717\
        be8c788662ee756efd1a955cb9a818ce6e35e9f2973997bdc74644920d29a4c7\
        cef297ef1fad1ee66e1710cf0a16e1b748c9bb64c57038c2df6112c477a47ba8\
        673ae38a5556fc576bef3feb6d9dbddb85f6aaf99f25fb24d59f0e253130cd26";

    /// The CA's CRL, `thisUpdate` 2026-06-01 and `nextUpdate` 2036-06-01, listing serial `2A`.
    pub(super) const CRL: &str = "\
        308201a130818a020101300d06092a864886f70d01010b050030243122302006\
        035504030c1971756f727261207265766f636174696f6e207465737420434117\
        0d3236303630313030303030305a170d3336303630313030303030305a302230\
        2002012a170d3236303931343230303930325a300c300a0603551d1504030a01\
        01a00e300c300a0603551d140403020101300d06092a864886f70d01010b0500\
        03820101007b0e5b475305ad9ad5fd1e4c853cb28b92299205bdfa587a1dbe61\
        e56b0be52caeb6a87694c76c73b76eb7989254d48f2eb76a400aa5c053d1db3d\
        1cd9e9fbd3c051f73f8902716842281706ec0a805a2571441661ede7cd3931b3\
        79dd5c22e0cadc4f32877897f1b94658085ca6c96790e8b9554d0695dc093e18\
        7fa1d721fa647a5c84d25cc5d299222ad9ada7c23ce85a1188bd748756d6bcf1\
        29428f5634351d0e14fd3c33faa600bc3c8de8ad64096232cb93e65f326c0d84\
        9bdf3530badedc48b64aa20c3c54e711b661018d169cab37f6a9c9b4fa21e9b9\
        074d66519ff60dc940a4e4853b09f8538093aa85cf73f5ba5ff96d9f41e64365\
        951f943115";

    /// An OCSP response about serial `2A`, signed by the CA itself — RFC 6960
    /// section 4.2.2.2 criterion 2.
    pub(super) const OCSP_REVOKED_BY_CA: &str = "\
        3082051d0a0100a08205163082051206092b0601050507300101048205033082\
        04ff3081b5a12630243122302006035504030c1971756f727261207265766f63\
        6174696f6e2074657374204341180f32303236303931343230303931345a307a\
        3078303a300906052b0e03021a05000414b7aaa6271dadd8f2d0bb7351e62f61\
        bfa369dc4904149763c035db60226aab36f04fa7e841087c82b24302012aa116\
        180f32303236303931343230303930325aa0030a0101180f3230323630393134\
        3230303931345aa011180f32303336303931313230303931345a300d06092a86\
        4886f70d01010b050003820101005a9e935925981245e75f7891bf6947f06dc5\
        11c9e9207b292e28c955bb8eae32918bafc83fb44ce039e65b1d9fa3c4d342d0\
        ab96a3880590e65db6b5ec1f20e9b7985d2aa7923bbc5e2aff96c2b30017413b\
        8edd65f2801eb65a5f3f93efbd47b768213d3fbad43aa6022ebc32e9403eb283\
        71071ff8187f709b93e5de0f4c4e6e088ab071957fcc069a04c1f9b3a0be9384\
        75f5c81bbd3ef6712dd413bb2c596e91b15494b89bb6921a68d44d7523aa9705\
        f5b6a4abf11d8bdc7294eba77df4932dfdfbaba3f53b0f1ddab9f43460d66362\
        437422d1712a7038e408e30ee69529ec76f522bfc460bd048f62cce28c25c4a9\
        446aab4006981284e93c83a37282a082032f3082032b308203273082020fa003\
        02010202020100300d06092a864886f70d01010b050030243122302006035504\
        030c1971756f727261207265766f636174696f6e2074657374204341301e170d\
        3236303130313030303030305a170d3336303130313030303030305a30243122\
        302006035504030c1971756f727261207265766f636174696f6e207465737420\
        434130820122300d06092a864886f70d01010105000382010f003082010a0282\
        010100b7a6790d52847b96814f886a4c2e6ddc0582728e2af44de3a631406bc2\
        a728ec6d5b6890e0ef1c6e469e0169a95b073395575aea37f1b4099c9226312f\
        841316e8ccd249e562f8a48428002d94e6348f9ff1e7f04b3e4ef7ee8b8c6954\
        a4d848e52a4720f91fd700e1c0bd97423124cb121ef3b86204744a23b2004f88\
        e9d8305a1a26f8e3431c5adddfe815f0c71e9bf04251bf2271c18cb480d76408\
        2c9813aa081756e7a5a17bbe4c7124a8167e14f1fcea71b900dba73beabb2809\
        c4a5874c53d7f53ef99ac4070bfbbf1c256e6c9833221e72a93345091332fe83\
        c42a0f77463fd358e53352a685bda49a3e6bfb40e13204f92df14d2a91207690\
        33e3250203010001a3633061301d0603551d0e041604149763c035db60226aab\
        36f04fa7e841087c82b243301f0603551d230418301680149763c035db60226a\
        ab36f04fa7e841087c82b243300f0603551d130101ff040530030101ff300e06\
        03551d0f0101ff040403020106300d06092a864886f70d01010b050003820101\
        003d6057f60aa336cfd10e2691642858121582a27d94709ad8695e81b9cdc93d\
        7d4abfd2e9888a66dc4845fa43952d98da760990077a9b4ac14f528da091625a\
        6668e23620dcd581d2afd48d9695c7e53402967ef78fab9893f09b168fbaaff9\
        97559973cb1c832fc61f1e384e7dcfc59892c7c2290803259a7c2b893bfd9720\
        65427690d45165b8037adc6a6cfc7836e20cdbfb2b4a5c56bac62acaeef852db\
        6220b059ce695d6d5621ae25b07d78baeff1a937632d6825b7b000b7542134e7\
        b9bdfc4dfa19184392469d86d66d8cdd3ba90674452ae59cd40df518a4cf45b4\
        0385d504c99ede4ed039eefd0478508c6daa2c2b03cd31016d0d3e9199a79c24\
        f2";

    /// An OCSP response about serial `2B`, signed by the CA itself.
    pub(super) const OCSP_GOOD_BY_CA: &str = "\
        308205070a0100a0820500308204fc06092b0601050507300101048204ed3082\
        04e930819fa12630243122302006035504030c1971756f727261207265766f63\
        6174696f6e2074657374204341180f32303236303931343230303931345a3064\
        3062303a300906052b0e03021a05000414b7aaa6271dadd8f2d0bb7351e62f61\
        bfa369dc4904149763c035db60226aab36f04fa7e841087c82b24302012b8000\
        180f32303236303931343230303931345aa011180f3230333630393131323030\
        3931345a300d06092a864886f70d01010b05000382010100480e9df610a792a7\
        3664c222553cb839c64d890b67920f7a921225fc0adf44195162c8cdb4bb7321\
        2d82b0bd47da3741cc15e13bd99e1c16838fbbbc00498c03a13111f36f139c46\
        6036eec06e26d3a3ebe3951070099a658255802c3e23f2c1a1de4e57cd0f9750\
        4253bdf3d2a421ae7e0b8171d19243cb65b3e7933070e0e60f3394f67ff085e2\
        0ecb5bcb1c64c68c54d81ab29798f7a01fa1d37dfc8fd5662d0891fcac20877b\
        8d5b762a610e28292f6b092e89b39125803d8d3737b7ae3881b159c8357c75a6\
        28ef4ab557efed7cc0ff0e325650e0fd60258b6decf829afb29ba65e251b7fa1\
        624440b6df24dd947b7d8a58de93484ba4b83242f8ef9545a082032f3082032b\
        308203273082020fa00302010202020100300d06092a864886f70d01010b0500\
        30243122302006035504030c1971756f727261207265766f636174696f6e2074\
        657374204341301e170d3236303130313030303030305a170d33363031303130\
        30303030305a30243122302006035504030c1971756f727261207265766f6361\
        74696f6e207465737420434130820122300d06092a864886f70d010101050003\
        82010f003082010a0282010100b7a6790d52847b96814f886a4c2e6ddc058272\
        8e2af44de3a631406bc2a728ec6d5b6890e0ef1c6e469e0169a95b073395575a\
        ea37f1b4099c9226312f841316e8ccd249e562f8a48428002d94e6348f9ff1e7\
        f04b3e4ef7ee8b8c6954a4d848e52a4720f91fd700e1c0bd97423124cb121ef3\
        b86204744a23b2004f88e9d8305a1a26f8e3431c5adddfe815f0c71e9bf04251\
        bf2271c18cb480d764082c9813aa081756e7a5a17bbe4c7124a8167e14f1fcea\
        71b900dba73beabb2809c4a5874c53d7f53ef99ac4070bfbbf1c256e6c983322\
        1e72a93345091332fe83c42a0f77463fd358e53352a685bda49a3e6bfb40e132\
        04f92df14d2a9120769033e3250203010001a3633061301d0603551d0e041604\
        149763c035db60226aab36f04fa7e841087c82b243301f0603551d2304183016\
        80149763c035db60226aab36f04fa7e841087c82b243300f0603551d130101ff\
        040530030101ff300e0603551d0f0101ff040403020106300d06092a864886f7\
        0d01010b050003820101003d6057f60aa336cfd10e2691642858121582a27d94\
        709ad8695e81b9cdc93d7d4abfd2e9888a66dc4845fa43952d98da760990077a\
        9b4ac14f528da091625a6668e23620dcd581d2afd48d9695c7e53402967ef78f\
        ab9893f09b168fbaaff997559973cb1c832fc61f1e384e7dcfc59892c7c22908\
        03259a7c2b893bfd972065427690d45165b8037adc6a6cfc7836e20cdbfb2b4a\
        5c56bac62acaeef852db6220b059ce695d6d5621ae25b07d78baeff1a937632d\
        6825b7b000b7542134e7b9bdfc4dfa19184392469d86d66d8cdd3ba90674452a\
        e59cd40df518a4cf45b40385d504c99ede4ed039eefd0478508c6daa2c2b03cd\
        31016d0d3e9199a79c24f2";

    /// An OCSP response about serial `2B`, signed by a responder the CA issued with
    /// `id-kp-OCSPSigning` — RFC 6960 section 4.2.2.2 criterion 3, with the delegation certificate
    /// inside the response's `certs`.
    ///
    /// **The responder's key is 3072-bit where the CA's is 2048-bit, and that is the point of the
    /// fixture rather than an accident.** Criterion 2 is tried first, and against a 384-octet
    /// signature the CA's 256-octet modulus is not merely wrong — the arithmetic cannot be set up
    /// at all, so it refuses by *name*. A reader that treated that refusal as the end of the
    /// search would report "this program does not verify 1.2.840.113549.1.1.11" about an algorithm
    /// it verifies, which is what two real documents in the crawl were told before this was
    /// written down.
    pub(super) const OCSP_GOOD_BY_DELEGATE: &str = "\
        308205f20a0100a08205eb308205e706092b0601050507300101048205d83082\
        05d430819ba1223020311e301c06035504030c1571756f727261206f63737020\
        726573706f6e646572180f32303236303931343230323135305a30643062303a\
        300906052b0e03021a05000414b7aaa6271dadd8f2d0bb7351e62f61bfa369dc\
        4904149763c035db60226aab36f04fa7e841087c82b24302012b8000180f3230\
        3236303931343230323135305aa011180f32303336303931313230323135305a\
        300d06092a864886f70d01010b050003820181003693560dc88008893417dbdf\
        d4498aeea2b3306e83430ef8b2f389be2c515db4c7c9bcea71c42e6cf2eb20d6\
        41e3b715dc9c72175091b5409b9a1fca9f7e9e0c1721cf4f27b5cd8c311171f2\
        adf8e400a09416f6c695f602481686c99d01fbdc2904b1ac881d4045b4b378b2\
        068859c55dc5076d5fa7167661392019c0a2c18bc49c3b909cf8166694ea7fd1\
        fba02126a15ef7294179538bf8ac1acdf52a369408398fad52f70b18df8f5d38\
        2a7f797c427ed4d226898421736fd735f3c89af884b446b2ad694a6f720472a6\
        d66f6b3b5a43de0dde6e1163f9fcfe855d3d2e060683a16ac17de7ba9bd8d0e5\
        a0dd241919106daa8b8abe0331ca8a11be2218aa7c1a87e14ec7f39028f90273\
        673488569409696dc44b4ce12f2d109b10fd273c0b679a0a1b456e10eaed99aa\
        094375dae9cc26bdc492185c391173643f99301c9ff6e4b8bb4da1486e2616c3\
        7228842736411def2d667f0f1e6c0e727a6ae3043bead1ef4cd3804ab52a3af2\
        d188cb1d2234449cb894ffa5512e95bef26085c6a082039e3082039a30820396\
        3082027ea00302010202012d300d06092a864886f70d01010b05003024312230\
        2006035504030c1971756f727261207265766f636174696f6e20746573742043\
        41301e170d3236303130313030303030305a170d333630313031303030303030\
        5a3020311e301c06035504030c1571756f727261206f63737020726573706f6e\
        646572308201a2300d06092a864886f70d01010105000382018f003082018a02\
        82018100dc5f798404bdd5413ba02076f746fa2b4954ab58571f27be58a78e19\
        c8d53e5c60c9eb0fea05a949dc81716cf19bb59bf20a42dab36350a3dd53224e\
        27af1ce9ccab3a4d510a854e30229b8961d4d1b837b9b438bda8143abd2556e4\
        ab121d891cd58bcc9eaeefd10297b67b5ea64dafc5015deb45215098680eff8f\
        a94499aa0b8525fca2517b645b2c11a434b27308b10694b9427f7cf2a12fcc60\
        3d863116df2977ff3cf690358d114376a027ddf9f5e91096e4406ba09d62a50e\
        c1085bae60ba4c6c0bb4ba5655838591c11251797d5f4d5219f0f367d16487cb\
        c9c542ebf1b0a363269614d3c58a5f14ce9512a77181c5b87ea87e469a1cba9e\
        17d66c4418445664daa3a055d20f81c2a6c1846116dac79eda880aeb56835c57\
        8991de84835f317f95d30cc1396654aa0754eacce1f3d705835652b7ee02c53e\
        9bde48cf57f2c83d7c469989570e3624ecc36aadf41ead5513b3df557731a8ab\
        ae8a2b32f3967f3890ed4d47fde8c20dfa1855503424cde2b1213b673e0255f7\
        117395350203010001a357305530130603551d25040c300a06082b0601050507\
        0309301d0603551d0e04160414ab609a648922c5b3fdc193994eddd4dfe17d24\
        44301f0603551d230418301680149763c035db60226aab36f04fa7e841087c82\
        b243300d06092a864886f70d01010b05000382010100374d5af77417454a6f51\
        a32c4e7c75a10e992843d2f459d755e7a6051c6cbf6012b3a742685ed5388281\
        9a5832892b6978c5bc46a06d2f6e04d1ea7f1c9851ce4210d3393d5d9b565b39\
        68ce7328edc3d0a72f8779bb9578826776c7274f29cf004ad7bda0232f919f81\
        e48b2a3c9fe32043fc652c921f7c95c302f1d6f8f07554382fa952c1dad35aa0\
        f1610308a47b2cd82d5923746a39c39b752c13d9cb8686583f7fad8ee34ac098\
        d9fb8cabc0531ddf769b773b6755c65b0fe96d260ff7ecb5204a647b618829da\
        1df28b8b04af1915b8a79c6665cdb265ae115878aad528c5c8ac2e2cbc3afed8\
        2cf2f512b839fff1a8eabc6db7781219867af94dd333";
}

#[test]
fn a_certificate_list_states_its_issuer_its_window_and_its_entries() {
    let bytes = hex(fixtures::CRL);
    let list = certificate_list(&bytes).expect("a CRL this tree issued reads");
    let ca = hex(fixtures::CA);
    let ca = certificate(&ca);
    assert_eq!(
        list.issuer, ca.subject,
        "section 5.1.2.3: the issuer field identifies the entity that signed the CRL"
    );
    // 2026-06-01 and 2036-06-01, which is what the fixture was asked for.
    assert_eq!(list.this_update, Instant::from_unix_seconds(1_780_272_000));
    assert_eq!(
        list.next_update,
        Some(Instant::from_unix_seconds(2_095_891_200))
    );
    assert_eq!(
        list.unrecognised_critical, None,
        "the CRL number is the only extension, and it is neither critical nor unrecognised"
    );
    let entry = list
        .entry_for(&[0x2A])
        .expect("the entries walk")
        .expect("serial 2A is listed");
    assert_eq!(entry.reason, Some(RevocationReason::KeyCompromise));
    assert!(
        list.entry_for(&[0x2B]).expect("the entries walk").is_none(),
        "serial 2B was never revoked"
    );
}

#[test]
fn a_crl_that_lists_a_serial_revokes_the_certificate() {
    let (ca, leaf) = (hex(fixtures::CA), hex(fixtures::REVOKED_LEAF));
    let (ca, leaf) = (certificate(&ca), certificate(&leaf));
    let crl = hex(fixtures::CRL);
    let material = Material::read(&[&crl], &[]);
    assert!(material.refused.is_empty(), "{:?}", material.refused);
    let answer = status(&subject(&leaf, &ca), &material, AT);
    let Revocation::Revoked { reason, from, .. } = answer else {
        panic!("a listed serial is revoked, and this said {answer:?}");
    };
    assert_eq!(reason, Some(RevocationReason::KeyCompromise));
    assert_eq!(from, Evidence::CertificateRevocationList);
}

#[test]
fn a_crl_that_does_not_list_a_serial_says_the_certificate_is_not_revoked() {
    let (ca, leaf) = (hex(fixtures::CA), hex(fixtures::GOOD_LEAF));
    let (ca, leaf) = (certificate(&ca), certificate(&leaf));
    let crl = hex(fixtures::CRL);
    let material = Material::read(&[&crl], &[]);
    assert_eq!(
        status(&subject(&leaf, &ca), &material, AT),
        Revocation::Good {
            from: Evidence::CertificateRevocationList,
            covered: 1,
        }
    );
}

/// The calibration for the sentence above, in the direction that matters.
///
/// Every refusal in this module is also what a broken signature check, a misread date or a serial
/// compared at the wrong offset would produce, so a suite in which nothing ever answers `Good`
/// cannot tell a working reader from one that refuses everything. The test above is that control;
/// these four are the defects planted back into it one at a time (trap 13), and each must move the
/// answer off `Good` and say why.
#[test]
fn four_things_that_must_each_stop_a_crl_from_saying_good() {
    let (ca, leaf) = (hex(fixtures::CA), hex(fixtures::GOOD_LEAF));
    let (ca, leaf) = (certificate(&ca), certificate(&leaf));
    let crl = hex(fixtures::CRL);

    // 1. Nobody supplied any material at all.
    assert_eq!(
        status(&subject(&leaf, &ca), &Material::none(), AT),
        Revocation::Unknown {
            why: Undetermined::NoMaterial,
            position: 0,
        }
    );

    // 2. The list is past its own `nextUpdate`, so "not listed" is a statement about 2036.
    assert_eq!(
        status(
            &subject(&leaf, &ca),
            &Material::read(&[&crl], &[]),
            LONG_AFTER
        ),
        Revocation::Unknown {
            why: Undetermined::Stale,
            position: 0,
        }
    );

    // 3. The list does not verify under the key that issued the certificate.
    let moved = with_a_moved_signature(&crl);
    assert_eq!(
        status(&subject(&leaf, &ca), &Material::read(&[&moved], &[]), AT),
        Revocation::Unknown {
            why: Undetermined::NotUnderIssuersKey,
            position: 0,
        }
    );

    // 4. The issuer's own certificate forbids it to sign lists — section 6.3.3 (f).
    let mut without = subject(&leaf, &ca);
    without.issuer_key_usage = Some(crate::x509::KeyUsage::default());
    assert_eq!(
        status(&without, &Material::read(&[&crl], &[]), AT),
        Revocation::Unknown {
            why: Undetermined::KeyUsageForbidsCrlSigning,
            position: 0,
        }
    );
}

#[test]
fn an_ocsp_response_the_issuer_signed_is_read_and_believed() {
    let (ca, leaf) = (hex(fixtures::CA), hex(fixtures::REVOKED_LEAF));
    let (ca, leaf) = (certificate(&ca), certificate(&leaf));
    let response = hex(fixtures::OCSP_REVOKED_BY_CA);
    let read = ocsp_response(&response).expect("a response this tree produced reads");
    assert_eq!(read.responses.len(), 1);
    assert!(matches!(
        read.responses.first().map(|single| single.status),
        Some(CertStatus::Revoked { .. })
    ));
    let material = Material::read(&[], &[&response]);
    let answer = status(&subject(&leaf, &ca), &material, AT);
    let Revocation::Revoked { from, .. } = answer else {
        panic!("the responder said revoked, and this said {answer:?}");
    };
    assert_eq!(from, Evidence::OcspResponse);

    let good = hex(fixtures::GOOD_LEAF);
    let good = certificate(&good);
    let response = hex(fixtures::OCSP_GOOD_BY_CA);
    assert_eq!(
        status(&subject(&good, &ca), &Material::read(&[], &[&response]), AT),
        Revocation::Good {
            from: Evidence::OcspResponse,
            covered: 1,
        }
    );
}

/// RFC 6960 section 4.2.2.2 criterion 3: a responder the CA delegated to, with the delegation
/// certificate inside the response.
#[test]
fn an_ocsp_response_a_delegate_signed_is_authorised_by_its_extended_key_usage() {
    let (ca, leaf) = (hex(fixtures::CA), hex(fixtures::GOOD_LEAF));
    let (ca, leaf) = (certificate(&ca), certificate(&leaf));
    let response = hex(fixtures::OCSP_GOOD_BY_DELEGATE);
    let read = ocsp_response(&response).expect("a response this tree produced reads");
    assert_eq!(
        read.certificates.len(),
        1,
        "the responder offered its own certificate, which is what makes the delegation checkable"
    );
    assert_eq!(
        status(&subject(&leaf, &ca), &Material::read(&[], &[&response]), AT),
        Revocation::Good {
            from: Evidence::OcspResponse,
            covered: 1,
        }
    );

    // And the same response with one byte moved is not evidence of anything. The byte is the
    // last of the encoding, which is inside the delegation certificate rather than inside the
    // signature — so what this plants is a *delegation* nobody can check, and the answer must
    // still be unknown rather than good.
    let moved = with_a_moved_signature(&response);
    assert!(
        matches!(
            status(&subject(&leaf, &ca), &Material::read(&[], &[&moved]), AT),
            Revocation::Unknown { .. }
        ),
        "a response whose delegation cannot be checked may not answer"
    );
}

#[test]
fn material_that_is_not_der_is_refused_by_name() {
    // X.690 clause 8.1.3.6's indefinite length around nothing: the shortest thing that is BER and
    // is not DER, which Table 261 and RFC 6960 section 4.2.1 each forbid in as many words.
    let indefinite = [0x30, 0x80, 0x00, 0x00];
    assert_eq!(
        certificate_list(&indefinite),
        Err(MaterialRefusal::NotDerEncoded)
    );
    assert_eq!(
        ocsp_response(&indefinite),
        Err(MaterialRefusal::NotDerEncoded)
    );
    let material = Material::read(&[&indefinite[..]], &[&indefinite[..]]);
    assert!(material.is_empty(), "nothing readable came out of it");
    assert_eq!(
        material.refused,
        vec![
            MaterialRefusal::NotDerEncoded,
            MaterialRefusal::NotDerEncoded
        ],
        "and both refusals are named rather than dropped"
    );
}

#[test]
fn the_worst_answer_is_the_one_a_path_reports() {
    let good = Revocation::Good {
        from: Evidence::OcspResponse,
        covered: 1,
    };
    let unknown = Revocation::Unknown {
        why: Undetermined::NotCovered,
        position: 1,
    };
    let revoked = Revocation::Revoked {
        at: AT,
        reason: None,
        from: Evidence::OcspResponse,
        position: 2,
    };
    assert_eq!(worst(good.clone(), unknown.clone()), unknown);
    assert_eq!(worst(unknown.clone(), revoked.clone()), revoked);
    assert_eq!(worst(Revocation::NotChecked, good.clone()), good);
    assert_eq!(
        worst(good.clone(), Revocation::NotChecked),
        good,
        "a real answer is never displaced by the absence of one"
    );
}

/// The whole of it, through the one function a caller uses.
#[test]
fn a_validated_path_carries_what_the_documents_material_said_about_it() {
    let (ca, revoked, good) = (
        hex(fixtures::CA),
        hex(fixtures::REVOKED_LEAF),
        hex(fixtures::GOOD_LEAF),
    );
    let (ca, revoked, good) = (certificate(&ca), certificate(&revoked), certificate(&good));
    let anchors = TrustAnchors::of(std::slice::from_ref(&ca));
    let crl = hex(fixtures::CRL);
    let material = Material::read(&[&crl], &[]);

    assert_eq!(
        validate(&good, &[], &anchors, &material, AT),
        Trust::Anchored {
            length: 1,
            revocation: Revocation::Good {
                from: Evidence::CertificateRevocationList,
                covered: 1,
            },
        }
    );
    let answer = validate(&revoked, &[], &anchors, &material, AT);
    let Trust::Anchored {
        revocation: Revocation::Revoked { reason, .. },
        ..
    } = answer
    else {
        panic!("the CA revoked this certificate, and the path said {answer:?}");
    };
    assert_eq!(reason, Some(RevocationReason::KeyCompromise));

    // And with nothing supplied, the same path says nothing about revocation rather than something
    // reassuring — which is the one rule ADR 1067 exists to fix.
    assert_eq!(
        validate(&good, &[], &anchors, &Material::none(), AT),
        Trust::Anchored {
            length: 1,
            revocation: Revocation::NotChecked,
        }
    );
}
