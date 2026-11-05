//! A time-stamp trust condition and the revocation checks a trust condition mandates, over a
//! hierarchy this project minted for them: a root, an intermediate CA, a time-stamping authority
//! the intermediate issued, a time-stamp token that authority issued over the signature field the
//! CMS fixtures write, and each issuer's CRL and the intermediate's OCSP response. `openssl`
//! issued them once and the DER is pasted in, the footing `crate::trust`'s and
//! `crate::timestamp`'s tests state; no corpus signature states a machine-readable policy (ADR
//! 1709), so every condition here is constructed, and each expected value is read off the clause
//! the test names. Every positive has its negative beside it, changing one thing (trap 13).

use std::borrow::Cow;

use super::{Check, Checked, Kind, KindCheck, Requirement, checks};
use crate::cms;
use crate::policy_rules::certification_path::tests::{hex, hierarchy};
use crate::policy_rules::certification_path::{
    Authorities, Found, MAX_AUTHORITY_SEARCHES, Path, Search, TrustPoint, TrustTrees,
    search_authority,
};
use crate::policy_rules::tests::facts;
use crate::policy_rules::{SignerFacts, Verdict};
use crate::revocation::{Evidence, Material, Undetermined};
use crate::timestamp::Unestablished;
use crate::x509::Instant;

/// 2027-01-01T00:00:00Z: after the token, every CRL and the OCSP response were issued, and inside
/// every certificate's validity.
const AT: Instant = Instant::from_unix_seconds(1_798_761_600);
/// 2026-06-01T00:00:00Z: inside the certificates' validity, before the authority was revoked.
const BEFORE_THE_REVOCATION: Instant = Instant::from_unix_seconds(1_780_272_000);

pub(crate) mod fixtures {
    //! Built once with `openssl` and pasted in; RSA 2048 and SHA-256 throughout.

    /// The self-signed root, `keyCertSign` and `cRLSign`, 2026 to 2036.
    pub(crate) const ROOT: &str = "\
        30820317308201ffa003020102020110300d06092a864886f70d01010b050030\
        2d312b302906035504030c2271756f72726120706f6c6963792074696d652d73\
        74616d70207465737420726f6f74301e170d3236303130313030303030305a17\
        0d3336303130313030303030305a302d312b302906035504030c2271756f7272\
        6120706f6c6963792074696d652d7374616d70207465737420726f6f74308201\
        22300d06092a864886f70d01010105000382010f003082010a0282010100b528\
        adc90c59bae158130282c430113ef81519bf4a2896790fd71e2c97b5cb9d777f\
        616f0f82a1e55f6b0b74627d67fe1d828ba756fc8f08c0ad8abf856891e799ee\
        e27284e51800652cf7887debffd9ff10811b8123201eb3f5e0deaef234538259\
        d32740f791ebb8db0c3e36bd644d4e19dd14db218d6460e16438f559d0c9610c\
        120e505246d820a0f1ce66d48f11bde5dfa49262fe016c8701f75493f26292f2\
        1bd7affb3fa989451368b301467b55608cb6d029dbf14d6f6ef3d145a4f27105\
        a3455963ad9c03505b3ed9354c9300448df02dce92c6568db8a3b8507ee4a709\
        b3b4d308e1328c6a1cd196f221451ff5d71ecb63ee3a9c8f25257501695b0203\
        010001a3423040300f0603551d130101ff040530030101ff300e0603551d0f01\
        01ff040403020106301d0603551d0e04160414e8f97d742d8b4fedcebdb60724\
        9ee0be472e88ff300d06092a864886f70d01010b050003820101001ebe2169d6\
        44d14d4b9b13a66fb879e03ba03b56a97a871a1dd987349f190eddcdf56dadf7\
        dae665e756c56b8be3660299b8531dc38cc23ef2db84b7fa32b3b059a6624e72\
        74ccee5ccae46b28e27612f0a451474e11c6d62c868322507df1b9ec33dd4471\
        182a9470b2f363e5548552185065a9dfe0b76d7006d69436b1c82063b31e9f3d\
        4af0f7b0f5300b1980b8163f9369e88f67d98a5a4a18abde54831be3fb6711ba\
        66de8d91356c2433efdcebb38e6cf21f6479d123663d21afe90f6045812babb6\
        179e6abfd2eb4c2192f6e2cbcec9e3f8f61a901c327b2d110bcc366e8749fa4f\
        341ac77a4db4c00e76f922930a4c678eb8bd7868a118f1a997fcec";

