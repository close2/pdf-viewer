//! Spike C: verify the generated tables against objects whose definitions are known.
//!
//! # What "verify" means here
//!
//! Asserting that the tables match the TSVs would only test that the generator can read
//! its own input. These assertions instead state what **ISO 32000-2 says**, checked
//! against the specification text in `doc/`, so a failure means either the generator is
//! wrong or the model disagrees with the standard. Both are worth knowing.
//!
//! Section references are to ISO 32000-2:2020.

use pdf_spec::{Availability, Indirectness, KeyPattern, PdfType, Requirement, Version, object};

/// The catalogue, ISO 32000-2 table 29.
///
/// Chosen as the primary check because it is the document's root: every viewer touches it,
/// and it exercises required keys, version-gated keys, defaults, value enumerations and
/// links to other object types in one table.
#[test]
fn the_catalogue_matches_the_specification() {
    let catalog = object("Catalog").expect("the model defines Catalog");

    // /Type is required and must be the name Catalog.
    let type_key = catalog.key("Type").expect("Catalog has /Type");
    assert_eq!(type_key.required, Requirement::Always);
    assert_eq!(type_key.types.len(), 1);
    assert_eq!(type_key.types[0].kind, PdfType::Name);
    assert_eq!(type_key.types[0].possible_values, Some("Catalog"));

    // /Pages is required, is a dictionary, and links to the page tree root.
    let pages = catalog.key("Pages").expect("Catalog has /Pages");
    assert_eq!(pages.required, Requirement::Always);
    assert_eq!(pages.types[0].kind, PdfType::Dictionary);
    assert_eq!(pages.types[0].links, &["PageTreeNodeRoot"]);
    assert_eq!(
        pages.types[0].indirect,
        Indirectness::Required,
        "the page tree root must be an indirect reference"
    );

    // /Version was added in PDF 1.4 and is one of the known version names.
    let version = catalog.key("Version").expect("Catalog has /Version");
    assert_eq!(version.availability, Availability::Since(Version::V1_4));
    assert_eq!(version.required, Requirement::Never);
    assert_eq!(
        version.types[0].possible_values,
        Some("1.0,1.1,1.2,1.3,1.4,1.5,1.6,1.7,2.0")
    );

    // /PageLayout defaults to SinglePage.
    let layout = catalog.key("PageLayout").expect("Catalog has /PageLayout");
    assert_eq!(layout.default_value, Some("SinglePage"));
    assert_eq!(
        layout.types[0].possible_values,
        Some("SinglePage,OneColumn,TwoColumnLeft,TwoColumnRight,TwoPageLeft,TwoPageRight")
    );

    // Exactly two keys are unconditionally required.
    let required: Vec<&str> = catalog
        .always_required()
        .filter_map(|spec| match spec.pattern {
            KeyPattern::Name(name) => Some(name),
            _ => None,
        })
        .collect();
    assert_eq!(
        required,
        vec!["Type", "Pages"],
        "a catalogue needs only /Type and /Pages"
    );

    assert!(
        !catalog.is_map_like(),
        "a catalogue does not accept arbitrary keys"
    );
}

/// Inheritance, which the page tree relies on and which is easy to get wrong.
///
/// ISO 32000-2 table 31 marks `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate` as
/// inheritable: a page without them takes its ancestor's value. A viewer that ignored this
/// would render pages at the wrong size, so the flag is worth pinning.
#[test]
fn page_attributes_that_the_specification_says_are_inheritable_are_marked_so() {
    let page = object("PageObject").expect("the model defines PageObject");

    for name in ["Resources", "MediaBox", "CropBox", "Rotate"] {
        let key = page
            .key(name)
            .unwrap_or_else(|| panic!("PageObject has /{name}"));
        assert!(key.inheritable, "/{name} is inheritable per table 31");
    }

    // /Contents is emphatically not inheritable — inheriting it would draw an ancestor's
    // content onto a page that has none.
    let contents = page.key("Contents").expect("PageObject has /Contents");
    assert!(!contents.inheritable);
}

/// A key with several permitted types, and its per-type constraints.
///
/// `/Contents` is a single stream or an array of streams. This is the case the positional
/// alignment of `Type` and `Link` exists to serve, so it is checked directly.
#[test]
fn multiple_type_alternatives_keep_their_own_links() {
    let page = object("PageObject").expect("the model defines PageObject");
    let contents = page.key("Contents").expect("PageObject has /Contents");

    assert_eq!(
        contents.types.len(),
        2,
        "/Contents is a stream or an array of streams"
    );

    let kinds: Vec<PdfType> = contents
        .types
        .iter()
        .map(|alternative| alternative.kind)
        .collect();
    assert!(kinds.contains(&PdfType::Stream));
    assert!(kinds.contains(&PdfType::Array));

    // Each alternative links to a different object type — the whole point of aligning them.
    for alternative in contents.types {
        assert!(
            !alternative.links.is_empty(),
            "{} alternative should link somewhere",
            alternative.kind
        );
    }
    let stream = contents
        .types
        .iter()
        .find(|alternative| alternative.kind == PdfType::Stream)
        .expect("a stream alternative");
    let array = contents
        .types
        .iter()
        .find(|alternative| alternative.kind == PdfType::Array)
        .expect("an array alternative");
    assert_ne!(
        stream.links, array.links,
        "the two alternatives must not have been given the same links"
    );
}

