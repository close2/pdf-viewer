//! ETSI TS 119 615 clauses 4.6 and 4.7, and ETSI TS 119 172-4 REQ-4.5-01 d) i)'s listed
//! authorities, one requirement at a time.
//!
//! Every expected value is the clause's step read against the fixture's construction: a root and
//! three time-stamping units generated for this module with `openssl req` on keys this tree made
//! (`/C=AT/O=r2031 Time Provider`, the same without the organisation, and `/C=US`), a token the
//! first unit issued with `openssl ts -reply`, and lists written by `trusted_list::tests`'
//! writer, whose provider is named `r2031 Time Provider`. Instants are seconds since the epoch.

use super::*;
use crate::trusted_list::tests::{Written, list_xml};
use crate::x509_fixtures::hex;

/// The token's `genTime`, `2026-11-16T01:40:52Z`.
pub(crate) const GEN_TIME: Instant = Instant::from_unix_seconds(1_794_793_252);
/// Four days later, inside every fixture certificate's validity: the instant a question is put.
pub(crate) const ASKED: Instant = Instant::from_unix_seconds(1_795_138_852);
/// `2026-11-18T00:00:00Z`, between the two.
const BETWEEN: &str = "2026-11-18T00:00:00Z";
/// TS 119 612 clause 5.5.4's `withdrawn`.
const WITHDRAWN: &str = "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn";
/// The provider's name, the units' `organizationName`.
pub(crate) const PROVIDER: &str = "r2031 Time Provider";

/// The root that issued the three units: `/C=AT/O=r2031 Time Provider/CN=r2031 TSA root`.
pub(crate) const ROOT: &str = "\
    3082037930820261a003020102021465e10f8c8672db81c52110d13878145c3a\
    7b32cf300d06092a864886f70d01010b05003044310b30090603550406130241\
    54311c301a060355040a0c1372323033312054696d652050726f766964657231\
    17301506035504030c0e72323033312054534120726f6f74301e170d32363131\
    31363031343035305a170d3336313131333031343035305a3044310b30090603\
    55040613024154311c301a060355040a0c1372323033312054696d652050726f\
    76696465723117301506035504030c0e72323033312054534120726f6f743082\
    0122300d06092a864886f70d01010105000382010f003082010a02820101009c\
    68abdf843d7c6388fe01c4481eeb49128cb1b1c94e9836d571650a9520bc097a\
    4205e52fe3d96a7cf7fe0e57f0910ed96643b09a6042b4fa9b12b875beec4bcd\
    de594e2d3b7cae922d33cf2b70d2dd4e390bbd99d2ac08372d1eefca57a0601b\
    2cae86c6c929737ba946a7e47962f801160a554f00ef9ee604fb1b31fc3a9401\
    352346fa37da9a83f7c93257f7eda92a8281faa7492bd9f016b7d644ed57457a\
    b6e48a789786193b4813694a7d6ff66c36dd5c9c1bfd19cbe813b5b23d804eab\
    95f93a19f12faf3dcb8a843bf161e132f785eeffa32e8ff34de88a6880335e24\
    bf7eae5c9bf3b0049b358651043847fb3a84b5f6a2d5fbba5bf0e0a231cd8302\
    03010001a3633061301d0603551d0e0416041480ea130015551ed1fb2eaaeaa8\
    88060534bb7a2e301f0603551d2304183016801480ea130015551ed1fb2eaaea\
    a888060534bb7a2e300f0603551d130101ff040530030101ff300e0603551d0f\
    0101ff040403020106300d06092a864886f70d01010b0500038201010030045e\
    a63036270844502380fb0b331f4b32f28d68882bab33e1b8eaf20fc33ba4936f\
    3fad07c24585df199c5175f5870e66796c4fa7e117b2946c3f969c6d93dbbd75\
    0662b4ef766fc9ba240c912b6f2118863af08e3b4b4262364d01de3fe71d0a29\
    1267df31179b631bf2dbfd2b68d38c29d90cdb9c88e873e55727915845f1e44d\
    e0fd26845a4cdc3926f3fcd3ba878c1efc675129348aa28c809c9131129c4e85\
    78fe84432ee833abf8bbb1e37040e2c8faf1e0e19a3dd87a8fef95e10ae00dad\
    26df2ff3f9683d3519556717380ddb90a97ee045a890b38dccd7e779d9cd406c\
    c71cfa6ba0d8352348c59a602f00422b08d90580a21d76a7505dab1e56";

