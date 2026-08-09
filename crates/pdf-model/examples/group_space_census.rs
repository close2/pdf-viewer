//! Which space a *painted* group actually composites in — §11.4.7's page group and §11.6.6's
//! inheritance, rather than whatever `/CS` a group dictionary happens to carry.
//!
//! A group's declared `/CS` is not always the space anything composites in, and this counts the
//! space that is. Two sentences decide it. §11.6.6 gives the entry effect "[f]or isolated
//! groups" and then:
//!
//! > For non-isolated groups, or if no group colour space is specified, the group colour space
//! > shall be inherited from the parent group or page.
//!
//! and §11.4.7 makes the root of that inheritance the page group, whose own `/CS` "shall serve
//! as the default blending colour space for each page".
//!
//! So a non-isolated group's `/DeviceCMYK` is *not* a departure unless something above it says
//! so, and a group with no `/CS` at all inside an isolated `/DeviceCMYK` one *is*. This census
//! prints the effective space at every painted group, where it was introduced, and the evidence
//! about what the group does inside it — the blend modes its own `/ExtGState`s name and the
//! largest `cyan + black` any `k` operator sets, which is where §10.4.2.5's `min` would bite.
//!
//! ```sh
//! cargo run --release -p pdf-model --example group_space_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_model::colour::ColourSpace;
use pdf_syntax::{Dictionary, Document, Lexer, Object, ObjectId, Stream, Token};

/// How far a form `XObject`'s own resources are followed.
const MAX_DEPTH: usize = 8;

/// How many pages of one document are walked.
const MAX_PAGES: usize = 100;

/// What one painted group was found to declare and to hold.
struct Group {
    /// Table 145's `/I`.
    isolated: bool,
    /// Table 145's `/K`.
    knockout: bool,
    /// Table 145's `/CS`, as written, with its component count.
    declared: String,
    /// The space compositing inside this group actually happens in, after §11.6.6's inheritance.
    effective: String,
    /// Whether this group is where that space is introduced, rather than one that inherited it.
    introduces: bool,
    /// Every blend mode any `/ExtGState` the group's own resources reach names.
    blends: BTreeSet<String>,
    /// Every content-stream operator that sets a colour or paints something not a path.
    operators: BTreeSet<String>,
    /// The `/ColorSpace` of every `XObject` and shading the group's own resources hold.
    drawn: BTreeSet<String>,
    /// The largest `cyan + black`, `magenta + black` or `yellow + black` any `k`/`K` sets.
    channel_sum: f32,
}

/// Whether a space composites in the three components the device raster already holds.
///
/// The same classification `pdf_model`'s own `space_departure` makes: an RGB space of three
/// components — `/DeviceRGB`, `CalRGB` or a three-component ICC profile — asks for exactly what
/// the device raster does, and every other space asks for different arithmetic. The marker
/// `(3 RGB components)` is written into the description by [`describe_space`] so that this can
/// be one test rather than two lists that drift.
fn is_device_rgb(described: &str) -> bool {
    described.contains("(3 RGB components)") || described.starts_with("the device's")
}

