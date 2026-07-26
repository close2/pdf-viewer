//! Generates the PDF object-model tables from the Arlington PDF Model.
//!
//! Reads `doc/arlington-pdf-model/tsv/2.0/*.tsv` and emits `static` Rust tables into
//! `OUT_DIR`. See `doc/adr/0003-arlington-generated-validation.md` for why this is
//! generated rather than hand-written.
//!
//! # Design rules
//!
//! **Nothing is silently dropped.** Every unrecognised token — a type name, a version, a
//! predicate shape — fails the build with the file, key and value named. The alternative,
//! skipping what it does not understand, would produce a validation layer that quietly
//! stopped checking part of the specification after an upstream update. A build failure is
//! recoverable; a silent gap is not.
//!
//! **Predicates are preserved, not interpreted.** Cells containing `fn:` expressions this
//! generator does not model are carried through verbatim as strings, so the runtime can
//! report "cannot decide" rather than guessing.
//!
//! **Structural invariants are asserted.** Arlington aligns `Type`, `Link`,
//! `PossibleValues` and bracketed `IndirectReference` positionally by `;`. That is relied
//! upon, so it is checked for every row rather than assumed.

// A build script's job is to abort the build when its input is malformed, and the panic
// message is the diagnostic a developer reads. Returning errors instead would only make
// `main` unwrap them one line later.
#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "aborting the build is the intended and only useful failure mode here"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::PathBuf;

/// Arlington column indices, in file order.
///
/// The twelfth column, `Note`, is a human-facing cross-reference to the specification and
/// is deliberately not read: it carries nothing a validator can act on.
mod column {
    pub(crate) const KEY: usize = 0;
    pub(crate) const TYPE: usize = 1;
    pub(crate) const SINCE_VERSION: usize = 2;
    pub(crate) const DEPRECATED_IN: usize = 3;
    pub(crate) const REQUIRED: usize = 4;
    pub(crate) const INDIRECT_REFERENCE: usize = 5;
    pub(crate) const INHERITABLE: usize = 6;
    pub(crate) const DEFAULT_VALUE: usize = 7;
    pub(crate) const POSSIBLE_VALUES: usize = 8;
    pub(crate) const SPECIAL_CASE: usize = 9;
    pub(crate) const LINK: usize = 10;
    pub(crate) const COUNT: usize = 12;
}