/// The unit that signed [`TOKEN`]: `/C=AT/O=r2031 Time Provider/CN=r2031 TSU`.
pub(crate) const TSU: &str = "\
    308203773082025fa00302010202022031300d06092a864886f70d01010b0500\
    3044310b3009060355040613024154311c301a060355040a0c13723230333120\
    54696d652050726f76696465723117301506035504030c0e7232303331205453\
    4120726f6f74301e170d3236313131363031343035305a170d33353032303230\
    31343035305a303f310b3009060355040613024154311c301a060355040a0c13\
    72323033312054696d652050726f76696465723112301006035504030c097232\
    3033312054535530820122300d06092a864886f70d01010105000382010f0030\
    82010a0282010100c99b042aabc26925d19f5f8a1ab076706282fe4deff234bc\
    e23bbf216cf6672fd3286c7c49f9240fc764b7942674cc698c018a6ebb564279\
    ae6815944fe981f45fc314e690214de9c7d6ee10b99c203b9b8198832e5fce7f\
    6067ef37fbfbd3c74db70ce159046b321ef761174530f4ee55849f6d6cec75bd\
    5c0aaca2199a6e5a9783a4e4d66a36857734d26a872c1be0e00089e5052110b5\
    19fccb947d08d039323ffc40246844ec82f200fe139896ed85b3fbe7748d8079\
    9a5b94ea68d495e074a4c99eafc64f710ed217c40bb30ba6dbd4370c4f3ad7cc\
    1b6453d82c88a0a28c2180a7afd4f40a79fc6ddecd353468fb8b8636d260358e\
    74f68cbcf0b5a85b0203010001a3783076300c0603551d130101ff0402300030\
    0e0603551d0f0101ff04040302078030160603551d250101ff040c300a06082b\
    06010505070308301d0603551d0e041604147d6c3dcf60183b24898c5c3f5197\
    fbd7049e088e301f0603551d2304183016801480ea130015551ed1fb2eaaeaa8\
    88060534bb7a2e300d06092a864886f70d01010b05000382010100197227286a\
    cb969123f5a313c5633611e335a86e580a7114e6c53945d85cd01cd996811314\
    34989aa806bec3286b92dc689f508fbdbc13db77ee47c68e3ca62612c8ef75d5\
    fa6dd8511606818375e1ab761b7ab95707fa32d02a613f34eaf035b6b3d9c615\
    83379a3ab5eb641f03ffbf40d170d1c04d8785c269acbe0d8fe45fc40713b686\
    c2c1f6395e009b8394880c75016afc64020884ee38f8d380f201ec7e81a69aef\
    182ee5e8a8c98779a3b730f04bf004002372edefc2d37b96d70fa5af439f9a75\
    a2297ec26c667777906220c40ccb70a7d6cd16517af921ca20d9b26fb72b5191\
    5755e11471ffe19ad43e8d5b5269de03e6339a3a87ecd97f62a7c6";

