//! What a redaction does to the XFA resource: the value of every field whose widget it removed is
//! taken out of the packets that hold it, or the resource goes whole (ISO 32000-2 §12.5.6.23,
//! §K.2; ADR 3789).
//!
//! A redaction that removes a field's widget removes the field's `/V`, `/RV` and `/DV` with it,
//! because the widget drew the value in the region and the default is what a reset would draw
//! there again (ADR 3725). A hybrid form holds the same value a second time: §K.2 binds the writer
//! of one,
//!
//! > The XFA field values shall be consistent with the corresponding V entries of the PDF field
//! > objects.
//!
//! and §12.5.6.23 binds the processor that applies a redaction to "remove all traces of the
//! specified content". So the value goes from each XFA place that is *this field's*, the three
//! the resource can hold it in:
//!
//! - **the `datasets` packet's data value the field binds**, found by XFA 3.3 chapter 4's binding
//!   (the template's `bind`, and the default `once` binding by name) exactly as the save that keeps
//!   §K.2's rule finds it ([`super::Datasets::keep`]);
//! - **the `form` packet's field**, the Form DOM written out (XFA 3.3 page 77's hierarchy of DOMs;
//!   page 262 says an unbound field's value is saved in it), found by the field's PDF name, which
//!   page 72 makes a SOM expression naming the Form DOM's containers;
//! - **the template's own `value`**, the field's default — the XFA counterpart of the `/DV` the
//!   redaction drops.
//!
//! **A value is emptied, never its node removed.** The binding is by name and by index among
//! same-named siblings, so taking a data value out of its data group would move every later
//! sibling of its name onto another field; an emptied data value is the null value, which is
//! what §K.2's consistency asks of a field whose `/V` is gone. A typed value's element (`text`,
//! `integer`, …) stays and its content goes, so the field keeps its content type.
//!
//! **Where the places cannot be found with certainty, the resource goes whole**, and the reason
//! is answered so the caller can say it: a form regenerated when opened or whose template is
//! dynamic, a template that does not name the field, a binding this module does not follow, a
//! configuration whose mapping rules it does not carry out, a data value another field outside the
//! region binds as well (emptying it would break §K.2's rule for that field), and a `form` packet
//! holding a field of the name that its path does not reach. Leaving the value would answer
//! neither clause, so an uncertain splice is never written.

use std::ops::Range;

use pdf_syntax::Document;
use pdf_syntax::object::{Object, ObjectId};

use super::template::{Class, Template};
use super::{Resource, packet, read_template, xml};

/// What a redaction leaves of a document's XFA resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redacted {
    /// The packets holding a removed field's value, each stream with its new text; empty where
    /// no packet held one.
    Spliced(Vec<Rewritten>),
    /// The resource cannot be cleared with certainty and goes whole, for this reason.
    Whole(String),
}

/// One packet stream with every value of a removed field emptied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rewritten {
    /// The stream: a packet of the array form, or the whole resource in the stream form.
    pub stream: ObjectId,
    /// Its decoded text, spliced: every byte the redaction did not empty is the producer's.
    pub text: String,
    /// The places emptied, each as a phrase naming the packet and the field, never the value.
    pub places: Vec<String>,
}

/// One packet as read: its stream, its text and its elements.
struct Packet {
    stream: ObjectId,
    text: String,
    tree: xml::Tree,
}

/// The ranges of one stream's text to empty, and the phrases naming them.
#[derive(Default)]
struct Splices {
    ranges: Vec<Range<usize>>,
    places: Vec<String>,
}

/// What a redaction that removed the widgets of `fields` — §12.7.4.2's qualified names — leaves
/// of the document's XFA resource; `None` where the document states none.
#[must_use]
pub fn redacted(document: &Document, fields: &[String]) -> Option<Redacted> {
    let stated = super::stated(document).filter(|stated| stated.resource)?;
    if fields.is_empty() {
        return Some(Redacted::Spliced(Vec::new()));
    }
    let whole = |reason: String| Some(Redacted::Whole(reason));
    if stated.needs_rendering {
        return whole(
            "the document asks to be regenerated when opened (Table 29's /NeedsRendering), so \
             its XFA form is a dynamic one whose data this program does not bind"
                .to_owned(),
        );
    }
    let resource = match Resource::read(document) {
        Some(Ok(resource)) => resource,
        Some(Err(error)) => return whole(error.to_string()),
        None => return whole("the XFA entry is not a resource this program reads".to_owned()),
    };
    if let Some(why) = resource.template.dynamic() {
        return whole(format!("its XFA form is a dynamic one: {why}"));
    }
    if let Some(reason) = resource.config.unfollowed() {
        return whole(reason);
    }
    let (template_packet, form_packet) = match other_packets(document, &resource) {
        Ok(packets) => packets,
        Err(reason) => return whole(reason),
    };
    match splices(
        document,
        &resource,
        &template_packet,
        form_packet.as_ref(),
        fields,
    ) {
        Ok(by_stream) => Some(Redacted::Spliced(rewritten(
            &resource,
            &template_packet,
            form_packet.as_ref(),
            by_stream,
        ))),
        Err(reason) => whole(reason),
    }
}

