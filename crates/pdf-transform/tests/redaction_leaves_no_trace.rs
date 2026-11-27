//! A redaction leaves no trace of what the region held, wherever in the file a copy of it sits.
//!
//! ISO 32000-2 §12.5.6.23 binds the processor that applies a redaction: it "shall remove all
//! traces of the specified content", and it "shall also be diligent in their consideration of all
//! content that can exist in a PDF document"; the owner's `doc/questions/A834` makes the removal
//! physical. So the fixture here puts one string in every place a PDF can carry text that
//! describes, repeats or draws the content of a region, redacts the region, and opens the output:
//! every object, every stream decoded (an object stream included, which is how a compressed
//! object's strings are reached), every string read both as its bytes and as a §7.9.2.2 text
//! string, so a UTF-16 copy is found as surely as a `PDFDocEncoding` one.
//!
//! The places divide by what the clause binds (ADR 3725):
//!
//! - **in the region**, removed: the page's content, the `/ActualText` of a marked-content
//!   sequence the removal took, the `/ActualText`, `/Alt`, `/E` and `/T` of the structure element
//!   that sequence belongs to and of its ancestors, the page's own `/Metadata` packet, every
//!   annotation whose rectangle meets the region with its `/Contents`, `/RC` and appearance, a
//!   field whose widget is there with its `/V`, the same field's value in the XFA resource — its
//!   `datasets` data value, its `form` packet value and its template default (§K.2, ADR 3789) —
//!   and the `/Redact` annotation itself, which a popup's `/Parent` would otherwise carry back in;
//! - **elsewhere**, carried and named in a [`pdf_transform::Warning`]: an outline item's title, a
//!   named destination, an embedded file, an optional content group's name, the document's own
//!   XMP packet, its information dictionary and an annotation outside the region. None of these
//!   is content *in* the region; each is the reader's to redact by its own route, and the warning
//!   is what makes that a decision rather than an accident.

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot be built must fail loudly"
)]

use std::collections::HashSet;
use std::fmt::Write as _;

use pdf_syntax::{Document, Limits, Object, ObjectId};
use pdf_transform::redact::RedactPlan;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Report, Source, apply};

/// The string every place of the fixture carries.
const SECRET: &str = "SECRETX";