/// A unit stating no `organizationName`: `/C=AT/CN=r2031 TSU without organisation`.
pub(crate) const NO_ORGANISATION: &str = "\
    3082036e30820256a00302010202022032300d06092a864886f70d01010b0500\
    3044310b3009060355040613024154311c301a060355040a0c13723230333120\
    54696d652050726f76696465723117301506035504030c0e7232303331205453\
    4120726f6f74301e170d3236313131363031343035315a170d33353032303230\
    31343035315a3036310b30090603550406130241543127302506035504030c1e\
    72323033312054535520776974686f7574206f7267616e69736174696f6e3082\
    0122300d06092a864886f70d01010105000382010f003082010a0282010100dc\
    2c6586e1a8faadc9bf886a64c5a944c897f3d89762fa8a0a14c86ce5e1a1d288\
    f7a6b71e824481f166b0066726a35b5a5e8fd7b9a509c72f543b58785e93a63d\
    8f4070e8f2dda44910a3b994a3d1745c7e5884a7039a338f9d96540727d780fd\
    755c82bc43100df053fd818207640b0144eeaf8b1343fa1862ea0fc7af784f7f\
    5f8426fa04ab3523f07a10582113393d6da721ee64779e2079c1ee91aad5389b\
    fb44f0a6b7fbd548326ce68c5127465bf64bc78dd590a5655405eaee4c8eee8f\
    8855270a1df7cfb4d27cf0b8ec25971ff447bc605527de559d76470dee648095\
    228c78aeb1c2c632da2db0e38634f4f0914c0aa423146c22576bf5b8206c5f02\
    03010001a3783076300c0603551d130101ff04023000300e0603551d0f0101ff\
    04040302078030160603551d250101ff040c300a06082b06010505070308301d\
    0603551d0e041604140c70ae07b5eef872a7c580d0d8695d5709732084301f06\
    03551d2304183016801480ea130015551ed1fb2eaaeaa888060534bb7a2e300d\
    06092a864886f70d01010b0500038201010092362b7442350cfccdb8cb47e73d\
    f8f78bb7ec1c3abc8f3b5486a00067a795f95f0a934905d46f3d5edf3fa03798\
    f4158cc37f7bc8cbbd12501e73f3c913df2509c6e5bf579eb37a9b3841d4d189\
    9784172a23b9b033bcbce1a8a96be644d766bc524bbbad3354845db50cf62d67\
    e5109ef1a3b5e457733366f8a5bfef65ed0c6e90004ff47634d9ed07ffa7872f\
    4b745f7d869b1759403133c3a5263268baa9fd894d06313db54ed1603431756e\
    d661b8cfe148bc788235cc9aecb6bf740d833b192934b6d237b14e8892d6553c\
    d47957148feb762d63206b2547adc67157cbba7533aa4e8dd9a3689ae11a16aa\
    88d2defaa55da39641d1a8acd9297ab7792e";

/// A unit of a territory that is not a Member State: `/C=US/O=r2031 Time Provider/CN=r2031 TSU elsewhere`.
pub(crate) const US: &str = "\
    3082038130820269a00302010202022033300d06092a864886f70d01010b0500\
    3044310b3009060355040613024154311c301a060355040a0c13723230333120\
    54696d652050726f76696465723117301506035504030c0e7232303331205453\
    4120726f6f74301e170d3236313131363031343035315a170d33353032303230\
    31343035315a3049310b3009060355040613025553311c301a060355040a0c13\
    72323033312054696d652050726f7669646572311c301a06035504030c137232\
    3033312054535520656c7365776865726530820122300d06092a864886f70d01\
    010105000382010f003082010a0282010100e6ef60f8567aa10f9ef6b4677566\
    5cb29e1dae4054cbd64ec5b8f85571c5a5b8696b1f1afeac387f27a1e2d93671\
    e2b8953ce0b06205d2e29098c6cbcfefd545a261366cc0d5d029aa345f439236\
    09f8e6ca492b362ed8da75514ce396d0c05fdc310a89bcae2625c261e00f6a92\
    10447d69fd197ad16025a1c98284b0467240b90fab50dc89337bc497b9981962\
    e03cba0e08ed2fc4ed5e0ecf95e9036562de8743dbf340e8dbfee83ea0524200\
    1dc1199b85a6ae65bdd5199d6313a74f0786fdabbcaae05106dcf6dff56adb02\
    9d6c4f0d7fc64d92cde94d341e0af54c4e3fff73f21c9f006dadd06d052187a7\
    de0962e2c87b19a18ca06f960792b8f1cdcd0203010001a3783076300c060355\
    1d130101ff04023000300e0603551d0f0101ff04040302078030160603551d25\
    0101ff040c300a06082b06010505070308301d0603551d0e04160414d2929ef7\
    a11ec992be65257d3dba4b970a2e917a301f0603551d2304183016801480ea13\
    0015551ed1fb2eaaeaa888060534bb7a2e300d06092a864886f70d01010b0500\
    038201010026f9cfbd581ef90b4ab64868d2c2509e14ab91985c1094a34bc50a\
    aad3061eaf52f0526e5a69293eb3743c2c2e0fa8422c333077b2c5fbdf7bcaab\
    bc80472052a9eb3568885fa8040e071d417af14d323aa10ef5aa04c39a0c0270\
    91892b944646d3d60b5102b3e923726da9af143e2aa61a254ca149ccdb6c9096\
    096a1b299e29a90707582e5375833622b308f11db7396306ff7443769e28662c\
    32e472ad4ef72ae0be8a292c399e10e5404a69d460f2155ebccdefe026697f2c\
    c24bdc44f22624e979bfbfcb7fa804ffce9c7b9f1aef0294bf7a50f08e5eb58a\
    204c901767b8341f241c2723ee523c905f1e001877843a56aee7b14f09d0fa85\
    12792dca96";

