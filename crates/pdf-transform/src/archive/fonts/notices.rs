//! Notices attached to the font file they describe, not to the page or font dictionary.
//!
//! ISO 32000-2 §14.3.2, Tables 347–348, and owner answer A550: bundled Foxit programs carry
//! their complete notice in an XMP Rights Management language alternative. Identity comes from
//! the supplied program bytes, so an unrelated font with a similar PDF name gains no notice.

use std::collections::BTreeMap;

use pdf_font::substitute::{Family, Request};
use pdf_syntax::object::Name;
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

use super::super::decision::Because;
use super::super::prepare::{Spare, metadata_stream};

/// A pre-existing licence must not be silently replaced while attaching this one.
const EXISTING_NOTICE: &str = "the font program already has unreadable metadata or different usage terms; adding the Foxit notice without replacing those terms is not built";
/// ISO 16684-1's predefined rights namespace.
const RIGHTS: &str = "http://ns.adobe.com/xap/1.0/rights/";

/// The notice is data shipped with the same programs, including its final newline.
const FOXIT: &str = include_str!("../../../../../data/standard-fonts/LICENSE_FOXIT");

/// Adds an indirect object-level packet for a bundled face, before its program is written.
pub(super) fn attach(
    document: &Document,
    original: &[u8],
    stream: &mut Object,
    spare: &mut Spare,
    written: &mut BTreeMap<ObjectId, Object>,
) -> Result<(), Because> {
    if !is_foxit(original) {
        return Ok(());
    }
    attach_notice(document, FOXIT, stream, spare, written)
}

/// Adds the packet carrying `notice` to a program stream: [`attach`]'s for a Foxit face, and the
/// CJK pack's for a program of that pack, whose SIL Open Font License asks each copy to carry it
/// (`doc/adr/2302`).
pub(super) fn attach_notice(
    document: &Document,
    notice: &str,
    stream: &mut Object,
    spare: &mut Spare,
    written: &mut BTreeMap<ObjectId, Object>,
) -> Result<(), Because> {
    let existing = stream
        .as_stream()
        .and_then(|stream| stream.dict.get("Metadata"));
    let metadata = if let Some(existing) = existing {
        let existing = document.resolve(existing);
        let existing = existing
            .as_stream()
            .ok_or(Because::NotBuiltYet(EXISTING_NOTICE))?;
        let bytes = document
            .decoded_stream_data(existing)
            .ok_or(Because::NotBuiltYet(EXISTING_NOTICE))?;
        let properties = pdf_model::xmp::Xmp::parse_detail(&bytes)
            .map_err(|_| Because::NotBuiltYet(EXISTING_NOTICE))?;
        if properties.iter().any(|property| {
            property.name.namespace == RIGHTS && property.name.local == "UsageTerms"
        }) {
            // Recognize the exact property even when unrelated metadata surrounds it.
            if exact_terms(&bytes, notice) {
                return Ok(());
            }
            return Err(Because::NotBuiltYet(EXISTING_NOTICE));
        }
        let bytes = pdf_model::xmp::supplement(
            &bytes,
            &[pdf_model::xmp::Supplement {
                namespace: RIGHTS,
                prefix: "xmpRights",
                local: "UsageTerms",
                value: notice.to_owned(),
                form: pdf_model::xmp::Form::Alternative,
            }],
        )
        .map_err(|_| Because::NotBuiltYet(EXISTING_NOTICE))?;
        metadata_stream(&existing.dict, &bytes)
    } else {
        metadata_stream(&Dictionary::new(), packet(notice).as_bytes())
    };
    let at = if let Some((at, _)) = written.iter().find(|(_, object)| **object == metadata) {
        *at
    } else {
        let at = spare
            .take(document)
            .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
        written.insert(at, metadata);
        at
    };
    let Object::Stream(stream) = stream else {
        return Err(Because::NotBuiltYet(super::PROGRAM_NOT_ENCODED));
    };
    std::sync::Arc::make_mut(stream)
        .dict
        .insert(Name::new(&b"Metadata"[..]), Object::Reference(at));
    Ok(())
}