    /// A CA certificate the root issued, `pathLenConstraint` 0.
    pub(crate) const INTERMEDIATE: &str = "\
        308203433082022ba003020102020111300d06092a864886f70d01010b050030\
        2d312b302906035504030c2271756f72726120706f6c6963792074696d652d73\
        74616d70207465737420726f6f74301e170d3236303130313030303030305a17\
        0d3336303130313030303030305a30353133303106035504030c2a71756f7272\
        6120706f6c6963792074696d652d7374616d70207465737420696e7465726d65\
        646961746530820122300d06092a864886f70d01010105000382010f00308201\
        0a0282010100b8ae8b1c6c78c0f9021646fce5c33207a6dde74009f9a73d7d26\
        6877e9496cd685c186c75eb7c957168e96af01aba488d4e41747f4bef42ec46e\
        91abca8188e0e5af357dc5a3b471d7242a54f42d70d4d659fab8789130f2a989\
        4cce803ce89614ac6e1469f0f91b53e4e3100893b303b57c124b91dd71dcce2e\
        255af472f3cae63440e111d81c31b4f36a389c76ccbc7c3e79a0ec5d82246dbd\
        5c3b8f086f79568989e822e71e71f92765f4aed3a66253f1cfff8f81f5876d13\
        7ae2ec00fba1ef9ffc4a9ad526f647fe851faf7d285e4d5679c25c32b376520d\
        8db0b7b963254f63dfc14fd0eff6ef674048a24b94a810622a9c568e7a55f35f\
        a4d29cb45e930203010001a366306430120603551d130101ff040830060101ff\
        020100300e0603551d0f0101ff040403020106301d0603551d0e04160414ca56\
        51c2c01e8dd246f410db5b69d73908dad743301f0603551d23041830168014e8\
        f97d742d8b4fedcebdb607249ee0be472e88ff300d06092a864886f70d01010b\
        050003820101006656d7b83dead31a1292f4dd0348b3accb8ff82e3f8106970a\
        a074c6243a1a819c442234cc6ea91527f52f64efb4d4a5e7083b1d98119722da\
        5feb46a42566d1559fa90fa8249df73934de7ced14b2da67feb3ef2c118b2ef5\
        cd1bd396b5dc2b8acc5d189c07d4835cf2378efd74a7ddaaaa2378e384259407\
        4ccef2fa4e2d2bc78d1e9c3a75177d8db2e16c064ffc785c5ec730bd6dfa4906\
        22df85ec98fe5766eaba6a8f053e179f79555c825477cb0b9cb2b8e4f18d00c3\
        5fbcc89b70a7c307fc461b6861c5bd8e826c4388abc402c6965a1af51b158b58\
        64918a08877d994e20da64a11920c810ad93b6d5e600acb2256bdfe260936d2b\
        c245fc4ce9d056";

    /// The time-stamping authority's certificate, which the intermediate issued, asserting a critical `id-kp-timeStamping` (RFC 3161 section 2.3).
    pub(crate) const AUTHORITY: &str = "\
        3082035a30820242a003020102020112300d06092a864886f70d01010b050030\
        353133303106035504030c2a71756f72726120706f6c6963792074696d652d73\
        74616d70207465737420696e7465726d656469617465301e170d323630313031\
        3030303030305a170d3336303130313030303030305a30323130302e06035504\
        030c2771756f72726120706f6c6963792074696d652d7374616d702074657374\
        20617574686f7269747930820122300d06092a864886f70d0101010500038201\
        0f003082010a0282010100d1c68074e86c4ce46327ad0494bb5da50812f37b5c\
        185c05ed79b07a65e6fa846e946b2aaab3271b493670bf14d99da6cb068d5ec8\
        9fa4e1468a4492c53965f0e85e6380e225877e5d2bcec7fe649fd2c25533444f\
        8ce6e01f51684e121ca21206980060af6fc9c50fe75f19e3ed2c3b318adf5a16\
        ae08b267b1c1c801bad305c81b169f1c70eda305602fd66bcad37621ff986104\
        2be75bb7ec73277747e0a98da5d1d6c60d445d9b0ff71c2901b3be5806eca812\
        e70627813416487fcbb1d814d61d808659d9b379c7fb7b974850825206533485\
        e9f703ce80889282a03cf82c4d88e8fd69a04c52db5e9a3000bc9fa9a71df974\
        c3e0c1361ae266f369a93f0203010001a3783076300c0603551d130101ff0402\
        3000300e0603551d0f0101ff04040302078030160603551d250101ff040c300a\
        06082b06010505070308301d0603551d0e0416041474d9858a4751464b44aca5\
        574e83c74f03719bda301f0603551d23041830168014ca5651c2c01e8dd246f4\
        10db5b69d73908dad743300d06092a864886f70d01010b0500038201010062df\
        58842d046aa1963658855b793c386211e38b000cfa8f4a0861413320239093a2\
        957273e4f6fb21bf2ca0a9ee5b8b203e35202dbdc6b80e91be083f0260a5bd92\
        2e767d7e4be25ccc08a4bc496a1aa198a0f459bef61962caf65d3bf5ad1cdd44\
        81da6dad3135b514e2dbcb1a39446d3fb86d198f404ea0fc905f4af92dc13ce4\
        3b686c1e615ec0dba3ef7e14c3ef936a7787bbc88d4441c56fbc920db2043091\
        6e2c1e557ac2114d5da93e45be4b6d91176247aa2ee036bf1f34bec13b207762\
        bdd2ddbd5577cf441e31a940b3b0b58a7d7f8d6d52e02a2229bb4650d56d2b86\
        84b98ae51337019ac9e9a4ecd3ee4e26b97def5bbbfdd8aa2af1b5255dd0";