#[expect(
    clippy::too_many_lines,
    reason = "one census printing one table; splitting it would hide what is counted"
)]
fn main() {
    let mut documents = 0_usize;
    let mut page_groups = 0_usize;
    let mut departing_pages = 0_usize;
    let mut with_groups = 0_usize;
    let mut declared: BTreeMap<String, usize> = BTreeMap::new();
    let mut effective: BTreeMap<String, usize> = BTreeMap::new();
    let mut introduced = 0_usize;
    let mut lines: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        // A mask group is §11.5.3's population and is closed (ADRs 0217, 0220); this census is
        // about the painted ones, so the groups an `/SMask` names are collected first and
        // skipped below.
        let mut masks: BTreeSet<ObjectId> = BTreeSet::new();
        for index in 0..pages.len().min(MAX_PAGES) {
            if let Some(page) = pages.get(index) {
                let mut seen = BTreeSet::new();
                mask_groups(&document, &page.resources, 0, &mut seen, &mut masks);
            }
        }
        let mut groups: Vec<Group> = Vec::new();
        let mut page_spaces: BTreeSet<String> = BTreeSet::new();
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            // §11.4.7: the page group is the root of §11.6.6's inheritance, and its `/CS` "shall
            // serve as the default blending colour space for each page".
            let page_space = group_space(&document, &page.dict)
                .unwrap_or_else(|| "the device's (§11.4.7)".to_owned());
            page_spaces.insert(page_space.clone());
            let mut seen = BTreeSet::new();
            walk(
                &document,
                &page.resources,
                0,
                &page_space,
                &masks,
                &mut seen,
                &mut groups,
            );
        }
        for space in &page_spaces {
            if space.starts_with("the device's") {
                continue;
            }
            page_groups = page_groups.saturating_add(1);
            if !is_device_rgb(space) {
                departing_pages = departing_pages.saturating_add(1);
                lines.push(format!("  {name}: page group /CS {space} (§11.4.7)"));
            }
        }
        if groups.is_empty() {
            continue;
        }
        with_groups = with_groups.saturating_add(1);
        for group in &groups {
            let counter = declared.entry(group.declared.clone()).or_default();
            *counter = counter.saturating_add(1);
            let counter = effective.entry(group.effective.clone()).or_default();
            *counter = counter.saturating_add(1);
            if is_device_rgb(&group.effective) {
                continue;
            }
            if group.introduces {
                introduced = introduced.saturating_add(1);
            }
            lines.push(format!(
                "  {name}: declared {} /I {} /K {} -> composites in {} ({}) \
                 max(c+k, m+k, y+k) {:.3} blends [{}] {} [{}]",
                group.declared,
                group.isolated,
                group.knockout,
                group.effective,
                if group.introduces {
                    "introduced here"
                } else {
                    "inherited"
                },
                group.channel_sum,
                group.blends.iter().cloned().collect::<Vec<_>>().join(", "),
                group
                    .operators
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" "),
                group.drawn.iter().cloned().collect::<Vec<_>>().join(", "),
            ));
        }
    }

    println!("{documents} document(s) opened");
    println!("  {page_groups} page group(s) state a /CS, {departing_pages} of them not RGB");
    println!("  {with_groups} document(s) hold a painted transparency group");
    println!("  declared /CS: {declared:?}");
    println!("  effective space: {effective:?}");
    println!("  {introduced} group(s) introduce a space that is not a three-component RGB one");
    for line in &lines {
        println!("{line}");
    }
}

/// A `/Group` dictionary's `/CS`, where the dictionary is a transparency group with one.
fn group_space(document: &Document, dict: &Dictionary) -> Option<String> {
    let attributes = document.get_key(dict, "Group");
    let attributes = attributes.as_dict()?;
    if document.get_key(attributes, "S").as_name()?.as_bytes() != b"Transparency" {
        return None;
    }
    let entry = document.get_key(attributes, "CS");
    if matches!(entry, Object::Null) {
        return None;
    }
    Some(describe_space(document, &entry))
}

/// One space, named as the file writes it and classified as the interpreter classifies it.
fn describe_space(document: &Document, entry: &Object) -> String {
    let parsed = ColourSpace::parse(document, entry, &Dictionary::new());
    let rgb = matches!(parsed, Some(ColourSpace::Rgb | ColourSpace::CalRgb { .. }))
        || matches!(&parsed, Some(ColourSpace::Icc { profile }) if profile.channels() == 3);
    format!(
        "{} ({})",
        describe(document, entry),
        parsed.as_ref().map_or_else(
            || "unparsed".to_owned(),
            |space| format!(
                "{} {}components",
                space.components(),
                if rgb { "RGB " } else { "" }
            )
        )
    )
}