/// Uses the namespace-aware writer only as a structural witness; no metadata is removed from
/// the output. Equality proves that the canonical element is the property actually selected,
/// rather than a matching string in a comment, CDATA, or a differently bound namespace.
fn exact_terms(bytes: &[u8], notice: &str) -> bool {
    let Ok(properties) = pdf_model::xmp::Xmp::parse_detail(bytes) else {
        return false;
    };
    let mut matching = properties.iter().filter(|property| {
        property.name.namespace == RIGHTS && property.name.local == "UsageTerms"
    });
    let Some(property) = matching.next() else {
        return false;
    };
    if matching.next().is_some() {
        return false;
    }
    let Some(alternatives) = property.value.alternatives() else {
        return false;
    };
    let [(language, value)] = alternatives else {
        return false;
    };
    if language.as_deref() != Some("x-default") || value.text() != Some(notice.trim()) {
        return false;
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let element = format!(
        "<xmpRights:UsageTerms><rdf:Alt>{}</rdf:Alt></xmpRights:UsageTerms>",
        terms(notice)
    );
    if !text.contains(&element) {
        return false;
    }
    let Ok(without) = pdf_model::xmp::remove(
        bytes,
        &[pdf_model::xmp::Name {
            namespace: RIGHTS.to_owned(),
            local: "UsageTerms".to_owned(),
        }],
    ) else {
        return false;
    };
    without == text.replacen(&element, "", 1).as_bytes()
}

/// Exact bundled bytes establish provenance before any metric restatement.
fn is_foxit(program: &[u8]) -> bool {
    [
        Family::Serif,
        Family::Monospace,
        Family::Symbol,
        Family::ZapfDingbats,
    ]
    .into_iter()
    .any(|family| {
        [false, true].into_iter().any(|bold| {
            [false, true].into_iter().any(|italic| {
                let (bytes, _) = pdf_font::standard::face(Request {
                    family,
                    bold,
                    italic,
                    standard: true,
                });
                bytes == program
            })
        })
    })
}

/// ISO 16684-1's `UsageTerms` is a language alternative, with one default-language item here.
fn packet(notice: &str) -> String {
    let text = terms(notice);
    format!(
        "<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
        <x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\
        <rdf:Description rdf:about=\"\" xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\">\
        <xmpRights:UsageTerms><rdf:Alt>{text}</rdf:Alt></xmpRights:UsageTerms>\
        </rdf:Description></rdf:RDF></x:xmpmeta>\n<?xpacket end=\"w\"?>"
    )
}

/// Exact character content, separate from the viewer reader that trims display values.
fn terms(value: &str) -> String {
    let text = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("<rdf:li xml:lang=\"x-default\">{text}</rdf:li>")
}

#[cfg(test)]
mod tests {
    #[test]
    fn license_text_escapes_xml_without_dropping_spaces_or_the_final_newline() {
        assert_eq!(
            super::terms(" <&>\n"),
            "<rdf:li xml:lang=\"x-default\"> &lt;&amp;&gt;\n</rdf:li>"
        );
    }

    #[test]
    fn every_foxit_program_is_recognized_by_bytes_but_liberation_and_unrelated_bytes_are_not() {
        use pdf_font::substitute::{Family, Request};
        for family in [
            Family::Serif,
            Family::Monospace,
            Family::Symbol,
            Family::ZapfDingbats,
            Family::SansSerif,
        ] {
            for bold in [false, true] {
                for italic in [false, true] {
                    let (bytes, _) = pdf_font::standard::face(Request {
                        family,
                        bold,
                        italic,
                        standard: true,
                    });
                    assert_eq!(super::is_foxit(bytes), family != Family::SansSerif);
                }
            }
        }
        assert!(!super::is_foxit(b"FoxitSerif"));
    }

    #[test]
    fn the_notice_is_a_default_language_alternative_with_every_license_character() {
        let packet = super::packet(super::FOXIT);
        let properties = pdf_model::xmp::Xmp::parse_detail(packet.as_bytes()).expect("XMP");
        let property = properties.first().expect("UsageTerms");
        assert_eq!(property.name.namespace, super::RIGHTS);
        assert_eq!(property.name.local, "UsageTerms");
        let alternatives = property.value.alternatives().expect("rdf:Alt");
        let (language, value) = alternatives.first().expect("default");
        assert_eq!(language.as_deref(), Some("x-default"));
        assert_eq!(value.text(), Some(super::FOXIT.trim()));
        assert!(
            !super::FOXIT.contains(['&', '<', '>']),
            "this raw-character witness needs no entity decoding"
        );
        let content = packet
            .split_once("<rdf:li xml:lang=\"x-default\">")
            .expect("item")
            .1
            .split_once("</rdf:li>")
            .expect("end")
            .0;
        assert_eq!(content, super::FOXIT);
    }
    #[test]
    fn exact_terms_rejects_trimmed_text_and_copies_outside_the_property() {
        let packet = super::packet(super::FOXIT);
        assert!(super::exact_terms(packet.as_bytes(), super::FOXIT));
        let trimmed = packet.replace(super::FOXIT, super::FOXIT.trim());
        assert!(!super::exact_terms(trimmed.as_bytes(), super::FOXIT));
        let element = format!(
            "<xmpRights:UsageTerms><rdf:Alt>{}</rdf:Alt></xmpRights:UsageTerms>",
            super::terms(super::FOXIT)
        );
        let wrong = packet.replace(super::FOXIT, "different terms");
        let decoy = wrong.replace("</rdf:RDF>", &format!("<!--{element}--></rdf:RDF>"));
        assert!(!super::exact_terms(decoy.as_bytes(), super::FOXIT));
        let duplicate = packet.replace(
            "</rdf:Description>",
            &format!("{element}</rdf:Description>"),
        );
        assert!(!super::exact_terms(duplicate.as_bytes(), super::FOXIT));
        // Keep Description in the real RDF namespace through another prefix, while rebinding
        // rdf for the otherwise canonical Alt/li element spellings inside the rights property.
        let rebound = packet
            .replace(
                "<rdf:Description rdf:about=\"\"",
                "<r:Description xmlns:r=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" xmlns:rdf=\"urn:wrong-rdf\" r:about=\"\"",
            )
            .replace("</rdf:Description>", "</r:Description>");
        assert!(!super::exact_terms(rebound.as_bytes(), super::FOXIT));
    }
}