/// The template's packet and the `form` packet where the resource holds one: in the stream form
/// both are elements of the one document the `datasets` packet is read from, and in the array
/// form each is a stream of its own.
fn other_packets(
    document: &Document,
    resource: &Resource,
) -> Result<(Packet, Option<(Packet, usize)>), String> {
    let datasets = &resource.datasets;
    let catalog = document.catalog().map_err(|error| error.to_string())?;
    let form = document.get_key(&catalog, "AcroForm");
    let entry = form
        .as_dict()
        .map_or(Object::Null, |form| document.get_key(form, "XFA"));
    let Object::Array(packets) = entry else {
        // The stream form: one document, which `Resource::read` parsed as the datasets packet.
        let whole = || Packet {
            stream: resource.datasets_stream,
            text: datasets.text().to_owned(),
            tree: datasets.tree().clone(),
        };
        let form = datasets.tree().child(0, "form").map(|root| (whole(), root));
        return Ok((whole(), form));
    };
    let named = |wanted: &str| {
        packets
            .as_chunks::<2>()
            .0
            .iter()
            .find_map(|pair| match pair {
                [Object::String(name), Object::Reference(id)]
                    if pdf_syntax::text_string(name) == wanted =>
                {
                    Some(*id)
                }
                _ => None,
            })
    };
    let read = |id: ObjectId, name: &'static str| {
        packet(document, id, name)
            .map(|(text, tree)| Packet {
                stream: id,
                text,
                tree,
            })
            .map_err(|error| error.to_string())
    };
    let template_id = named("template").ok_or("the XFA resource has no template packet")?;
    let template = read(template_id, "template")?;
    let form = match named("form") {
        Some(id) => {
            let form = read(id, "form")?;
            let root = form
                .tree
                .first("form")
                .ok_or("the XFA form packet holds no form element")?;
            Some((form, root))
        }
        None => None,
    };
    Ok((template, form))
}

/// Every range to empty, by stream, for the values of `fields` in the three places.
fn splices(
    document: &Document,
    resource: &Resource,
    template_packet: &Packet,
    form_packet: Option<&(Packet, usize)>,
    fields: &[String],
) -> Result<Vec<(ObjectId, Splices)>, String> {
    let template = &resource.template;
    let datasets = &resource.datasets;
    let form_template = form_packet.map(|(packet, root)| {
        (
            packet,
            read_template(document, packet.stream, &packet.tree, *root),
        )
    });
    let mut by_stream: Vec<(ObjectId, Splices)> = Vec::new();
    let mut emptied_data: Vec<usize> = Vec::new();
    for field in fields {
        let target = template
            .resolve(field)
            .ok_or_else(|| format!("its template names no field {field}"))?;

        let ranges = value_ranges(&template_packet.tree, template, target);
        if !ranges.is_empty() {
            let splices = splices_of(&mut by_stream, template_packet.stream);
            splices.ranges.extend(ranges);
            splices
                .places
                .push(format!("the template's default value of field {field}"));
        }

        let bound = datasets
            .bound(template, target)
            .map_err(|why| format!("the data field {field} binds is not found: {why}"))?;
        if let Some(element) = bound
            && let Some(range) = datasets.tree().get(element).and_then(content)
        {
            emptied_data.push(element);
            let splices = splices_of(&mut by_stream, resource.datasets_stream);
            splices.ranges.push(range);
            splices
                .places
                .push(format!("the datasets packet's data value of field {field}"));
        }

        if let Some((packet, form)) = &form_template {
            let ranges = match form.resolve(field) {
                Some(found) => value_ranges(&packet.tree, form, found),
                None if names_a_field(&packet.tree, field) => {
                    return Err(format!(
                        "its form packet holds a field named as {field} is that the field's \
                         path does not reach"
                    ));
                }
                None => Vec::new(),
            };
            if !ranges.is_empty() {
                let splices = splices_of(&mut by_stream, packet.stream);
                splices.ranges.extend(ranges);
                splices
                    .places
                    .push(format!("the form packet's value of field {field}"));
            }
        }
    }
    shared_elsewhere(document, template, datasets, fields, &emptied_data)?;
    Ok(by_stream)
}