/// The token [`TSU`] issued with `openssl ts -reply`, carrying [`TSU`] and [`ROOT`].
pub(crate) const TOKEN: &str = "\
    308209b806092a864886f70d010702a08209a9308209a5020103310f300d0609\
    60864801650304020105003073060b2a864886f70d0109100104a06404623060\
    02010106052a03048f6f3031300d060960864801650304020105000420eba5ff\
    a22f1b19c0a2df423aee56a47317eae042495a88f2b7865245676ea66d020102\
    180f32303236313131363031343035325a3003020101020836c3c8c155bed1f5\
    a08206f8308203773082025fa00302010202022031300d06092a864886f70d01\
    010b05003044310b3009060355040613024154311c301a060355040a0c137232\
    3033312054696d652050726f76696465723117301506035504030c0e72323033\
    312054534120726f6f74301e170d3236313131363031343035305a170d333530\
    3230323031343035305a303f310b3009060355040613024154311c301a060355\
    040a0c1372323033312054696d652050726f7669646572311230100603550403\
    0c0972323033312054535530820122300d06092a864886f70d01010105000382\
    010f003082010a0282010100c99b042aabc26925d19f5f8a1ab076706282fe4d\
    eff234bce23bbf216cf6672fd3286c7c49f9240fc764b7942674cc698c018a6e\
    bb564279ae6815944fe981f45fc314e690214de9c7d6ee10b99c203b9b819883\
    2e5fce7f6067ef37fbfbd3c74db70ce159046b321ef761174530f4ee55849f6d\
    6cec75bd5c0aaca2199a6e5a9783a4e4d66a36857734d26a872c1be0e00089e5\
    052110b519fccb947d08d039323ffc40246844ec82f200fe139896ed85b3fbe7\
    748d80799a5b94ea68d495e074a4c99eafc64f710ed217c40bb30ba6dbd4370c\
    4f3ad7cc1b6453d82c88a0a28c2180a7afd4f40a79fc6ddecd353468fb8b8636\
    d260358e74f68cbcf0b5a85b0203010001a3783076300c0603551d130101ff04\
    023000300e0603551d0f0101ff04040302078030160603551d250101ff040c30\
    0a06082b06010505070308301d0603551d0e041604147d6c3dcf60183b24898c\
    5c3f5197fbd7049e088e301f0603551d2304183016801480ea130015551ed1fb\
    2eaaeaa888060534bb7a2e300d06092a864886f70d01010b0500038201010019\
    7227286acb969123f5a313c5633611e335a86e580a7114e6c53945d85cd01cd9\
    9681131434989aa806bec3286b92dc689f508fbdbc13db77ee47c68e3ca62612\
    c8ef75d5fa6dd8511606818375e1ab761b7ab95707fa32d02a613f34eaf035b6\
    b3d9c61583379a3ab5eb641f03ffbf40d170d1c04d8785c269acbe0d8fe45fc4\
    0713b686c2c1f6395e009b8394880c75016afc64020884ee38f8d380f201ec7e\
    81a69aef182ee5e8a8c98779a3b730f04bf004002372edefc2d37b96d70fa5af\
    439f9a75a2297ec26c667777906220c40ccb70a7d6cd16517af921ca20d9b26f\
    b72b51915755e11471ffe19ad43e8d5b5269de03e6339a3a87ecd97f62a7c630\
    82037930820261a003020102021465e10f8c8672db81c52110d13878145c3a7b\
    32cf300d06092a864886f70d01010b05003044310b3009060355040613024154\
    311c301a060355040a0c1372323033312054696d652050726f76696465723117\
    301506035504030c0e72323033312054534120726f6f74301e170d3236313131\
    363031343035305a170d3336313131333031343035305a3044310b3009060355\
    040613024154311c301a060355040a0c1372323033312054696d652050726f76\
    696465723117301506035504030c0e72323033312054534120726f6f74308201\
    22300d06092a864886f70d01010105000382010f003082010a02820101009c68\
    abdf843d7c6388fe01c4481eeb49128cb1b1c94e9836d571650a9520bc097a42\
    05e52fe3d96a7cf7fe0e57f0910ed96643b09a6042b4fa9b12b875beec4bcdde\
    594e2d3b7cae922d33cf2b70d2dd4e390bbd99d2ac08372d1eefca57a0601b2c\
    ae86c6c929737ba946a7e47962f801160a554f00ef9ee604fb1b31fc3a940135\
    2346fa37da9a83f7c93257f7eda92a8281faa7492bd9f016b7d644ed57457ab6\
    e48a789786193b4813694a7d6ff66c36dd5c9c1bfd19cbe813b5b23d804eab95\
    f93a19f12faf3dcb8a843bf161e132f785eeffa32e8ff34de88a6880335e24bf\
    7eae5c9bf3b0049b358651043847fb3a84b5f6a2d5fbba5bf0e0a231cd830203\
    010001a3633061301d0603551d0e0416041480ea130015551ed1fb2eaaeaa888\
    060534bb7a2e301f0603551d2304183016801480ea130015551ed1fb2eaaeaa8\
    88060534bb7a2e300f0603551d130101ff040530030101ff300e0603551d0f01\
    01ff040403020106300d06092a864886f70d01010b0500038201010030045ea6\
    3036270844502380fb0b331f4b32f28d68882bab33e1b8eaf20fc33ba4936f3f\
    ad07c24585df199c5175f5870e66796c4fa7e117b2946c3f969c6d93dbbd7506\
    62b4ef766fc9ba240c912b6f2118863af08e3b4b4262364d01de3fe71d0a2912\
    67df31179b631bf2dbfd2b68d38c29d90cdb9c88e873e55727915845f1e44de0\
    fd26845a4cdc3926f3fcd3ba878c1efc675129348aa28c809c9131129c4e8578\
    fe84432ee833abf8bbb1e37040e2c8faf1e0e19a3dd87a8fef95e10ae00dad26\
    df2ff3f9683d3519556717380ddb90a97ee045a890b38dccd7e779d9cd406cc7\
    1cfa6ba0d8352348c59a602f00422b08d90580a21d76a7505dab1e563182021c\
    30820218020101304a3044310b3009060355040613024154311c301a06035504\
    0a0c1372323033312054696d652050726f76696465723117301506035504030c\
    0e72323033312054534120726f6f7402022031300d0609608648016503040201\
    0500a081a4301a06092a864886f70d010903310d060b2a864886f70d01091001\
    04301c06092a864886f70d010905310f170d3236313131363031343035325a30\
    2f06092a864886f70d010904312204203044b69f44abfc9316967f3d23ebac06\
    18fbdc0c937a78367153737b287bdc2a3037060b2a864886f70d010910022f31\
    2830263024302204209efbe04fcd4f8216ab12d8da54b09864d70e38c488171a\
    56f750fc378a489abd300d06092a864886f70d010101050004820100509c6893\
    e877d4b36c4fc9adc7702d2b945977dc987ff613b658eaecded46b289c00a7a5\
    12b2220b3258caec0f935d89c275f81fa737c3e35318ea2dc20740da26879d23\
    a09d17127b89670f8d8ae08086ccc368f3bb84c92c63bb12e92c46ae6e053b50\
    28c924f33cfe40f764e5109232896e180cdd8ae8c472e37d8ba201369aa7ce3e\
    e755772352a01c303899fe2a2bdd6d647190c3809a152c90632336876d6c44ae\
    02cea03e71f36375e47fb8420e6435bc15aeae9bc9e24426cefe42dfface5a10\
    6abbe0d93a198fcd1ca62f3944d3c3e79c896120575801b09cfcbdb30da355f1\
    f038964a3e8f993c4592b68e1ff0d8421eb592943b42ef8b7f2590ad";

