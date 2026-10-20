//! An extension schema container missing a field nothing in the file states, discarded with the
//! properties it described (`doc/pdf-a-mitigations.md`'s
//! `metadata/extension-schema-container-fields` entry).
//!
//! ISO 19005-2 section 6.6.2.3.3's four tables say which fields a container's schema, property,
//! value type and field descriptions state. A field stated with another prefix is respelled
//! without asking anyone; a field the packet does not state at all — a schema's name, a
//! property's description, a value's category — is in no part of the file, so writing one would
//! be this converter inventing metadata about metadata. What is left is the catalogue's `discard`:
//! the container goes, and with it every property in a namespace it described, because section
//! 6.6.2.3.1 admits a property only where its schema is predefined or described — which is
//! section 3.9's removal of `doc/pdf-a-conversion-limits.md`, and the same loss,
//! `metadata-property`, named property by property in the report and in `xmpMM:History`.
//!
//! **The whole container rather than the incomplete description inside it.** `pdf_model::xmp`
//! removes a property and not one item of an array, so a container holding one complete and one
//! incomplete schema loses both, and both schemas' properties; the report names every one, so the
//! cost is visible where it is paid. A narrower cut is owed the day the writer can take an item.

use std::collections::BTreeSet;

use pdf_archive::MisusedProperty;
use pdf_model::xmp::{self, Detail, Name as XmpName};

use super::decision::Because;

/// ISO 19005-2 section 6.6.2.3.2's container property, `pdfaExtension:schemas`.
const CONTAINER: (&str, &str) = ("http://www.aiim.org/pdfa/ns/extension/", "schemas");

/// Section 6.6.2.3.3's Table 2 field naming the namespace a schema description describes.
const NAMESPACE_URI: (&str, &str) = ("http://www.aiim.org/pdfa/ns/schema#", "namespaceURI");

/// Why the container went, in the words a report prints beside it.
const CONTAINER_BECAUSE: &str = "this extension schema container left out a field ISO 19005-2 \
     section 6.6.2.3.3 requires and the file states nowhere, so it was discarded with every \
     property it described";

/// Why a property the container described went, in the words a report prints beside it.
const BECAUSE: &str = "the extension schema container describing this property's namespace left \
     out a field ISO 19005-2 section 6.6.2.3.3 requires and the file states nowhere, so the \
     container was discarded and every property it described went with it";

/// Why the packet could not be read or cut.
const NOT_CUT: &str = "this packet's extension schema container leaves out a field ISO 19005-2 \
     section 6.6.2.3.3 requires, and the packet could not be read or edited to discard the \
     container and the properties it describes";

/// Why the cut packet still fails.
const STILL_THERE: &str = "the extension schema container was cut from this packet and the \
     packet read back still states a container field ISO 19005-2 section 6.6.2.3.3 requires";

/// The packet without its container and the properties in the namespaces it described, and those
/// properties named as a report names a removal.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where the packet does not parse, the writer refuses the cut, or the
/// packet read back still fails the requirement.
pub(super) fn discarded(bytes: &[u8]) -> Result<(Vec<u8>, Vec<MisusedProperty>), Because> {
    let properties = xmp::Xmp::parse_detail(bytes).map_err(|_| Because::NotBuiltYet(NOT_CUT))?;
    let is = |name: &XmpName, (namespace, local): (&str, &str)| {
        name.namespace == namespace && name.local == local
    };
    let described: BTreeSet<String> = properties
        .iter()
        .filter(|property| is(&property.name, CONTAINER))
        .filter_map(|container| container.value.array())
        .flatten()
        .filter_map(|schema| schema.field(NAMESPACE_URI.0, NAMESPACE_URI.1))
        .filter_map(|uri| uri.value.text().map(str::to_owned))
        .collect();
    let mut names = Vec::new();
    let mut removed = Vec::new();
    // The container is a property too, and the first named: a schema describing no property
    // still loses its description, and that is what the history and the report owe a reader.
    for property in &properties {
        if !is(&property.name, CONTAINER) && !described.contains(&property.name.namespace) {
            continue;
        }
        removed.push(MisusedProperty {
            name: property.name.clone(),
            spelled: format!("{}:{}", property.prefix, property.name.local),
            stated: stated(&property.value),
            because: if is(&property.name, CONTAINER) {
                CONTAINER_BECAUSE
            } else {
                BECAUSE
            }
            .to_owned(),
        });
        names.push(property.name.clone());
    }
    let cut = xmp::remove(bytes, &names).map_err(|_| Because::NotBuiltYet(NOT_CUT))?;
    // Read back rather than trusted, `prepare_properties`'s rule: the writer says what it cut and
    // the packet says what it holds, and only the second is what a validator sees.
    if !pdf_archive::extension_container_fields(&cut).is_empty() {
        return Err(Because::NotBuiltYet(STILL_THERE));
    }
    Ok((cut, removed))
}

/// What a removed property stated, cut down to something a report can print.
fn stated(value: &Detail) -> String {
    match value.text() {
        Some(text) => text.chars().take(80).collect(),
        None => "a structured value".to_owned(),
    }
}