/// The splices of `stream`, added where it has none yet.
fn splices_of(by_stream: &mut Vec<(ObjectId, Splices)>, stream: ObjectId) -> &mut Splices {
    let index = if let Some(index) = by_stream.iter().position(|(id, _)| *id == stream) {
        index
    } else {
        by_stream.push((stream, Splices::default()));
        by_stream.len().saturating_sub(1)
    };
    // `index` is a position the vector holds: found by `position`, or the element just pushed.
    &mut by_stream[index].1
}

/// Refuses a splice that would empty a data value a field outside the redaction binds too: §K.2
/// would then be broken for that field, whose `/V` the redaction keeps.
fn shared_elsewhere(
    document: &Document,
    template: &Template,
    datasets: &super::Datasets,
    fields: &[String],
    emptied: &[usize],
) -> Result<(), String> {
    if emptied.is_empty() {
        return Ok(());
    }
    for other in crate::view::widgets_by_field_name(document).into_keys() {
        if fields.contains(&other) {
            continue;
        }
        let Some(target) = template.resolve(&other) else {
            continue;
        };
        if let Ok(Some(element)) = datasets.bound(template, target)
            && emptied.contains(&element)
        {
            return Err(format!(
                "its data value is bound by field {other} as well, which the redaction keeps"
            ));
        }
    }
    Ok(())
}

/// Whether any field or exclusion group of the packet is named as the last step of `field`.
fn names_a_field(tree: &xml::Tree, field: &str) -> bool {
    // not a repair: a step of a name this program holds, not a value the file states.
    let last = field.rsplit('.').next().unwrap_or(field);
    let name = last.split_once('[').map_or(last, |(name, _)| name);
    tree.elements.iter().any(|element| {
        matches!(element.local.as_str(), "field" | "exclGroup")
            && element.attribute("name") == Some(name)
    })
}

/// The content of an element, where it has any.
fn content(element: &xml::Element) -> Option<Range<usize>> {
    element.content.clone().filter(|range| !range.is_empty())
}

/// The ranges that hold the value of container `target` of `template`, read from `tree`: the
/// content of each element its `value` holds — the typed element kept, its content emptied — and
/// for an exclusion group the same of each of its fields.
fn value_ranges(tree: &xml::Tree, template: &Template, target: usize) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut pending = vec![target];
    while let Some(at) = pending.pop() {
        let Some(container) = template.get(at) else {
            continue;
        };
        match container.class {
            Class::ExclGroup => pending.extend(container.children.iter().copied()),
            Class::Field => {}
            _ => continue,
        }
        let Some(value) = tree
            .child(container.element, "value")
            .and_then(|value| tree.get(value))
        else {
            continue;
        };
        if !value.text.trim().is_empty() {
            out.extend(content(value));
            continue;
        }
        out.extend(
            value
                .children
                .iter()
                .filter_map(|&child| tree.get(child).and_then(content)),
        );
    }
    out
}

/// Each stream's text with its ranges emptied, the packets read once more by stream.
fn rewritten(
    resource: &Resource,
    template_packet: &Packet,
    form_packet: Option<&(Packet, usize)>,
    by_stream: Vec<(ObjectId, Splices)>,
) -> Vec<Rewritten> {
    let text_of = |stream: ObjectId| -> &str {
        if stream == resource.datasets_stream {
            resource.datasets.text()
        } else if stream == template_packet.stream {
            &template_packet.text
        } else {
            form_packet.map_or("", |(packet, _)| packet.text.as_str())
        }
    };
    by_stream
        .into_iter()
        .map(|(stream, splices)| Rewritten {
            stream,
            text: emptied(text_of(stream), splices.ranges),
            places: splices.places,
        })
        .collect()
}

/// `text` with every range emptied; a range inside another is the other's.
fn emptied(text: &str, mut ranges: Vec<Range<usize>>) -> String {
    ranges.sort_by_key(|range| (range.start, std::cmp::Reverse(range.end)));
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for range in ranges {
        if range.start < from {
            continue;
        }
        // not a repair: the ranges are the parser's own, on character boundaries of `text`.
        out.push_str(text.get(from..range.start).unwrap_or_default());
        from = range.end;
    }
    // not a repair: as above, the end of the last range this parser handed out.
    out.push_str(text.get(from..).unwrap_or_default());
    out
}

#[cfg(test)]
mod tests {
    use super::emptied;

    #[test]
    fn a_range_inside_another_is_the_other_s() {
        assert_eq!(emptied("<a><b>x</b>y</a>", vec![6..7, 3..12]), "<a></a>");
    }

    #[test]
    fn every_byte_outside_the_ranges_is_kept() {
        assert_eq!(
            emptied("<v>1</v><w>2</w>", vec![11..12, 3..4]),
            "<v></v><w></w>"
        );
    }
}