    /// The root's CRL, listing nothing, current from issue for ten years.
    pub(crate) const ROOT_CRL: &str = "\
        30820185306f020101300d06092a864886f70d01010b0500302d312b30290603\
        5504030c2271756f72726120706f6c6963792074696d652d7374616d70207465\
        737420726f6f74170d3236313130353231343032395a170d3336313130323231\
        343032395aa00e300c300a0603551d140403020101300d06092a864886f70d01\
        010b05000382010100156de3c4c61833b40dd70dea35e0b7645604f0239ac10d\
        89b3bb76fadf86118d9aa24651dc7eed6faab4761196a6990a2306ed462f9fac\
        13cac40f7ba86d4291df2f95a4e16c117007d36d6237edcdeb9df859942549f8\
        e8e24eb0b091aa78ef78a6a081750aba834985453f6ba10138be30fac2ed21f2\
        4038f90609bc7af756e2da724654e77586a4a19335c981630b22abbf0a57609b\
        c618807a2d6c7db3ff1b87faef2648d4eeddc73daa07995ce9e61e3502a54de1\
        50dcfa26095200f7715f33a5e52c1be8486f398c89c678a5d13d24f7b1ca3f99\
        0677acc8048a17699303de03ecb3f4df0e675405a343a02d5d45c64e324cee3b\
        c26d029276574883ea";

    /// The intermediate's CRL, listing nothing.
    pub(crate) const INTERMEDIATE_CRL: &str = "\
        3082018d3077020101300d06092a864886f70d01010b05003035313330310603\
        5504030c2a71756f72726120706f6c6963792074696d652d7374616d70207465\
        737420696e7465726d656469617465170d3236313130353231343032395a170d\
        3336313130323231343032395aa00e300c300a0603551d140403020101300d06\
        092a864886f70d01010b050003820101002840244042548bb7b351e31303d8a7\
        6d0e4c3c6b9ea9b630d17c39574a890a48abef87e966fd5969717f5f72f2345e\
        b5cdf54bbf849ed8ec98ecca950ef0ab29cb29476b2896e1591223ba6e7c755b\
        b2d1fc1791356a710b39a985d0e519ca391abb47d97674b14391a6df58c53577\
        2d1e58dd5e53be3f0cc3509d3c6dc2c9aaeb36ed21319b970e8d4621cf070e76\
        243e0e33fa1f6063ac22cf387af5e8eac95431878204b256d7eecb6217df638f\
        c5f9759a7d184dca0179b3b9bcc739958fd1b4609c0beb3e80b5052553a8355a\
        4dc785d670fbeaace0f05db96937bb4d8a5c1d68f72814224586c3fce07343a0\
        c2d9d091ba5e7b4afc11ca98faa7000efb";

    /// The intermediate's CRL, listing the authority's serial `12` for `keyCompromise` as of 2026-11-05.
    pub(crate) const INTERMEDIATE_CRL_REVOKED: &str = "\
        308201b230819b020101300d06092a864886f70d01010b050030353133303106\
        035504030c2a71756f72726120706f6c6963792074696d652d7374616d702074\
        65737420696e7465726d656469617465170d3236313130353231343033305a17\
        0d3336313130323231343033305a30223020020112170d323631313035323134\
        3033305a300c300a0603551d1504030a0101a00e300c300a0603551d14040302\
        0102300d06092a864886f70d01010b05000382010100a96a175b45977b0aaba8\
        fa10a7267a406c88dceeeb50510d5235cbbc02985276b673bc661b449e784630\
        75484faea40deab75400fe66c79862b565901708e54b92771a29c431c6c046b0\
        6a6a8bf00c557985419edd9fc017fbe3f81ed1adf9c2273aed32051a18bf64b4\
        49359986ae1a77cc9f13f1fc24ae128b0741bfb980ed9c9d6f6ccaa5572b0117\
        8e023ec36bb4a70465ec1187b49629cd7fe3c377532608453a12499f2c52a3d2\
        8fbba166fa1e4851a6738f3a091fbb4a59cddec6e95f816d065e141398aad14c\
        731cafbbebdf9c1c97a41919b3f90355084163912b4b47d9c4fcf1a11e2ec874\
        531610d0090c4536739142d36d7b943472fd2082165a";