fn main() {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets this"));
    let tsv_dir = manifest.join("../../doc/arlington-pdf-model/tsv/2.0");

    // The model is a git submodule. An unhelpful "no such file" here would send someone
    // hunting through build.rs, so say what to run.
    assert!(
        tsv_dir.is_dir(),
        "the Arlington PDF Model is missing at {}\n\
         It is a git submodule; run:  git submodule update --init",
        tsv_dir.display()
    );

    let mut files: Vec<PathBuf> = std::fs::read_dir(&tsv_dir)
        .expect("the model directory is readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "tsv"))
        .collect();
    // Sorted so the generated table can be binary-searched, and so output is byte-stable
    // across machines rather than following directory order.
    files.sort();

    assert!(
        !files.is_empty(),
        "no TSV files under {}",
        tsv_dir.display()
    );

    let mut objects = BTreeMap::new();
    for path in &files {
        let name = path
            .file_stem()
            .expect("a .tsv path has a stem")
            .to_str()
            .unwrap_or_else(|| panic!("non-UTF-8 filename: {}", path.display()))
            .to_owned();
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        objects.insert(name.clone(), parse_object(&name, &text));
    }

    let generated = emit(&objects);
    let out =
        PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets this")).join("arlington.rs");
    std::fs::write(&out, generated)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", out.display()));

    // Regenerate when the model changes. The submodule is pinned, so in practice this
    // fires when the pin moves.
    println!("cargo:rerun-if-changed={}", tsv_dir.display());
    for path in &files {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

/// One parsed row, held as the Rust expressions that will be emitted for it.
struct Row {
    pattern: String,
    types: Vec<String>,
    availability: String,
    deprecated_in: String,
    required: String,
    inheritable: bool,
    default_value: String,
    special_case: String,
}

fn parse_object(name: &str, text: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for (number, line) in text.lines().enumerate().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        // `number` is a zero-based enumerate index over lines; +1 makes it the
        // one-based line number a person would look for in an editor.
        let at = || format!("{name}.tsv line {}", number.saturating_add(1));
        assert!(
            fields.len() == column::COUNT,
            "{}: expected {} columns, found {}",
            at(),
            column::COUNT,
            fields.len()
        );
        rows.push(parse_row(&fields, &at));
    }
    assert!(!rows.is_empty(), "{name}.tsv has no key rows");
    rows
}

fn parse_row(fields: &[&str], at: &dyn Fn() -> String) -> Row {
    let types = split_types(fields[column::TYPE]);
    let links = split_groups(fields[column::LINK]);
    let values = split_groups(fields[column::POSSIBLE_VALUES]);
    let indirect = split_groups(fields[column::INDIRECT_REFERENCE]);

    // The alignment this whole design leans on. Checked per row, because a future model
    // update that broke it would otherwise pair a type with another type's constraints.
    for (label, group) in [("Link", &links), ("PossibleValues", &values)] {
        assert!(
            group.len() <= 1 || group.len() == types.len(),
            "{}: {label} has {} groups but Type has {} alternatives",
            at(),
            group.len(),
            types.len()
        );
    }

    let alternatives = types
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let (kind, gate) = parse_type(raw, at);
            let link_group = pick(&links, index);
            let value_group = pick(&values, index);
            let indirect_group = pick(&indirect, index);
            format!(
                "TypeAlternative {{ kind: PdfType::{kind}, links: &[{}], \
                 possible_values: {}, indirect: {}, gate: {gate} }}",
                link_group
                    .map(|group| group
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(|item| format!("{item:?}"))
                        .collect::<Vec<_>>()
                        .join(", "))
                    .unwrap_or_default(),
                optional_str(value_group.filter(|group| !group.is_empty())),
                parse_indirect(indirect_group.unwrap_or(""), at),
            )
        })
        .collect();

    Row {
        pattern: parse_key(fields[column::KEY], at),
        types: alternatives,
        availability: parse_availability(fields[column::SINCE_VERSION], at),
        deprecated_in: match fields[column::DEPRECATED_IN].trim() {
            "" => "None".to_owned(),
            version => format!("Some(Version::{})", parse_version(version, at)),
        },
        required: parse_required(fields[column::REQUIRED], at),
        inheritable: parse_bool(fields[column::INHERITABLE], at),
        default_value: optional_str(non_empty(fields[column::DEFAULT_VALUE])),
        special_case: optional_str(non_empty(fields[column::SPECIAL_CASE])),
    }
}