/// A `TSA/QTST` service granted from 2024, whose digital identity is `identity`.
pub(crate) fn qtst(identity: &[u8]) -> Written<'_> {
    Written {
        service_type: TSA_QTST,
        status: GRANTED,
        since: "2024-01-01T00:00:00Z",
        history: &[],
        for_type: None,
        identity: Some(identity),
    }
}

pub(crate) fn list(territory: &str, provider: &str, services: &[Written<'_>]) -> TrustedList {
    TrustedList::read(list_xml(territory, &[(provider, services)]).as_bytes())
        .expect("the written list reads")
}

fn parsed(der: &[u8]) -> Certificate<'_> {
    x509::parse(der).expect("the fixture parses")
}

fn determined(answer: Answer) -> Outputs {
    let Answer::Determined(outputs) = answer else {
        panic!("the procedure should have run: {answer:?}");
    };
    outputs
}

/// PRO-4.6.4-02, -03 and -09 a): the unit's subject names `AT`, Austria's list holds a `TSA/QTST`
/// service of the unit's own provider whose identity issued it, granted at the instant — so
/// `Qualified`, `PROCESS_PASSED`, `CC` `AT` and `TSP-Name` the provider's.
///
/// And -09 b), the plant beside it: the same service `withdrawn` is `Not_Qualified`.
#[test]
fn a_granted_time_stamping_service_of_the_units_provider_is_qualified() {
    let (root, unit) = (hex(ROOT), hex(TSU));
    let lists = [list("AT", PROVIDER, &[qtst(&root)])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(out.results, Indication::Qualified);
    assert_eq!(out.status, Status::Passed);
    assert!(out.sub_status.is_empty(), "{:?}", out.sub_status);
    assert_eq!(out.country.as_deref(), Some("AT"));
    assert_eq!(out.tsp_name.as_deref(), Some(PROVIDER));

    let withdrawn = Written {
        status: WITHDRAWN,
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[withdrawn])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(out.results, Indication::NotQualified);
    assert_eq!(out.status, Status::Passed);
    assert_eq!(out.tsp_name.as_deref(), Some(PROVIDER));
}

/// PRO-4.6.4-01: an instant before `2016-06-30T22:00:00Z` is `Not_Qualified` and `PROCESS_PASSED`
/// before any list is read, so `CC` stays void (OUT-4.6.3-01). The instant itself is not before
/// it, and the same list answers `Qualified` there.
#[test]
fn before_the_regulation_applied_no_provider_was_qualified() {
    let (root, unit) = (hex(ROOT), hex(TSU));
    let lists = [list("AT", PROVIDER, &[qtst(&root)])];
    let before = Instant::from_unix_seconds(REGULATION_APPLIES_FROM.unix_seconds() - 1);
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        before,
        TSA_QTST,
    ));
    assert_eq!(
        (out.results, out.status, out.country),
        (Indication::NotQualified, Status::Passed, None)
    );
    // At the onset itself the run goes past -01, so `CC` is set; the unit's certificate was not
    // yet issued then, so clause 4.3's path test finds no service: -05, `Not_Qualified`.
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        REGULATION_APPLIES_FROM,
        TSA_QTST,
    ));
    assert_eq!(
        (
            out.results,
            out.status,
            out.country.as_deref(),
            out.tsp_name
        ),
        (Indication::NotQualified, Status::Passed, Some("AT"), None)
    );
}