/// Writes `objects` (numbered from one) into a §7.5.4 cross-referenced file, `trailer` added to
/// the trailer dictionary.
fn assemble(objects: &[String], trailer: &str) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// A stream object's text with its `/Length` stated.
fn stream(entries: &str, data: &str) -> String {
    format!(
        "<< {entries} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// `text` as a §7.9.2.2 UTF-16BE text string, written in hexadecimal with its byte order mark.
fn utf16(text: &str) -> String {
    text.encode_utf16()
        .fold(String::from("<FEFF"), |mut hex, unit| {
            let _ = write!(hex, "{unit:04X}");
            hex
        })
        + ">"
}

/// A one-page tagged form document that carries [`SECRET`] in every place the module comment
/// names, with a `/Redact` annotation over the line that shows it. Each place's copy is preceded
/// by a label, so a hit says which place it is.
fn fixture() -> Vec<u8> {
    fixture_with("name")
}

/// [`fixture`], its XFA template's field named `template_field`: the PDF field is `name`, so any
/// other name is a template that does not name the field, whose places cannot be found.
fn fixture_with(template_field: &str) -> Vec<u8> {
    let content = format!(
        "/P <</MCID 0>> BDC BT /F1 12 Tf 20 150 Td (KEEP) Tj ET EMC\n\
         /Span <</MCID 1 /ActualText (bdc-actual {SECRET})>> BDC \
         BT /F1 12 Tf 20 100 Td ({SECRET}) Tj ET EMC\n\
         BT /F1 12 Tf 20 92 Td ({SECRET}) Tj ET"
    );
    let mut objects = vec![
        // 1
        "<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 9 0 R /MarkInfo << /Marked true >> \
         /Metadata 12 0 R /Outlines 13 0 R /Names << /Dests 15 0 R /EmbeddedFiles 16 0 R >> \
         /OCProperties << /OCGs [18 0 R] /D << /Order [18 0 R] >> >> /AcroForm << /Fields \
         [20 0 R] /XFA [(xdp:xdp) 28 0 R (template) 29 0 R (datasets) 30 0 R (form) 31 0 R \
         (</xdp:xdp>) 32 0 R] >> >>"
            .to_owned(),
        // 2
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        // 3
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 5 0 R \
         >> /Properties << /oc1 18 0 R >> >> /Contents 4 0 R /Annots [7 0 R 8 0 R 19 0 R 20 0 R \
         21 0 R 22 0 R] /StructParents 0 /Metadata 11 0 R >>"
            .to_owned(),
        // 4
        stream("", &content),
        // 5
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        // 6: the element whose content the region does not reach.
        "<< /Type /StructElem /S /P /P 24 0 R /Pg 3 0 R /K 0 /Alt (KEEP-ALT) >>".to_owned(),
        // 7
        format!(
            "<< /Type /Annot /Subtype /Redact /Rect [10 90 150 120] /Contents (redact-note \
             {SECRET}) /Popup 8 0 R >>"
        ),
        // 8: a popup outside the region, whose /Parent is the redaction.
        "<< /Type /Annot /Subtype /Popup /Rect [160 160 200 200] /Parent 7 0 R >>".to_owned(),
        // 9
        "<< /Type /StructTreeRoot /K [24 0 R] /ParentTree 23 0 R /ParentTreeNextKey 1 >>"
            .to_owned(),
        // 10: the element whose marked content the region takes, every alternate in UTF-16.
        format!(
            "<< /Type /StructElem /S /Span /P 24 0 R /Pg 3 0 R /K 1 /ActualText {} /Alt {} /E {} \
             /T {} >>",
            utf16(&format!("elem-actual {SECRET}")),
            utf16(&format!("elem-alt {SECRET}")),
            utf16(&format!("elem-e {SECRET}")),
            utf16(&format!("elem-t {SECRET}")),
        ),
        // 11
        stream(
            "/Type /Metadata /Subtype /XML",
            &format!("<x:xmpmeta><dc:description>page-xmp {SECRET}</dc:description></x:xmpmeta>"),
        ),
        // 12
        stream(
            "/Type /Metadata /Subtype /XML",
            &format!("<x:xmpmeta><dc:title>doc-xmp {SECRET}</dc:title></x:xmpmeta>"),
        ),
        // 13
        "<< /Type /Outlines /First 14 0 R /Last 14 0 R /Count 1 >>".to_owned(),
        // 14
        format!("<< /Title (outline {SECRET}) /Parent 13 0 R /Dest [3 0 R /XYZ 20 100 0] >>"),
        // 15
        format!("<< /Names [(dest-{SECRET}) [3 0 R /Fit]] >>"),
        // 16
        "<< /Names [(file.txt) 17 0 R] >>".to_owned(),
        // 17
        format!(
            "<< /Type /Filespec /F (file.txt) /UF (file.txt) /EF << /F 25 0 R >> /Desc \
             (file-desc {SECRET}) >>"
        ),
        // 18
        format!("<< /Type /OCG /Name (ocg {SECRET}) >>"),
        // 19: a text annotation inside the region.
        format!(
            "<< /Type /Annot /Subtype /Text /Rect [30 95 50 115] /Contents (note {SECRET}) /RC \
             (<p>rich {SECRET}</p>) >>"
        ),
        // 20: a text field merged with its widget, inside the region.
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /V (field-v {SECRET}) /Rect [100 \
             95 140 115] /P 3 0 R /AP << /N 26 0 R >> >>"
        ),
        // 21: a square annotation outside the region.
        format!(
            "<< /Type /Annot /Subtype /Square /Rect [10 10 50 50] /Contents {} >>",
            utf16(&format!("outside {SECRET}"))
        ),
        // 22: a reply, outside the region, to the text annotation inside it.
        "<< /Type /Annot /Subtype /Text /Rect [160 10 180 30] /IRT 19 0 R /Contents (re: ok) >>"
            .to_owned(),
        // 23
        "<< /Nums [0 [6 0 R 10 0 R]] >>".to_owned(),
        // 24: the document element, whose subtree holds both.
        format!(
            "<< /Type /StructElem /S /Document /P 9 0 R /K [6 0 R 10 0 R] /ActualText \
             (doc-actual KEEP {SECRET}) >>"
        ),
        // 25
        stream("/Type /EmbeddedFile", &format!("embedded {SECRET}")),
        // 26
        stream(
            "/Type /XObject /Subtype /Form /BBox [0 0 40 20] /Resources << /Font << /F1 5 0 R \
             >> >>",
            &format!("BT /F1 10 Tf 2 5 Td (ap {SECRET}) Tj ET"),
        ),
        // 27
        format!("<< /Subject (info {SECRET}) >>"),
    ];
    objects.extend(xfa_objects(template_field));
    assemble(&objects, "/Info 27 0 R")
}

/// Objects 28 to 32: §K.2's array form of the XFA resource, the field's value in three packets —
/// the template's default, the `datasets` data value and the `form` packet's value.
fn xfa_objects(template_field: &str) -> Vec<String> {
    vec![
        stream("", "<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">"),
        stream(
            "",
            &format!(
                "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\"><subform>\
                 <field name=\"{template_field}\"><ui><textEdit/></ui><value><text>xfa-default \
                 {SECRET}</text></value></field></subform></template>"
            ),
        ),
        stream(
            "",
            &format!(
                "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data>\
                 <form1><name>xfa-data {SECRET}</name></form1></xfa:data></xfa:datasets>"
            ),
        ),
        stream(
            "",
            &format!(
                "<form xmlns=\"http://www.xfa.org/schema/xfa-form/2.8/\"><subform><field \
                 name=\"name\"><value><text>xfa-form {SECRET}</text></value></field></subform>\
                 </form>"
            ),
        ),
        stream("", "</xdp:xdp>"),
    ]
}

/// The report and the bytes of the one output.
fn redact(bytes: Vec<u8>) -> (Report, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            image_samples: None,
        }),
        &[Source::new(bytes)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the redaction applies");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one source, one output");
    (report, outputs.remove(0).1)
}