/// Every group an `/SMask` in a reachable `/ExtGState` names.
fn mask_groups(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    seen: &mut BTreeSet<ObjectId>,
    masks: &mut BTreeSet<ObjectId>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    if let Some(states) = document.get_key(resources, "ExtGState").as_dict() {
        for (_, value) in states.iter() {
            let resolved = document.resolve(value);
            let Some(state) = resolved.as_dict() else {
                continue;
            };
            let mask = document.get_key(state, "SMask");
            if let Some(mask) = mask.as_dict()
                && let Some(id) = mask.get("G").and_then(Object::as_reference)
            {
                masks.insert(id);
            }
        }
    }
    let mut forms: Vec<Stream> = Vec::new();
    for_each_form(document, resources, seen, |_, _, stream| {
        forms.push(stream.clone());
    });
    for stream in forms {
        if let Some(inner) = document.get_key(&stream.dict, "Resources").as_dict() {
            mask_groups(document, inner, depth.saturating_add(1), seen, masks);
        }
    }
}

/// Every painted transparency group reachable from one resource dictionary.
fn walk(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    inherited: &str,
    masks: &BTreeSet<ObjectId>,
    seen: &mut BTreeSet<ObjectId>,
    groups: &mut Vec<Group>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let mut forms: Vec<(Option<ObjectId>, Stream)> = Vec::new();
    for_each_form(document, resources, seen, |_, id, stream| {
        forms.push((id, stream.clone()));
    });
    for (id, stream) in forms {
        let attributes = document.get_key(&stream.dict, "Group");
        let transparency = attributes.as_dict().is_some_and(|attributes| {
            document
                .get_key(attributes, "S")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Transparency")
        });
        let is_mask = id.is_some_and(|id| masks.contains(&id));
        let mut inside = inherited.to_owned();
        if transparency && !is_mask {
            let group = read_group(document, &stream, inherited);
            inside.clone_from(&group.effective);
            groups.push(group);
        }
        if let Some(inner) = document.get_key(&stream.dict, "Resources").as_dict() {
            walk(
                document,
                inner,
                depth.saturating_add(1),
                &inside,
                masks,
                seen,
                groups,
            );
        }
    }
}