/// PRO-4.6.4-05: a list whose only service is of another type finds nothing for `TSA/QTST`, which
/// is `Not_Qualified` and `PROCESS_PASSED` with no `TSP-Name`.
#[test]
fn a_list_publishing_no_such_service_says_not_qualified() {
    let (root, unit) = (hex(ROOT), hex(TSU));
    let other = Written {
        service_type: TSA,
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[other])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(
        (out.results, out.status, out.tsp_name),
        (Indication::NotQualified, Status::Passed, None)
    );
}

/// PRO-4.6.4-02 with PRO-4.3.4-01, through -04: a subject in `US` names no Member State, clause
/// 4.3 fails, and the run is `Indeterminate` with `PROCESS_FAILED` and clause 4.3's sub-status.
/// A Member State whose list was not supplied is not run at all.
#[test]
fn a_unit_outside_the_member_states_is_indeterminate_and_a_missing_list_is_not_run() {
    let (root, elsewhere, unit) = (hex(ROOT), hex(US), hex(TSU));
    let lists = [list("AT", PROVIDER, &[qtst(&root)])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&elsewhere),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(out.results, Indication::Indeterminate);
    assert_eq!(out.status, Status::Failed);
    assert_eq!(
        out.sub_status,
        [
            SubStatus::ProcessFailed,
            SubStatus::Matching(Fault::NotAnEuMemberState)
        ]
    );
    assert_eq!(out.country.as_deref(), Some("US"));

    let belgian = [list("BE", PROVIDER, &[qtst(&root)])];
    assert_eq!(
        issuer_qualification(&belgian, &parsed(&unit), &[], ASKED, TSA_QTST),
        Answer::NoListForTerritory("AT".to_owned())
    );
}