/// The label in front of each [`SECRET`] in `text`: the word before it, or the key path where the
/// string stands alone.
fn labels_in(text: &str, path: &str, hits: &mut Vec<String>) {
    for (at, _) in text.match_indices(SECRET) {
        let before = text.get(..at).unwrap_or_default().trim_end();
        let label = before
            .rsplit(|c: char| c.is_whitespace() || c == '>' || c == '(')
            .next()
            .unwrap_or_default();
        hits.push(format!("{path}: {label}"));
    }
}

/// `bytes` read as Latin-1, and as UTF-16 of either byte order from either of the first two
/// bytes, so a copy in any of them is found.
fn readings(bytes: &[u8]) -> Vec<String> {
    let units = |bytes: &[u8], big: bool| -> String {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if big {
                    u16::from_be_bytes(*pair)
                } else {
                    u16::from_le_bytes(*pair)
                }
            })
            .collect();
        String::from_utf16_lossy(&units)
    };
    let odd = bytes.get(1..).unwrap_or_default();
    vec![
        bytes.iter().copied().map(char::from).collect(),
        units(bytes, true),
        units(bytes, false),
        units(odd, true),
        units(odd, false),
    ]
}

/// Every string an object holds, at any depth, with the key path that reaches it.
fn strings(object: &Object, path: &str, hits: &mut Vec<String>, depth: usize) {
    if depth > 32 {
        return;
    }
    match object {
        Object::String(bytes) => {
            let mut found = Vec::new();
            for reading in readings(bytes) {
                labels_in(&reading, path, &mut found);
            }
            labels_in(&pdf_syntax::text_string(bytes), path, &mut found);
            found.dedup();
            hits.extend(found.into_iter().take(1));
        }
        Object::Array(items) => {
            for item in items {
                strings(item, &format!("{path}[]"), hits, depth.saturating_add(1));
            }
        }
        Object::Dictionary(dict) => {
            for (key, value) in dict.iter() {
                let key = String::from_utf8_lossy(key.as_bytes());
                strings(
                    value,
                    &format!("{path}/{key}"),
                    hits,
                    depth.saturating_add(1),
                );
            }
        }
        Object::Stream(stream) => {
            for (key, value) in stream.dict.iter() {
                let key = String::from_utf8_lossy(key.as_bytes());
                strings(
                    value,
                    &format!("{path}/{key}"),
                    hits,
                    depth.saturating_add(1),
                );
            }
        }
        _ => {}
    }
}

/// Every place the written file still holds [`SECRET`]: its raw bytes, every object's strings,
/// every stream decoded, and the text a reader extracts from its page.
fn hits(bytes: &[u8]) -> Vec<String> {
    let mut hits = Vec::new();
    for reading in readings(bytes) {
        labels_in(&reading, "raw", &mut hits);
    }
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let size = document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .expect("a /Size");
    for number in 1..size {
        let object = document.get(ObjectId {
            number,
            generation: 0,
        });
        strings(&object, "object", &mut hits, 0);
        if let Some(stream) = object.as_stream() {
            let data = document
                .decoded_stream_data(stream)
                .expect("every stream decodes");
            let mut found = Vec::new();
            for reading in readings(&data) {
                labels_in(&reading, "stream", &mut found);
            }
            found.sort();
            found.dedup();
            hits.extend(found);
        }
    }
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    labels_in(
        &pdf_model::interpret(&document, &page).text,
        "extracted",
        &mut hits,
    );
    hits.sort();
    hits.dedup();
    hits
}

/// The labels of the places the module comment says the clause does not bind.
const ELSEWHERE: [&str; 8] = [
    "outline",
    "dest-",
    "file-desc",
    "embedded",
    "ocg",
    "doc-xmp",
    "info",
    "outside",
];