/// Map-like objects, where any key is permitted.
#[test]
fn wildcard_lookup_serves_map_like_objects() {
    let map = object("AppearanceSubDict").expect("the model defines AppearanceSubDict");
    assert!(map.is_map_like());

    // Any name resolves, via the wildcard row.
    let arbitrary = map
        .key("SomeNameNobodyPredicted")
        .expect("a wildcard accepts any key");
    assert_eq!(arbitrary.pattern, KeyPattern::Wildcard);
    assert_eq!(arbitrary.types[0].kind, PdfType::Stream);
}

/// Array element lookup, including repeating groups.
#[test]
fn array_elements_resolve_by_index() {
    let array = object("ArrayOfDecodeParams").expect("the model defines ArrayOfDecodeParams");
    // A wildcard array: every element has the same definition.
    let element = array.element(0).expect("element 0 resolves");
    assert_eq!(element.pattern, KeyPattern::Wildcard);
    assert!(
        array.element(17).is_some(),
        "any index resolves in a wildcard array"
    );
}

/// Conditionally-required keys must be reported as conditional, never as a plain
/// yes-or-no.
///
/// This is the property that keeps the unevaluated predicates honest. Collapsing
/// `Conditional` to `Never` would accept invalid files; collapsing it to `Always` would
/// reject valid ones.
#[test]
fn conditional_requirements_are_preserved_verbatim() {
    let units = object("3DUnits").expect("the model defines 3DUnits");
    let key = units.key("TU").expect("3DUnits has /TU");

    match key.required {
        Requirement::Conditional(predicate) => {
            assert!(
                predicate.starts_with("fn:IsRequired("),
                "the predicate should be carried through unchanged, got {predicate:?}"
            );
            assert!(
                predicate.contains("TSm"),
                "and should still name the keys it depends on"
            );
        }
        other => panic!("/TU is conditionally required, not {other:?}"),
    }

    assert_ne!(
        key.required,
        Requirement::Always,
        "a conditional requirement must not be reported as unconditional"
    );
}

/// Extension-gated availability, the shape 645 rows use.
#[test]
fn extension_gated_keys_record_their_extension() {
    let extension_gated = pdf_spec::OBJECTS
        .iter()
        .flat_map(|spec| spec.keys.iter())
        .find(|key| matches!(key.availability, Availability::SinceOrExtension { .. }))
        .expect("the model contains keys that were extensions before becoming standard");

    match extension_gated.availability {
        Availability::SinceOrExtension {
            since, extension, ..
        } => {
            assert_eq!(since, Version::V2_0, "these were all folded into PDF 2.0");
            assert!(!extension.is_empty(), "the extension must be named");
        }
        other => panic!("unexpected availability {other:?}"),
    }

    // An extension-only key has no standard version at all, and must not be reported as
    // available in some version by default.
    let extension_only = pdf_spec::OBJECTS
        .iter()
        .flat_map(|spec| spec.keys.iter())
        .find(|key| matches!(key.availability, Availability::Extension { .. }));
    if let Some(key) = extension_only {
        assert_eq!(key.availability.standard_since(), None);
    }
}

/// Deprecation, which a viewer must honour to render old documents correctly.
#[test]
fn deprecated_keys_record_the_version_that_removed_them() {
    let deprecated = pdf_spec::OBJECTS
        .iter()
        .flat_map(|spec| spec.keys.iter())
        .filter(|key| key.deprecated_in.is_some())
        .count();
    assert_eq!(
        deprecated, 373,
        "the pinned model deprecates this many keys"
    );
}

/// The whole model is present, not a subset.
///
/// A generator that silently skipped files it could not parse would produce a smaller but
/// apparently working table, so the counts are pinned to the model revision.
#[test]
fn the_entire_pinned_model_is_generated() {
    assert_eq!(
        pdf_spec::OBJECT_COUNT,
        611,
        "TSV files in tsv/2.0 at the pinned revision"
    );
    assert_eq!(pdf_spec::KEY_COUNT, 3973, "key rows across all of them");
    assert_eq!(pdf_spec::OBJECTS.len(), pdf_spec::OBJECT_COUNT);
}

/// Lookup must be exact, and must not invent entries.
#[test]
fn unknown_object_names_are_not_found() {
    assert!(object("NoSuchObjectType").is_none());
    assert!(
        object("catalog").is_none(),
        "lookup is case-sensitive, as Arlington names are"
    );
    assert!(object("").is_none());
}

/// The table must be sorted, since lookup binary-searches it.
#[test]
fn the_object_table_is_sorted() {
    let names: Vec<&str> = pdf_spec::OBJECTS.iter().map(|spec| spec.name).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "binary search requires a sorted table");
}