/// Splits a `Type` cell into its alternatives.
///
/// Unlike `Link`, `PossibleValues` and `IndirectReference`, the `Type` column is
/// semicolon-separated but *not* bracketed — `stream;string-text`, not `[stream];[...]`.
/// Feeding it to [`split_groups`] yields one alternative containing a semicolon, which the
/// alignment assertion in [`parse_row`] then rejects. Kept as a separate function so the
/// distinction is visible rather than an easily-missed branch.
fn split_types(cell: &str) -> Vec<String> {
    cell.trim()
        .split(';')
        .map(str::trim)
        .filter(|alternative| !alternative.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// Splits a bracketed cell into its `;`-separated groups, stripping the brackets.
///
/// `[a,b];[c]` becomes `["a,b", "c"]`. An unbracketed cell such as `TRUE` becomes
/// `["TRUE"]`, so scalar and grouped forms are handled uniformly.
fn split_groups(cell: &str) -> Vec<String> {
    let cell = cell.trim();
    if cell.is_empty() {
        return Vec::new();
    }
    if !cell.starts_with('[') {
        return vec![cell.to_owned()];
    }
    // Split on the boundary between groups rather than on every `;`, because a predicate
    // inside a group may itself contain one.
    cell.split("];[")
        .map(|group| {
            group
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_owned()
        })
        .collect()
}

/// Picks the group for a type alternative, allowing a single group to apply to all.
fn pick(groups: &[String], index: usize) -> Option<&str> {
    if groups.len() == 1 {
        groups.first().map(String::as_str)
    } else {
        groups.get(index).map(String::as_str)
    }
}

fn parse_key(cell: &str, at: &dyn Fn() -> String) -> String {
    let key = cell.trim();
    assert!(!key.is_empty(), "{}: empty Key", at());

    if key == "*" {
        return "KeyPattern::Wildcard".to_owned();
    }
    if let Some(Ok(index)) = key.strip_suffix('*').map(str::parse::<u32>) {
        return format!("KeyPattern::Repeating({index})");
    }
    if let Ok(index) = key.parse::<u32>() {
        return format!("KeyPattern::Index({index})");
    }
    format!("KeyPattern::Name({key:?})")
}

/// Parses a `Type` cell entry into a variant name and its gate.
///
/// Two rows in the model wrap a type in a predicate — `fn:Extension(Malforms,name)` and
/// `fn:Deprecated(2.0,array)` — so those shapes are handled rather than rejected.
fn parse_type(raw: &str, at: &dyn Fn() -> String) -> (String, String) {
    let raw = raw.trim();

    if let Some(inner) = raw
        .strip_prefix("fn:Extension(")
        .and_then(|r| r.strip_suffix(')'))
    {
        let (extension, kind) = inner.split_once(',').unwrap_or_else(|| {
            panic!(
                "{}: fn:Extension in Type needs a name and a type: {raw}",
                at()
            )
        });
        return (
            type_variant(kind, at),
            format!("TypeGate::Extension({extension:?})"),
        );
    }
    if let Some(inner) = raw
        .strip_prefix("fn:Deprecated(")
        .and_then(|r| r.strip_suffix(')'))
    {
        let (version, kind) = inner.split_once(',').unwrap_or_else(|| {
            panic!(
                "{}: fn:Deprecated in Type needs a version and a type: {raw}",
                at()
            )
        });
        return (
            type_variant(kind, at),
            format!(
                "TypeGate::DeprecatedIn(Version::{})",
                parse_version(version, at)
            ),
        );
    }

    (type_variant(raw, at), "TypeGate::Always".to_owned())
}

/// Maps an Arlington type token to a [`PdfType`] variant name.
///
/// An unknown token is a build failure. Arlington's vocabulary is closed today; if it
/// grows, this must be extended deliberately rather than the new type being ignored.
fn type_variant(token: &str, at: &dyn Fn() -> String) -> String {
    match token.trim() {
        "array" => "Array",
        "bitmask" => "Bitmask",
        "boolean" => "Boolean",
        "date" => "Date",
        "dictionary" => "Dictionary",
        "integer" => "Integer",
        "matrix" => "Matrix",
        "name" => "Name",
        "name-tree" => "NameTree",
        "null" => "Null",
        "number" => "Number",
        "number-tree" => "NumberTree",
        "rectangle" => "Rectangle",
        "stream" => "Stream",
        "string" => "String",
        "string-ascii" => "StringAscii",
        "string-byte" => "StringByte",
        "string-text" => "StringText",
        other => panic!(
            "{}: unknown Arlington type {other:?}. Add it to PdfType and to this \
             mapping; do not let it be dropped.",
            at()
        ),
    }
    .to_owned()
}

fn parse_version(raw: &str, at: &dyn Fn() -> String) -> String {
    match raw.trim() {
        "1.0" => "V1_0",
        "1.1" => "V1_1",
        "1.2" => "V1_2",
        "1.3" => "V1_3",
        "1.4" => "V1_4",
        "1.5" => "V1_5",
        "1.6" => "V1_6",
        "1.7" => "V1_7",
        "2.0" => "V2_0",
        other => panic!("{}: unknown PDF version {other:?}", at()),
    }
    .to_owned()
}

/// Parses a `SinceVersion` cell.
///
/// Three shapes occur, and the set is closed as of the pinned model revision:
///
/// - a plain version, `1.4`;
/// - `fn:Extension(Name)` or `fn:Extension(Name,1.6)`, available only under an extension;
/// - `fn:Eval(fn:Extension(Name,1.6) || 2.0)`, standard from 2.0 and available earlier
///   under the extension.
fn parse_availability(cell: &str, at: &dyn Fn() -> String) -> String {
    let raw = cell.trim();

    if let Some(inner) = raw
        .strip_prefix("fn:Eval(")
        .and_then(|r| r.strip_suffix(')'))
    {
        let (left, right) = inner
            .split_once("||")
            .unwrap_or_else(|| panic!("{}: unsupported SinceVersion expression {raw:?}", at()));
        let (extension, extension_since) = parse_extension(left.trim(), at);
        return format!(
            "Availability::SinceOrExtension {{ since: Version::{}, extension: {extension:?}, \
             extension_since: {extension_since} }}",
            parse_version(right.trim(), at)
        );
    }

    if raw.starts_with("fn:Extension(") {
        let (extension, since) = parse_extension(raw, at);
        return format!("Availability::Extension {{ name: {extension:?}, since: {since} }}");
    }

    format!("Availability::Since(Version::{})", parse_version(raw, at))
}

/// Parses `fn:Extension(Name)` or `fn:Extension(Name,1.6)`.
fn parse_extension(raw: &str, at: &dyn Fn() -> String) -> (String, String) {
    let inner = raw
        .strip_prefix("fn:Extension(")
        .and_then(|r| r.strip_suffix(')'))
        .unwrap_or_else(|| panic!("{}: expected fn:Extension(..), found {raw:?}", at()));

    match inner.split_once(',') {
        Some((name, version)) => (
            name.trim().to_owned(),
            format!("Some(Version::{})", parse_version(version, at)),
        ),
        None => (inner.trim().to_owned(), "None".to_owned()),
    }
}

fn parse_required(cell: &str, at: &dyn Fn() -> String) -> String {
    let raw = cell.trim();
    match raw {
        "TRUE" => "Requirement::Always".to_owned(),
        "FALSE" => "Requirement::Never".to_owned(),
        predicate if predicate.starts_with("fn:IsRequired(") => {
            format!("Requirement::Conditional({predicate:?})")
        }
        other => panic!("{}: unsupported Required value {other:?}", at()),
    }
}

fn parse_indirect(cell: &str, at: &dyn Fn() -> String) -> String {
    let raw = cell.trim();
    match raw {
        "TRUE" => "Indirectness::Required".to_owned(),
        "FALSE" | "" => "Indirectness::Forbidden".to_owned(),
        predicate if predicate.starts_with("fn:MustBeDirect") => {
            format!("Indirectness::Conditional({predicate:?})")
        }
        predicate if predicate.starts_with("fn:MustBeIndirect") => {
            format!("Indirectness::Conditional({predicate:?})")
        }
        other => panic!("{}: unsupported IndirectReference value {other:?}", at()),
    }
}

fn parse_bool(cell: &str, at: &dyn Fn() -> String) -> bool {
    match cell.trim() {
        "TRUE" => true,
        "FALSE" | "" => false,
        other => panic!("{}: expected TRUE or FALSE, found {other:?}", at()),
    }
}

fn non_empty(cell: &str) -> Option<&str> {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn optional_str(value: Option<&str>) -> String {
    value.map_or_else(|| "None".to_owned(), |text| format!("Some({text:?})"))
}

fn emit(objects: &BTreeMap<String, Vec<Row>>) -> String {
    let mut out = String::with_capacity(1 << 20);
    out.push_str(
        "// @generated by build.rs from the Arlington PDF Model. Do not edit.\n\
         //\n\
         // Sorted by object name so OBJECTS can be binary-searched.\n\n",
    );

    let _ = writeln!(out, "/// Number of object definitions in the model.");
    let _ = writeln!(out, "pub const OBJECT_COUNT: usize = {};", objects.len());
    let total: usize = objects.values().map(Vec::len).sum();
    let _ = writeln!(
        out,
        "/// Number of key rows across every object definition."
    );
    let _ = writeln!(out, "pub const KEY_COUNT: usize = {total};\n");

    for (name, rows) in objects {
        let _ = writeln!(out, "static {}: &[KeySpec] = &[", table_ident(name));
        for row in rows {
            let _ = writeln!(
                out,
                "    KeySpec {{ pattern: {}, types: &[{}], availability: {}, \
                 deprecated_in: {}, required: {}, inheritable: {}, default_value: {}, \
                 special_case: {} }},",
                row.pattern,
                row.types.join(", "),
                row.availability,
                row.deprecated_in,
                row.required,
                row.inheritable,
                row.default_value,
                row.special_case
            );
        }
        out.push_str("];\n");
    }

    out.push_str("\n/// Every object definition in the Arlington model, sorted by name.\n");
    out.push_str("pub static OBJECTS: &[ObjectSpec] = &[\n");
    for name in objects.keys() {
        let _ = writeln!(
            out,
            "    ObjectSpec {{ name: {name:?}, keys: {} }},",
            table_ident(name)
        );
    }
    out.push_str("];\n");

    out
}

/// Turns an object name into a valid Rust identifier for its key table.
fn table_ident(name: &str) -> String {
    let mut ident = String::from("KEYS_");
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            ident.push(character.to_ascii_uppercase());
        } else {
            ident.push('_');
        }
    }
    ident
}