fn elsewhere(hit: &str) -> bool {
    ELSEWHERE.iter().any(|label| hit.ends_with(label))
}

#[test]
fn no_place_the_region_binds_keeps_what_it_held() {
    let (_, out) = redact(fixture());
    let found = hits(&out);
    let bound: Vec<&String> = found.iter().filter(|hit| !elsewhere(hit)).collect();
    assert!(bound.is_empty(), "the output still holds {bound:#?}");
}

#[test]
fn every_place_elsewhere_is_carried_and_named() {
    let (report, out) = redact(fixture());
    let found = hits(&out);
    let labels: HashSet<&str> = ELSEWHERE
        .iter()
        .copied()
        .filter(|label| found.iter().any(|hit| hit.ends_with(label)))
        .collect();
    assert_eq!(labels.len(), ELSEWHERE.len(), "carried: {found:#?}");
    let warned: Vec<&str> = report
        .warnings
        .iter()
        .map(|warning| warning.detail.as_str())
        .collect();
    // A warning names the place and never repeats the text: the report is read on a terminal and
    // kept in logs, and a warning that quoted the removed words would be one more copy of them.
    assert!(
        warned.iter().all(|detail| !detail.contains(SECRET)),
        "a warning repeats the removed text: {warned:#?}"
    );
    for place in [
        "/Outlines",
        "/Dests",
        "/EmbeddedFiles",
        "/OCProperties",
        "/Metadata",
        "/Info",
        "/Annots",
    ] {
        assert!(
            warned.iter().any(|detail| detail.contains(place)),
            "no warning names {place}: {warned:#?}"
        );
    }
}

#[test]
fn what_the_removal_took_is_reported() {
    let (report, _) = redact(fixture());
    let details: Vec<&str> = report
        .departures
        .iter()
        .map(|departure| departure.detail.as_str())
        .collect();
    for phrase in [
        "/Metadata",
        "structure element",
        "marked-content",
        "annotation",
        "field",
        "XFA packet",
    ] {
        assert!(
            details.iter().any(|detail| detail.contains(phrase)),
            "no departure names {phrase}: {details:#?}"
        );
    }
}

/// The XFA packets of the written file's interactive form dictionary, each decoded, by name.
fn xfa_packets(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let catalog = document.catalog().expect("a catalog");
    let form = document.get_key(&catalog, "AcroForm");
    let Object::Array(packets) = document.get_key(form.as_dict()?, "XFA") else {
        return None;
    };
    Some(
        packets
            .chunks(2)
            .filter_map(|pair| {
                let [Object::String(name), value] = pair else {
                    return None;
                };
                let stream = match value {
                    Object::Reference(id) => document.get(*id),
                    other => other.clone(),
                };
                let data = document
                    .decoded_stream_data(stream.as_stream()?)
                    .expect("a packet decodes");
                Some((
                    String::from_utf8_lossy(name).into_owned(),
                    String::from_utf8_lossy(&data).into_owned(),
                ))
            })
            .collect(),
    )
}

/// §K.2's consistency kept, not only the value gone: every packet stays, each value's element
/// stays with its content emptied, so a later field of the same name still binds by its index
/// (ADR 3789).
#[test]
fn the_xfa_packets_stay_with_the_value_emptied() {
    let (_, out) = redact(fixture());
    let packets = xfa_packets(&out).expect("the XFA resource is carried");
    let packet = |wanted: &str| {
        packets
            .iter()
            .find(|(name, _)| name == wanted)
            .map(|(_, text)| text.as_str())
            .expect("the packet is carried")
    };
    assert!(packet("datasets").contains("<name></name>"), "{packets:#?}");
    assert!(
        packet("form").contains("<value><text></text></value>"),
        "{packets:#?}"
    );
    assert!(
        packet("template").contains("<value><text></text></value>"),
        "{packets:#?}"
    );
}

/// Where the template does not name the field, the places of its value cannot be found with
/// certainty, and `/XFA` goes whole and is said (ADR 3789): never left with the value in it.
#[test]
fn an_xfa_resource_whose_places_cannot_be_found_goes_whole() {
    let (report, out) = redact(fixture_with("other"));
    assert!(xfa_packets(&out).is_none(), "the /XFA entry is carried");
    let found = hits(&out);
    assert!(
        !found.iter().any(|hit| hit.contains("xfa-")),
        "the output still holds {found:#?}"
    );
    assert!(
        report
            .departures
            .iter()
            .any(|departure| departure.detail.contains("/XFA not written")),
        "no departure names the /XFA: {:#?}",
        report.departures
    );
}