    /// An OCSP response the intermediate signed: the authority's certificate is good.
    pub(crate) const OCSP_GOOD: &str = "\
        308205340a0100a082052d3082052906092b06010505073001010482051a3082\
        05163081b0a13730353133303106035504030c2a71756f72726120706f6c6963\
        792074696d652d7374616d70207465737420696e7465726d656469617465180f\
        32303236313130353231343033305a30643062303a300906052b0e03021a0500\
        041476753c96c02b2d20959ed70cecd303dcb40c21dc0414ca5651c2c01e8dd2\
        46f410db5b69d73908dad7430201128000180f32303236313130353231343033\
        305aa011180f32303336313130323231343033305a300d06092a864886f70d01\
        010b05000382010100140ac2f30d7bc118790442b9fd5bb77fff325df7d1977e\
        5a1724eafcf644db0453100986a1e92f0f9ebc4281cabf34cfdedce4bca9902c\
        f2997c8d1d8f36532ff205674b9d93ad510baef915c22c94a4bdb54afe256012\
        6c461a02b99fb78a9185f44c62bfc3516029bc8869b7e0ffa95fc5e5846bc246\
        acd9affd273cc1e41cc6b3c3dc3a137144eb426a0ebfa48c76e883ba380a5da1\
        f21916d4c35ae29488cf64ce0c07c8d3859dc27aa3c7df722c47f615dc716d67\
        956a674c13e2ae0cc84dc3a38af68079defec1e7940e415aefa19364a838f332\
        0f8dfadf224ae268c2064055a7d1fcfda60ab69511fbf26a1accf3b5b3651fd6\
        9fa5f93ef80102a05ba082034b30820347308203433082022ba0030201020201\
        11300d06092a864886f70d01010b0500302d312b302906035504030c2271756f\
        72726120706f6c6963792074696d652d7374616d70207465737420726f6f7430\
        1e170d3236303130313030303030305a170d3336303130313030303030305a30\
        353133303106035504030c2a71756f72726120706f6c6963792074696d652d73\
        74616d70207465737420696e7465726d65646961746530820122300d06092a86\
        4886f70d01010105000382010f003082010a0282010100b8ae8b1c6c78c0f902\
        1646fce5c33207a6dde74009f9a73d7d266877e9496cd685c186c75eb7c95716\
        8e96af01aba488d4e41747f4bef42ec46e91abca8188e0e5af357dc5a3b471d7\
        242a54f42d70d4d659fab8789130f2a9894cce803ce89614ac6e1469f0f91b53\
        e4e3100893b303b57c124b91dd71dcce2e255af472f3cae63440e111d81c31b4\
        f36a389c76ccbc7c3e79a0ec5d82246dbd5c3b8f086f79568989e822e71e71f9\
        2765f4aed3a66253f1cfff8f81f5876d137ae2ec00fba1ef9ffc4a9ad526f647\
        fe851faf7d285e4d5679c25c32b376520d8db0b7b963254f63dfc14fd0eff6ef\
        674048a24b94a810622a9c568e7a55f35fa4d29cb45e930203010001a3663064\
        30120603551d130101ff040830060101ff020100300e0603551d0f0101ff0404\
        03020106301d0603551d0e04160414ca5651c2c01e8dd246f410db5b69d73908\
        dad743301f0603551d23041830168014e8f97d742d8b4fedcebdb607249ee0be\
        472e88ff300d06092a864886f70d01010b050003820101006656d7b83dead31a\
        1292f4dd0348b3accb8ff82e3f8106970aa074c6243a1a819c442234cc6ea915\
        27f52f64efb4d4a5e7083b1d98119722da5feb46a42566d1559fa90fa8249df7\
        3934de7ced14b2da67feb3ef2c118b2ef5cd1bd396b5dc2b8acc5d189c07d483\
        5cf2378efd74a7ddaaaa2378e3842594074ccef2fa4e2d2bc78d1e9c3a75177d\
        8db2e16c064ffc785c5ec730bd6dfa490622df85ec98fe5766eaba6a8f053e17\
        9f79555c825477cb0b9cb2b8e4f18d00c35fbcc89b70a7c307fc461b6861c5bd\
        8e826c4388abc402c6965a1af51b158b5864918a08877d994e20da64a11920c8\
        10ad93b6d5e600acb2256bdfe260936d2bc245fc4ce9d056";

