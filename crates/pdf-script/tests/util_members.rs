//! `util`'s seven helpers ADR 1762 bridges: `scand`, `crackURL`, `spansToXML`, `xmlToSpans`,
//! `streamFromString`, `stringFromStream` and `iconStreamFromIcon`.
//!
//! Where Adobe's reference prints an example, the example is the fixture and its printed result
//! the expected value (habit 39): `crackURL`'s two examples, `scand`'s round trip through `printd`,
//! and the `Span` round trip `xmlToSpans` is documented beside. Every other expected value is the
//! documented choice the member's doc comment states, or RFC 3986's syntax.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::aform::Trigger;
use pdf_model::view::ScriptSite;
use pdf_script::{Budget, Ending, Event, Outcome, Realm, RefusalKind, Request};

/// A calculate script on `Total`, with no field told, at seven hours east of UT.
fn request(script: &str) -> Request {
    Request {
        site: ScriptSite::Field(Trigger::Calculate),
        field: "Total".to_owned(),
        label: String::new(),
        script: script.to_owned(),
        event: Event {
            value: String::new(),
            change: String::new(),
            selection_start: 0,
            selection_end: 0,
            will_commit: false,
            commit_key: 0,
            field_full: false,
            change_ex: String::new(),
            source: String::new(),
            shift: false,
            modifier: false,
            key_down: false,
            rich_value: String::new(),
        },
        fields: Vec::new(),
        page: 0,
        pages: 1,
        dirty: false,
        document: None,
        view: pdf_model::view::WindowView::default(),
        moment: 1_704_465_015_000,
        utc_offset_seconds: 7 * 3600,
    }
}

/// Runs `script` in a realm of its own.
fn run(script: &str) -> Outcome {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    realm.run(&request(script))
}

/// What the script left in `event.value`.
fn value(script: &str) -> String {
    let ran = run(script);
    assert_eq!(ran.ending, Ending::Finished, "{script}: {ran:?}");
    ran.value.expect("the script sets event.value")
}

#[test]
fn crack_url_answers_the_reference_s_first_example() {
    // The reference's example 1 and the object it prints, property by property; `cUser` and
    // `cPassword`, which the URL does not state, are absent from what it prints.
    let written = value(
        r#"var a = util.crackURL("http://example.org/myPath?name0=value0&name1=value1#frag");
           event.value = [a.cScheme, a.cHost, a.nPort, a.cPath, a.cQuery, a.cFragment,
                          a.nURLType, "cUser" in a, "cPassword" in a].join("|");"#,
    );
    assert_eq!(
        written,
        "http|example.org|80|/myPath|name0=value0&name1=value1|frag|0|false|false"
    );
}

#[test]
fn crack_url_answers_the_reference_s_ipv6_example() {
    // Example 2: its script prints the host in brackets where `nURLType` is 1, so the host is
    // answered without them.
    let written = value(
        r#"var a = util.crackURL("http://[2001:1890:110b:15ce:250:56ff:febb:6111]/Ex");
           event.value = (a.nURLType == 1 ? "host = [" + a.cHost + "]" : "host = " + a.cHost)
                         + "|" + a.cPath + "|" + a.nPort;"#,
    );
    assert_eq!(
        written,
        "host = [2001:1890:110b:15ce:250:56ff:febb:6111]|/Ex|80"
    );
}

#[test]
fn crack_url_reads_user_information_and_a_port_and_throws_a_parameter_error() {
    let written = value(
        r#"var a = util.crackURL("https://ann:pw@host.example:8443/");
           var thrown = [];
           ["ftp://example.org/", "not a url", ""].forEach(function (u) {
               try { util.crackURL(u); thrown.push("none"); }
               catch (e) { thrown.push(e.name); }
           });
           try { util.crackURL(); } catch (e) { thrown.push(e.name); }
           event.value = [a.cUser, a.cPassword, a.nPort].join("|") + "|" + thrown.join(",");"#,
    );
    assert_eq!(
        written,
        "ann|pw|8443|TypeError,TypeError,TypeError,TypeError"
    );
}

#[test]
fn scand_reads_back_what_printd_wrote() {
    // The reference's example 1: today's date written through `mm/dd/yyyy`, read back through it,
    // and written again in another picture. Then the numbered formats, each read back as the moment
    // `printd` wrote, and a text that does not follow the picture, which answers null.
    let written = value(
        r#"var d = new Date(2000, 7, 1, 14, 56, 5);
           var back = util.scand("mm/dd/yyyy", util.printd("mm/dd/yyyy", d));
           var parts = [util.printd("yyyy mmm dd", back)];
           [0, 1, 2].forEach(function (n) {
               parts.push(util.scand(n, util.printd(n, d)).getTime() == d.getTime());
           });
           parts.push(util.scand("mm/dd/yyyy", "13/45/2000"));
           parts.push(util.scand("mm/dd/yy", "01/02/49").getFullYear());
           parts.push(util.scand("mm/dd/yy", "01/02/50").getFullYear());
           event.value = parts.join("|");"#,
    );
    assert_eq!(written, "2000 Aug 01|true|true|true||2049|1950");
}