/// PRO-4.6.4-08: a subject stating no `organizationName`, and one whose organisation is not the
/// provider's name, are each `Indeterminate` with
/// `ERROR_TSP_NAME_INCONSISTENCY_BETWEEN_TSTO-CERT_AND_TL`.
#[test]
fn a_unit_not_named_for_the_provider_is_indeterminate() {
    let (root, bare, unit) = (hex(ROOT), hex(NO_ORGANISATION), hex(TSU));
    let lists = [list("AT", PROVIDER, &[qtst(&root)])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&bare),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(
        (out.results, out.status, out.sub_status),
        (
            Indication::Indeterminate,
            Status::Failed,
            vec![SubStatus::TspNameInconsistency]
        )
    );
    let renamed = [list("AT", "Another provider", &[qtst(&root)])];
    let out = determined(issuer_qualification(
        &renamed,
        &parsed(&unit),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(out.sub_status, [SubStatus::TspNameInconsistency]);
}

/// PRO-4.6.4-06: two found services whose statuses at the instant differ are `Indeterminate` with
/// `ERROR_INCONSISTENCY_IN_TL_ON_TSTO-CERT_STATUS`. PRO-4.6.4-07: two whose statuses agree but
/// whose identities' keys differ — one the root that issued the unit, one the unit itself — add
/// the duplication warning and the run goes on to -09.
#[test]
fn found_services_disagreeing_on_status_or_key_are_said_so() {
    let (root, unit) = (hex(ROOT), hex(TSU));
    let withdrawn = Written {
        status: WITHDRAWN,
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[qtst(&root), withdrawn])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(
        (out.results, out.status, out.sub_status),
        (
            Indication::Indeterminate,
            Status::Failed,
            vec![SubStatus::InconsistentStatus]
        )
    );

    let lists = [list("AT", PROVIDER, &[qtst(&root), qtst(&unit)])];
    let out = determined(issuer_qualification(
        &lists,
        &parsed(&unit),
        &[],
        ASKED,
        TSA_QTST,
    ));
    assert_eq!(out.results, Indication::Qualified);
    assert_eq!(out.status, Status::Passed);
    assert_eq!(out.sub_status, [SubStatus::DuplicatedServiceInformation]);
}

/// Clause 4.7 over the token: PRO-4.7.4-02 at the instant asked and -04 at `genTime` agree, so the
/// first run's `Qualified` and `PROCESS_PASSED` are the answer (-03).
///
/// PRO-4.7.4-06: a service granted only from a day between the two, `under supervision` before,
/// answers `Not_Qualified` at `genTime`; the result stays the first run's, the status is
/// `PROCESS_FAILED`, and the sub-status names the second answer.
///
/// PRO-4.7.4-05: a second service whose status at `genTime` differs from the first's makes the run
/// at `genTime` fail by -06 while the run at the instant asked passes; clause 4.7 then fails with
/// that run's status and sub-status added.
#[test]
fn a_token_is_a_qualified_time_stamp_only_where_both_instants_agree() {
    let (root, token) = (hex(ROOT), hex(TOKEN));
    let cms = crate::cms::signed_data(&token).expect("the token is a SignedData");
    let lists = [list("AT", PROVIDER, &[qtst(&root)])];
    let out = qualified_time_stamp_of(&lists, &cms, GEN_TIME, ASKED).map(determined);
    let out = out.expect("the token names its unit");
    assert_eq!(
        (
            out.results,
            out.status,
            out.country.as_deref(),
            out.tsp_name
        ),
        (Indication::Qualified, Status::Passed, Some("AT"), None)
    );

    let later = Written {
        since: BETWEEN,
        history: &[(
            "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/undersupervision",
            "2024-01-01T00:00:00Z",
        )],
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[later])];
    let out = determined(
        qualified_time_stamp_of(&lists, &cms, GEN_TIME, ASKED).expect("the token names its unit"),
    );
    assert_eq!(out.results, Indication::Qualified);
    assert_eq!(out.status, Status::Failed);
    assert_eq!(
        out.sub_status,
        [SubStatus::ResultsDifferAtGeneration(
            Indication::NotQualified
        )]
    );

    let withdrawn_then_granted = Written {
        since: BETWEEN,
        history: &[(WITHDRAWN, "2024-01-01T00:00:00Z")],
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[qtst(&root), withdrawn_then_granted])];
    let out = determined(
        qualified_time_stamp_of(&lists, &cms, GEN_TIME, ASKED).expect("the token names its unit"),
    );
    assert_eq!(out.results, Indication::Qualified);
    assert_eq!(out.status, Status::Failed);
    assert_eq!(
        out.sub_status,
        [SubStatus::ProcessFailed, SubStatus::InconsistentStatus]
    );
}