    /// A time-stamp token the authority issued over the SHA-256 of the octets `DE AD`, which is the signature field `crate::cms::fixtures::detached_with_signature_timestamp` writes, carrying the authority's certificate and the intermediate.
    pub(crate) const TOKEN: &str = "\
        3082094a06092a864886f70d010702a082093b30820937020103310f300d0609\
        60864801650304020105003068060b2a864886f70d0109100104a05904573055\
        02010106042a0304013031300d06096086480165030402010500042059ca84fb\
        79f2a7447b9e82c7412df58c688910cba202b7d4e9bf329ce07f931c02010218\
        0f32303236313130353231343033305a3003020101a08206a53082035a308202\
        42a003020102020112300d06092a864886f70d01010b05003035313330310603\
        5504030c2a71756f72726120706f6c6963792074696d652d7374616d70207465\
        737420696e7465726d656469617465301e170d3236303130313030303030305a\
        170d3336303130313030303030305a30323130302e06035504030c2771756f72\
        726120706f6c6963792074696d652d7374616d70207465737420617574686f72\
        69747930820122300d06092a864886f70d01010105000382010f003082010a02\
        82010100d1c68074e86c4ce46327ad0494bb5da50812f37b5c185c05ed79b07a\
        65e6fa846e946b2aaab3271b493670bf14d99da6cb068d5ec89fa4e1468a4492\
        c53965f0e85e6380e225877e5d2bcec7fe649fd2c25533444f8ce6e01f51684e\
        121ca21206980060af6fc9c50fe75f19e3ed2c3b318adf5a16ae08b267b1c1c8\
        01bad305c81b169f1c70eda305602fd66bcad37621ff9861042be75bb7ec7327\
        7747e0a98da5d1d6c60d445d9b0ff71c2901b3be5806eca812e7062781341648\
        7fcbb1d814d61d808659d9b379c7fb7b974850825206533485e9f703ce808892\
        82a03cf82c4d88e8fd69a04c52db5e9a3000bc9fa9a71df974c3e0c1361ae266\
        f369a93f0203010001a3783076300c0603551d130101ff04023000300e060355\
        1d0f0101ff04040302078030160603551d250101ff040c300a06082b06010505\
        070308301d0603551d0e0416041474d9858a4751464b44aca5574e83c74f0371\
        9bda301f0603551d23041830168014ca5651c2c01e8dd246f410db5b69d73908\
        dad743300d06092a864886f70d01010b0500038201010062df58842d046aa196\
        3658855b793c386211e38b000cfa8f4a0861413320239093a2957273e4f6fb21\
        bf2ca0a9ee5b8b203e35202dbdc6b80e91be083f0260a5bd922e767d7e4be25c\
        cc08a4bc496a1aa198a0f459bef61962caf65d3bf5ad1cdd4481da6dad3135b5\
        14e2dbcb1a39446d3fb86d198f404ea0fc905f4af92dc13ce43b686c1e615ec0\
        dba3ef7e14c3ef936a7787bbc88d4441c56fbc920db20430916e2c1e557ac211\
        4d5da93e45be4b6d91176247aa2ee036bf1f34bec13b207762bdd2ddbd5577cf\
        441e31a940b3b0b58a7d7f8d6d52e02a2229bb4650d56d2b8684b98ae5133701\
        9ac9e9a4ecd3ee4e26b97def5bbbfdd8aa2af1b5255dd0308203433082022ba0\
        03020102020111300d06092a864886f70d01010b0500302d312b302906035504\
        030c2271756f72726120706f6c6963792074696d652d7374616d702074657374\
        20726f6f74301e170d3236303130313030303030305a170d3336303130313030\
        303030305a30353133303106035504030c2a71756f72726120706f6c69637920\
        74696d652d7374616d70207465737420696e7465726d65646961746530820122\
        300d06092a864886f70d01010105000382010f003082010a0282010100b8ae8b\
        1c6c78c0f9021646fce5c33207a6dde74009f9a73d7d266877e9496cd685c186\
        c75eb7c957168e96af01aba488d4e41747f4bef42ec46e91abca8188e0e5af35\
        7dc5a3b471d7242a54f42d70d4d659fab8789130f2a9894cce803ce89614ac6e\
        1469f0f91b53e4e3100893b303b57c124b91dd71dcce2e255af472f3cae63440\
        e111d81c31b4f36a389c76ccbc7c3e79a0ec5d82246dbd5c3b8f086f79568989\
        e822e71e71f92765f4aed3a66253f1cfff8f81f5876d137ae2ec00fba1ef9ffc\
        4a9ad526f647fe851faf7d285e4d5679c25c32b376520d8db0b7b963254f63df\
        c14fd0eff6ef674048a24b94a810622a9c568e7a55f35fa4d29cb45e93020301\
        0001a366306430120603551d130101ff040830060101ff020100300e0603551d\
        0f0101ff040403020106301d0603551d0e04160414ca5651c2c01e8dd246f410\
        db5b69d73908dad743301f0603551d23041830168014e8f97d742d8b4fedcebd\
        b607249ee0be472e88ff300d06092a864886f70d01010b050003820101006656\
        d7b83dead31a1292f4dd0348b3accb8ff82e3f8106970aa074c6243a1a819c44\
        2234cc6ea91527f52f64efb4d4a5e7083b1d98119722da5feb46a42566d1559f\
        a90fa8249df73934de7ced14b2da67feb3ef2c118b2ef5cd1bd396b5dc2b8acc\
        5d189c07d4835cf2378efd74a7ddaaaa2378e3842594074ccef2fa4e2d2bc78d\
        1e9c3a75177d8db2e16c064ffc785c5ec730bd6dfa490622df85ec98fe5766ea\
        ba6a8f053e179f79555c825477cb0b9cb2b8e4f18d00c35fbcc89b70a7c307fc\
        461b6861c5bd8e826c4388abc402c6965a1af51b158b5864918a08877d994e20\
        da64a11920c810ad93b6d5e600acb2256bdfe260936d2bc245fc4ce9d0563182\
        020c30820208020101303a30353133303106035504030c2a71756f7272612070\
        6f6c6963792074696d652d7374616d70207465737420696e7465726d65646961\
        7465020112300d06096086480165030402010500a081a4301a06092a864886f7\
        0d010903310d060b2a864886f70d0109100104301c06092a864886f70d010905\
        310f170d3236313130353231343033305a302f06092a864886f70d0109043122\
        042042dd9f6872f0b273e0d9bc53750627d08d7b062ed7734b607676985dc141\
        259a3037060b2a864886f70d010910022f31283026302430220420345f3624ce\
        f193ce123263d6f903d4572750ba351eb7066812f8d9e42c82b58b300d06092a\
        864886f70d010101050004820100cbdedc30e06c4fc9ccfde1e61ab8be1d992f\
        2f55b2b02395520934572bd547e554181bc3c8eb61d3ae83acec44b8198c5467\
        ac3a8c0f25383365283cc17ff8bdddd9b1a96d6b3d3c479bf901b06a47355214\
        bac3f473db5445c0da79461fd77f1002bed1b34859c1f3ededba225003660a69\
        a44dff94f24aba4ef3d260b4b09d0a633dbd7833386627aa3c56eb6ddd8b9940\
        99ca9daff88c11cd5bae1a2800419c0ddaada930394a84a8d07987f6c020f1ef\
        f3fd4fee23820bb282704e9884c2e90dc4ee36f3a5478f06afe9bd33ffeb6180\
        c1d6a3885e441e4df83d79d99d11fb4cd6b4cc5dbf2505f6a37b078fe3786e53\
        3636459f3e5ae6ce0f511ffdc28a";
}

