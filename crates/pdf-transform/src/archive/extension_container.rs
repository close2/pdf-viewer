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
//! **The incomplete description rather than the whole container, where the container can say
//! which it is.** A container holding complete and incomplete schema descriptions loses only the
//! incomplete ones, by `pdf_model::xmp::remove_items`, and with them only the properties of the
//! namespaces no description left behind describes — a complete description's properties are
//! still described, so nothing asks for them to go (`doc/adr/2131`). The whole container goes
//! where every description in it is incomplete, and where the packet does not state exactly one
//! container whose value is a bag: then `pdf_archive::incomplete_schema_descriptions` cannot say
//! which items of which bag are the incomplete ones. The report names every description and
//! every property either cut takes, so the cost is visible where it is paid.

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

/// Why one schema description went, in the words a report prints beside it.
const DESCRIPTION_BECAUSE: &str = "this schema description in the extension schema container \
     left out a field ISO 19005-2 section 6.6.2.3.3 requires and the file states nowhere, so it \
     was discarded with every property only it described; the container's other descriptions \
     stay";

/// Why a property only a discarded description described went.
const DESCRIBED_BECAUSE: &str = "the only schema description of this property's namespace left \
     out a field ISO 19005-2 section 6.6.2.3.3 requires and the file states nowhere, so the \
     description was discarded and the properties it described went with it";

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
     packet read back still leaves out a container field ISO 19005-2 section 6.6.2.3.3 requires";

/// The packet without its container and the properties in the namespaces it described, and those
/// properties named as a report names a removal.
///
/// # Errors
///
/// [`Because::NotBuiltYet`] where the packet does not parse, the writer refuses the cut, or the
/// packet read back still fails the requirement.
///
/// What is left may still misspell a field of a description it kept; that is the respelling's to
/// answer, and the caller asks it of the packet this returns.
pub(super) fn discarded(bytes: &[u8]) -> Result<Cut, Because> {
    let properties = xmp::Xmp::parse_detail(bytes).map_err(|_| Because::NotBuiltYet(NOT_CUT))?;
    let is = |name: &XmpName, (namespace, local): (&str, &str)| {
        name.namespace == namespace && name.local == local
    };
    if let Some(cut) = narrower(bytes, &properties)? {
        return Ok(cut);
    }
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
    read_back(cut, removed)
}

/// A cut packet, and every description and property the cut took, as a report names them.
type Cut = (Vec<u8>, Vec<MisusedProperty>);

/// The cut that takes only the incomplete descriptions, where the container is one bag holding
/// complete ones beside them; `None` where the whole container is the cut.
fn narrower(bytes: &[u8], properties: &[xmp::Property]) -> Result<Option<Cut>, Because> {
    let Some(incomplete) = pdf_archive::incomplete_schema_descriptions(bytes) else {
        return Ok(None);
    };
    let Some(container) = properties.iter().find(|property| {
        property.name.namespace == CONTAINER.0 && property.name.local == CONTAINER.1
    }) else {
        return Ok(None);
    };
    let schemas = container.value.array().unwrap_or_default();
    if incomplete.is_empty() || incomplete.len() >= schemas.len() {
        return Ok(None);
    }
    let namespace_of = |schema: &Detail| {
        schema
            .field(NAMESPACE_URI.0, NAMESPACE_URI.1)
            .and_then(|uri| uri.value.text().map(str::to_owned))
    };
    let mut cut_namespaces = BTreeSet::new();
    let mut kept_namespaces = BTreeSet::new();
    let mut removed = Vec::new();
    for (position, schema) in schemas.iter().enumerate() {
        let namespace = namespace_of(schema);
        if !incomplete.contains(&position) {
            kept_namespaces.extend(namespace);
            continue;
        }
        removed.push(MisusedProperty {
            name: container.name.clone(),
            spelled: format!("{}:{}", container.prefix, container.name.local),
            stated: format!(
                "the schema description of {}",
                namespace
                    .as_deref()
                    .unwrap_or("a namespace it does not state")
            ),
            because: DESCRIPTION_BECAUSE.to_owned(),
        });
        cut_namespaces.extend(namespace);
    }
    // A namespace another description in the container still describes keeps its properties:
    // section 6.6.2.3.1 asks for a description, and one is still there.
    let gone: BTreeSet<&String> = cut_namespaces.difference(&kept_namespaces).collect();
    let mut names = Vec::new();
    for property in properties {
        if !gone.contains(&property.name.namespace) {
            continue;
        }
        removed.push(MisusedProperty {
            name: property.name.clone(),
            spelled: format!("{}:{}", property.prefix, property.name.local),
            stated: stated(&property.value),
            because: DESCRIBED_BECAUSE.to_owned(),
        });
        names.push(property.name.clone());
    }
    let cut = xmp::remove_items(bytes, &container.name, &incomplete)
        .and_then(|cut| xmp::remove(&cut, &names))
        .map_err(|_| Because::NotBuiltYet(NOT_CUT))?;
    read_back(cut, removed).map(Some)
}

/// The cut packet, where the packet read back leaves out no field — read back rather than
/// trusted, `prepare_properties`'s rule: the writer says what it cut and the packet says what it
/// holds, and only the second is what a validator sees. A misspelling a kept description still
/// carries is not this cut's to judge.
fn read_back(cut: Vec<u8>, removed: Vec<MisusedProperty>) -> Result<Cut, Because> {
    if pdf_archive::extension_container_fields(&cut)
        .iter()
        .any(|fault| !fault.misspelled)
    {
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