/// Calls `visit` for every form `XObject` one resource dictionary names, once per object.
fn for_each_form(
    document: &Document,
    resources: &Dictionary,
    seen: &mut BTreeSet<ObjectId>,
    mut visit: impl FnMut(&Document, Option<ObjectId>, &Stream),
) {
    let objects = document.get_key(resources, "XObject");
    let Some(dict) = objects.as_dict() else {
        return;
    };
    for (_, value) in dict.iter() {
        let id = value.as_reference();
        if let Some(id) = id
            && !seen.insert(id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        if let Object::Stream(stream) = &resolved {
            visit(document, id, stream);
        }
    }
}

/// Reads one painted group's declaration, its effective space and what its content paints.
#[expect(
    clippy::too_many_lines,
    reason = "one census reading one group; splitting it would hide what is counted"
)]
fn read_group(document: &Document, stream: &Stream, inherited: &str) -> Group {
    let attributes = document.get_key(&stream.dict, "Group");
    let attributes = attributes.as_dict().cloned().unwrap_or_default();
    let isolated = matches!(document.get_key(&attributes, "I"), Object::Boolean(true));
    let declared = group_space(document, &stream.dict);
    // §11.6.6: the `/CS` takes effect "[f]or isolated groups"; otherwise the space "shall be
    // inherited from the parent group or page".
    let (effective, introduces) = match (&declared, isolated) {
        (Some(space), true) => (space.clone(), space != inherited),
        _ => (inherited.to_owned(), false),
    };
    let mut group = Group {
        isolated,
        knockout: matches!(document.get_key(&attributes, "K"), Object::Boolean(true)),
        declared: declared.unwrap_or_else(|| "absent".to_owned()),
        effective,
        introduces,
        blends: BTreeSet::new(),
        operators: BTreeSet::new(),
        drawn: BTreeSet::new(),
        channel_sum: 0.0,
    };

    if let Some(inner) = document.get_key(&stream.dict, "Resources").as_dict() {
        if let Some(states) = document.get_key(inner, "ExtGState").as_dict() {
            for (_, value) in states.iter() {
                let resolved = document.resolve(value);
                let Some(state) = resolved.as_dict() else {
                    continue;
                };
                match document.get_key(state, "BM") {
                    Object::Name(name) => {
                        group
                            .blends
                            .insert(String::from_utf8_lossy(name.as_bytes()).into_owned());
                    }
                    Object::Array(items) => {
                        for item in &items {
                            if let Object::Name(name) = document.resolve(item) {
                                group
                                    .blends
                                    .insert(String::from_utf8_lossy(name.as_bytes()).into_owned());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for category in ["Shading", "XObject"] {
            let entries = document.get_key(inner, category);
            let Some(entries) = entries.as_dict() else {
                continue;
            };
            for (_, value) in entries.iter() {
                let resolved = document.resolve(value);
                let dict = match &resolved {
                    Object::Dictionary(dict) => dict.clone(),
                    Object::Stream(stream) => stream.dict.clone(),
                    _ => continue,
                };
                let space = document.get_key(&dict, "ColorSpace");
                let subtype = document.get_key(&dict, "Subtype");
                let kind = subtype.as_name().map_or_else(
                    || category.to_owned(),
                    |name| String::from_utf8_lossy(name.as_bytes()).into_owned(),
                );
                group
                    .drawn
                    .insert(format!("{kind} {}", describe(document, &space)));
            }
        }
    }

    let Some(content) = document.decoded_stream_data(stream) else {
        group.operators.insert("<undecodable>".to_owned());
        return group;
    };
    let mut lexer = Lexer::new(&content);
    let mut operands: Vec<f32> = Vec::new();
    while let Some(token) = lexer.next_token() {
        match token {
            #[expect(
                clippy::cast_precision_loss,
                reason = "a content-stream operand outside f32's range is not a colour"
            )]
            Token::Integer(value) => operands.push(value as f32),
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a content-stream operand outside f32's range is not a colour"
            )]
            Token::Real(value) => operands.push(value as f32),
            Token::Keyword(operator) => {
                let operator = String::from_utf8_lossy(&operator).into_owned();
                if matches!(
                    operator.as_str(),
                    "k" | "K"
                        | "g"
                        | "G"
                        | "rg"
                        | "RG"
                        | "sc"
                        | "SC"
                        | "scn"
                        | "SCN"
                        | "cs"
                        | "CS"
                        | "sh"
                        | "Do"
                        | "BI"
                        | "gs"
                ) {
                    group.operators.insert(operator.clone());
                }
                if (operator == "k" || operator == "K")
                    && let [cyan, magenta, yellow, black] = operands[..]
                {
                    // §10.4.2.5 adds the black component to each of the other three and clamps
                    // the sum at 1.0, so this is where the *classic* conversion stops being
                    // affine. This tree's conversion is multilinear over the ink cube and is
                    // not affine anywhere, which ADR 0251 measures — the column is kept because
                    // it is what the classic reading would turn on.
                    let sum = cyan.max(magenta).max(yellow) + black;
                    group.channel_sum = group.channel_sum.max(sum);
                }
                operands.clear();
            }
            _ => operands.clear(),
        }
    }
    group
}

/// A colour space object as a person would name it, without resolving it to components.
fn describe(document: &Document, entry: &Object) -> String {
    match document.resolve(entry) {
        Object::Null => "absent".to_owned(),
        Object::Name(name) => format!("/{}", String::from_utf8_lossy(name.as_bytes())),
        Object::Array(items) => items.first().map_or_else(
            || "an empty array".to_owned(),
            |first| match document.resolve(first) {
                Object::Name(name) => {
                    format!("[/{} …]", String::from_utf8_lossy(name.as_bytes()))
                }
                _ => "an array".to_owned(),
            },
        ),
        _ => "something else".to_owned(),
    }
}