#[test]
fn spans_round_trip_through_xml_and_keep_the_reference_s_properties() {
    // The reference's `xmlToSpans` example: a string to spans and back, with the styling the
    // `Span` page's superscript example builds.
    let written = value(
        r#"var spans = [{alignment: "center", text: "The answer is x"},
                        {text: "2/3", superscript: true},
                        {text: ".  ", superscript: false},
                        {underline: true, text: "Did you get it right?", fontStyle: "italic",
                         textColor: color.red}];
           var back = util.xmlToSpans(util.spansToXML(spans));
           event.value = back.map(function (s) {
               return [s.text, s.alignment, s.superscript, s.underline, s.fontStyle,
                       s.textColor.join(",")].join("/");
           }).join("|");"#,
    );
    assert_eq!(
        written,
        "The answer is x/center/false/false/normal/RGB,0,0,0|\
         2/3/center/true/false/normal/RGB,0,0,0|\
         .  /center/false/false/normal/RGB,0,0,0|\
         Did you get it right?/center/false/true/italic/RGB,1,0,0"
    );
}

#[test]
fn xml_to_spans_reads_plain_characters_as_one_span() {
    let written = value(
        r#"var spans = util.xmlToSpans("no markup here");
           event.value = spans.length + "|" + spans[0].text + "|" + spans[0].textSize;"#,
    );
    assert_eq!(written, "1|no markup here|12");
}

#[test]
fn a_string_reads_back_through_its_stream_in_utf_8_and_utf_16() {
    let written = value(
        r#"var s = util.streamFromString("Grüße", "utf-8");
           var first = s.read(2);
           var rest = s.read();
           var end = s.read(4);
           var both = ["utf-8", "utf-16"].map(function (set) {
               return util.stringFromStream(util.streamFromString("Grüße 😀", set), set);
           });
           var own = {chunks: ["4869", ""], read: function () { return this.chunks.shift(); }};
           event.value = [first, rest, end === "", both.join(","),
                          util.stringFromStream(own)].join("|");"#,
    );
    // "Grüße" in UTF-8 is 47 72 c3bc c39f 65: two bytes asked for, then the rest.
    assert_eq!(written, "4772|c3bcc39f65|true|Grüße 😀,Grüße 😀|Hi");
}

#[test]
fn a_character_set_this_program_does_not_carry_is_refused_by_name() {
    let ran = run(r#"util.streamFromString("x", "Shift-JIS");"#);
    assert!(matches!(ran.ending, Ending::Threw(_)), "{ran:?}");
    let refusal = ran.refusals.first().expect("one refusal");
    assert_eq!(refusal.member, "util.streamFromString");
    assert!(
        matches!(&refusal.kind, RefusalKind::Unreachable(why) if why.contains("Shift-JIS")),
        "{refusal:?}"
    );
    let ran = run(r#"util.stringFromStream({read: function () { return "xyz"; }});"#);
    assert!(
        matches!(&ran.ending, Ending::Threw(why) if why.contains("hexadecimal")),
        "{ran:?}"
    );
}

#[test]
fn a_stream_that_never_ends_meets_the_string_budget_or_the_call_bound() {
    let ran = run(r#"var chunk = "00".repeat(1 << 20);
                     util.stringFromStream({read: function () { return chunk; }});"#);
    assert!(matches!(ran.ending, Ending::Exceeded(_)), "{ran:?}");
    let ran = run(r#"util.stringFromStream({read: function () { return "00"; }});"#);
    assert!(
        matches!(&ran.ending, Ending::Threw(why) if why.contains("4 096 times")),
        "{ran:?}"
    );
}

#[test]
fn icon_stream_from_icon_is_refused_with_its_reason() {
    let ran = run("util.iconStreamFromIcon({});");
    assert!(matches!(ran.ending, Ending::Threw(_)), "{ran:?}");
    let refusal = ran.refusals.first().expect("one refusal");
    assert_eq!(refusal.member, "util.iconStreamFromIcon");
    assert!(
        matches!(&refusal.kind, RefusalKind::Unreachable(why) if why.contains("Icon")),
        "{refusal:?}"
    );
}