/// ETSI TS 119 172-4 REQ-4.5-01 d) i): the services of types `TSA/QTST` and `TSA` clause 4.3
/// returns for the token's unit at its `genTime`, each with its identity — and none where the list
/// publishes the unit's issuer only as a `CA/QC` service, which is the absence the report states.
#[test]
fn a_tokens_listed_authorities_are_its_time_stamping_services_and_only_those() {
    let (root, token) = (hex(ROOT), hex(TOKEN));
    let cms = crate::cms::signed_data(&token).expect("the token is a SignedData");
    let national = Written {
        service_type: TSA,
        status: "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/recognisedatnationallevel",
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[qtst(&root), national])];
    let found = listed_authorities(&lists, &cms, GEN_TIME);
    let types: Vec<_> = found
        .iter()
        .map(|(listed, identities)| {
            (
                listed.service_type.as_str(),
                listed.status_at_generation.as_deref(),
                identities.as_slice() == [root.clone()],
            )
        })
        .collect();
    assert_eq!(
        types,
        [
            (TSA_QTST, Some(GRANTED), true),
            (
                TSA,
                Some("http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/recognisedatnationallevel"),
                true
            ),
        ]
    );
    let [(first, _), ..] = found.as_slice() else {
        panic!("two services: {found:?}");
    };
    assert_eq!(
        (first.territory.as_str(), first.provider.as_str()),
        ("AT", PROVIDER)
    );
    assert_eq!(first.authentication, Authentication::VouchedForByTheHost);

    let issuer_only = Written {
        service_type: super::super::CA_QC,
        ..qtst(&root)
    };
    let lists = [list("AT", PROVIDER, &[issuer_only])];
    assert!(listed_authorities(&lists, &cms, GEN_TIME).is_empty());
}