fn trees(point: &[u8]) -> TrustTrees<'_> {
    TrustTrees {
        points: vec![TrustPoint {
            certificate: Cow::Borrowed(point),
            path_length: None,
            reliable_until: None,
            unprocessed: Vec::new(),
        }],
        trusted_lists: 0,
    }
}

/// A `SignedData` whose `signature-time-stamp` attribute carries `token`, over the signature field
/// `DE AD` that [`fixtures::TOKEN`]'s imprint digests.
fn stamped(token: &[u8]) -> Vec<u8> {
    cms::fixtures::detached_with_signature_timestamp(&[0x00; 32], token)
}

fn at(instant: Instant) -> SignerFacts<'static> {
    facts(None).validated_at(instant)
}

fn authority_search(token: &[u8], point: &[u8], instant: Instant) -> Search {
    let bytes = stamped(token);
    let cms = cms::signed_data(&bytes).expect("a SignedData");
    search_authority(&trees(point), &cms, &at(instant))
}

/// The authority's path, searched to the root at `instant`.
fn authority_path(instant: Instant) -> Path {
    let root = hex(fixtures::ROOT);
    authority_search(&hex(fixtures::TOKEN), &root, instant)
        .path
        .expect("the authority's path validates to the root")
}

fn material<'a>(crls: &[&'a [u8]], ocsps: &[&'a [u8]]) -> Material<'a> {
    Material::read(crls, ocsps)
}

fn require(end_entity: Check, authorities: Check) -> Requirement {
    Requirement {
        end_entity,
        authorities,
        unprocessed: Vec::new(),
    }
}

fn verdicts(found: &[KindCheck; 2]) -> [Verdict; 2] {
    [found[0].verdict(), found[1].verdict()]
}

/// TR 102 272 clause 6.8 and TS 119 172-2 clause 4.25.1 item 3 b): the trust trees are for the
/// path that authenticates the time-stamping authority, which starts at the authority's own
/// certificate. RFC 3161 section 2.3 makes that certificate's extended key usage critical, so the
/// search must state the time-stamping purpose for the path to validate at all.
#[test]
fn the_time_stamping_authoritys_own_path_reaches_the_policys_trust_point() {
    let root = hex(fixtures::ROOT);
    let found = authority_search(&hex(fixtures::TOKEN), &root, AT);
    assert_eq!(
        found.found,
        Found::Reached {
            length: 2,
            unprocessed: Vec::new()
        },
        "the authority and the intermediate, validated under the root"
    );
    assert_eq!(found.verdict(), Verdict::Met);
    let path = found.path.expect("a path that validated is handed out");
    assert_eq!(
        path.certificates().len(),
        3,
        "authority, intermediate, root"
    );
    assert_eq!(
        path.certificates()[0].encoding,
        hex(fixtures::AUTHORITY),
        "the end of the path is the authority's certificate, not the signer's"
    );

    // Another hierarchy's root: no chain from the authority reaches it, which ETSI EN 319 102-1
    // clause 5.2.6.4 calls indeterminate rather than failed.
    let elsewhere = hierarchy();
    let other = authority_search(&hex(fixtures::TOKEN), &elsewhere.root, AT);
    assert_eq!(other.found, Found::NoChain);
    assert_eq!(other.verdict(), Verdict::NotEvaluated);
}

/// What stands before the authority's path: the token must be the signature's, it must verify,
/// and it must be about this signature (ETSI EN 319 122-1 clause 5.3).
#[test]
fn a_token_that_authenticates_nobody_leaves_the_condition_unevaluated() {
    let root = hex(fixtures::ROOT);
    // No attribute at all.
    let bytes = cms::fixtures::detached(&[0x00; 32]);
    let cms = cms::signed_data(&bytes).expect("a SignedData");
    let none = search_authority(&trees(&root), &cms, &at(AT));
    assert_eq!(none.found, Found::NoTimestamp);
    assert_eq!(none.verdict(), Verdict::NotEvaluated);

    // `crate::timestamp`'s own token was minted over other octets entirely.
    let elsewhere = authority_search(&hex(crate::timestamp::tests::fixtures::TOKEN), &root, AT);
    assert_eq!(elsewhere.found, Found::TimestampAboutAnotherSignature);
    assert_eq!(elsewhere.verdict(), Verdict::NotEvaluated);

    // One octet of the token's genTime moved: its attributes no longer commit to its content
    // (RFC 5652 section 5.6), so its signer authenticated nothing.
    let mut moved = hex(fixtures::TOKEN);
    let index = moved
        .windows(8)
        .position(|window| window == b"20261105")
        .expect("genTime inside the encapsulated content");
    moved[index + 3] = b'7';
    let unbound = authority_search(&moved, &root, AT);
    assert_eq!(
        unbound.found,
        Found::TimestampNotVerified(Unestablished::ContentNotBoundToTheSignature)
    );
    assert_eq!(unbound.verdict(), Verdict::NotEvaluated);
}

/// Principle 3: one signature's time-stamp conditions are searched at most
/// [`MAX_AUTHORITY_SEARCHES`] times, and the one past the bound is said, never met.
#[test]
fn time_stamp_conditions_past_the_bound_are_not_searched() {
    let root = hex(fixtures::ROOT);
    let bytes = stamped(&hex(fixtures::TOKEN));
    let cms = cms::signed_data(&bytes).expect("a SignedData");
    let facts = at(AT);
    let authorities = Authorities::default();
    for _ in 0..MAX_AUTHORITY_SEARCHES {
        assert_eq!(
            authorities.search(&trees(&root), &cms, &facts).verdict(),
            Verdict::Met
        );
    }
    let past = authorities.search(&trees(&root), &cms, &facts);
    assert_eq!(past.found, Found::PastSearchBound);
    assert_eq!(past.verdict(), Verdict::NotEvaluated);
}

/// TR 102 272 clause 6.6.2: `clrCheck` for both kinds. The authority's certificate is answered
/// by the intermediate's CRL and the intermediate's by the root's; with the root's CRL missing,
/// the CA certificate's check is not answered, and that is indeterminate, never broken.
#[test]
fn a_crl_check_is_answered_per_kind_of_certificate() {
    let path = authority_path(AT);
    let (root_crl, intermediate_crl) = (hex(fixtures::ROOT_CRL), hex(fixtures::INTERMEDIATE_CRL));
    let both = material(&[&root_crl, &intermediate_crl], &[]);
    let found = checks(
        &require(Check::Crl, Check::Crl),
        Some(&path),
        &both,
        Some(AT),
    );
    assert_eq!(found[0].kind, Kind::EndEntity);
    assert_eq!(found[0].checked, Checked::Answered { certificates: 1 });
    assert_eq!(found[1].kind, Kind::Authority);
    assert_eq!(found[1].checked, Checked::Answered { certificates: 1 });
    assert_eq!(verdicts(&found), [Verdict::Met, Verdict::Met]);

    let leaf_only = material(&[&intermediate_crl], &[]);
    let found = checks(
        &require(Check::Crl, Check::Crl),
        Some(&path),
        &leaf_only,
        Some(AT),
    );
    assert_eq!(found[0].verdict(), Verdict::Met);
    assert_eq!(
        found[1].checked,
        Checked::Unanswered {
            position: 1,
            missing: Evidence::CertificateRevocationList,
            why: Undetermined::NotCovered,
        }
    );
    assert_eq!(found[1].verdict(), Verdict::NotEvaluated);
}

/// Clause 6.6.2's kinds are kinds of *material*: an OCSP response answers `ocspCheck` and not
/// `clrCheck`, `bothCheck` wants the two, and `eitherCheck` takes whichever is there.
#[test]
fn the_kind_of_material_decides_which_check_is_answered() {
    let path = authority_path(AT);
    let (crl, ocsp) = (hex(fixtures::INTERMEDIATE_CRL), hex(fixtures::OCSP_GOOD));
    let end_entity = |check, material: &Material<'_>| {
        checks(
            &require(check, Check::NoCheck),
            Some(&path),
            material,
            Some(AT),
        )[0]
        .clone()
    };
    let responses = material(&[], &[&ocsp]);
    let lists = material(&[&crl], &[]);
    let both = material(&[&crl], &[&ocsp]);
    assert_eq!(end_entity(Check::Ocsp, &responses).verdict(), Verdict::Met);
    assert_eq!(
        end_entity(Check::Crl, &responses).checked,
        Checked::Unanswered {
            position: 0,
            missing: Evidence::CertificateRevocationList,
            why: Undetermined::NotCovered,
        },
        "an OCSP response does not answer a CRL check"
    );
    assert_eq!(end_entity(Check::Both, &both).verdict(), Verdict::Met);
    assert_eq!(
        end_entity(Check::Both, &lists).checked,
        Checked::Unanswered {
            position: 0,
            missing: Evidence::OcspResponse,
            why: Undetermined::NotCovered,
        },
        "bothCheck with a CRL alone lacks the OCSP check"
    );
    assert_eq!(end_entity(Check::Either, &lists).verdict(), Verdict::Met);
    assert_eq!(
        end_entity(Check::Either, &responses).verdict(),
        Verdict::Met
    );
    assert_eq!(
        end_entity(Check::Either, &Material::none()).verdict(),
        Verdict::NotEvaluated
    );
    assert_eq!(
        end_entity(Check::NoCheck, &Material::none()).checked,
        Checked::NotMandated
    );
    assert_eq!(
        end_entity(Check::Other, &both).checked,
        Checked::OtherMechanism,
        "`other` is a policy extension's mechanism, which this program reads none of"
    );
}

/// RFC 5280 section 6.1.3 (a)(3): a check that finds the certificate revoked at the instant
/// breaks the condition — and the same list, asked about an instant before the revocation took
/// effect, answers that the certificate was good then.
#[test]
fn a_mandated_check_that_finds_the_authority_revoked_breaks_the_condition() {
    let revoked = hex(fixtures::INTERMEDIATE_CRL_REVOKED);
    let lists = material(&[&revoked], &[]);
    let path = authority_path(AT);
    let found = checks(
        &require(Check::Crl, Check::NoCheck),
        Some(&path),
        &lists,
        Some(AT),
    );
    assert!(
        matches!(
            found[0].checked,
            Checked::Revoked {
                position: 0,
                from: Evidence::CertificateRevocationList,
                ..
            }
        ),
        "{:?}",
        found[0].checked
    );
    assert_eq!(found[0].verdict(), Verdict::Broken);

    let earlier = authority_path(BEFORE_THE_REVOCATION);
    let found = checks(
        &require(Check::Crl, Check::NoCheck),
        Some(&earlier),
        &lists,
        Some(BEFORE_THE_REVOCATION),
    );
    assert_eq!(found[0].checked, Checked::Answered { certificates: 1 });
    assert_eq!(found[0].verdict(), Verdict::Met);
}

/// Without a validated path the certificates a check covers are not known; with one holding no CA
/// certificate below its trust point, the CA check has nothing to ask and is met.
#[test]
fn a_check_needs_a_path_and_a_ca_check_needs_a_ca() {
    let crl = hex(fixtures::INTERMEDIATE_CRL);
    let lists = material(&[&crl], &[]);
    let found = checks(&require(Check::Crl, Check::Crl), None, &lists, Some(AT));
    assert_eq!(found[0].checked, Checked::NoPath);
    assert_eq!(
        verdicts(&found),
        [Verdict::NotEvaluated, Verdict::NotEvaluated]
    );

    // The intermediate as the trust point: the path is the authority alone.
    let intermediate = hex(fixtures::INTERMEDIATE);
    let short = authority_search(&hex(fixtures::TOKEN), &intermediate, AT)
        .path
        .expect("the authority validates under its issuer");
    let found = checks(
        &require(Check::Crl, Check::Crl),
        Some(&short),
        &lists,
        Some(AT),
    );
    assert_eq!(found[1].checked, Checked::NoCertificate);
    assert_eq!(verdicts(&found), [Verdict::Met, Verdict::Met]);
}
