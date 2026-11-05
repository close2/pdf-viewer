//! A spot colourant defined nowhere but inside its `DeviceN` space is given the `Separation` that
//! space states for it alone (ADR 2490's R12, `doc/adr/2690`).
//!
//! ISO 19005-2 section 6.2.4.4 and ISO 19005-4 section 6.2.4.4 want a `Colorants` entry for every
//! spot colourant a `DeviceN` space uses. ISO 32000-2 §8.6.6.5 says what such an entry is: "the
//! alternate colour space and tint transformation function of a Separation colour space describe
//! the appearance of that colourant alone". The entry written is the space's own alternate and its
//! own §7.10.5 program with the other inputs fixed at zero, so these tests evaluate both functions
//! over a grid of tints and require the same numbers.

use super::*;
use pdf_model::function::Function;
use pdf_syntax::object::{Dictionary, Object};

const ROW: &str = "graphics/spot-colourants-appear-in-the-colorants-dictionary";

const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// A `CalRGB` alternate, which needs no output intent at either part.
const CAL_RGB: &str = "[/CalRGB << /WhitePoint [0.9505 1.0 1.089] >>]";

/// A three-input, three-output §7.10.5 program in which every output reads more than one input:
/// `r = x0 + 0.5 x1`, `g = x1 x2 + 0.2`, `b = 1 - 0.3 (x0 + x2)`.
const PROGRAM: &[u8] = b"{ 2 index 2 index 0.5 mul add 2 index 2 index mul 0.2 add \
                          4 index 3 index add 0.3 mul 1 exch sub 6 3 roll pop pop pop }";

/// Its dictionary. The second input's domain excludes zero, so the value the producer's function
/// evaluates an absent `B` as is the bound `0.25` (§7.10.1's clipping), which the restriction has
/// to push in its place.
fn type_four(inputs: &str, outputs: &str, program: &[u8]) -> Vec<u8> {
    stream(
        &format!(
            "/FunctionType 4 /Domain [{inputs}] /Range [{outputs}] /Length {}",
            program.len()
        ),
        program,
    )
}

fn base(target: Target) -> Conforming {
    match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    }
}

/// A page whose resources name each of `spaces` as `/CS<i>`, objects 6 onwards, followed by
/// `streams` as binary objects, and which fills a square in each — ISO 19005's colour space rules
/// bind the spaces a page uses, so a space named and never used is no test of them.
fn naming(target: Target, spaces: &[String], streams: Vec<Vec<u8>>) -> Vec<u8> {
    let mut names = String::new();
    let mut paints = String::new();
    for (at, number) in (6_usize..).take(spaces.len()).enumerate() {
        // §8.6.8: `cs` sets the space and its initial colour, so no `sc` is needed to fill in it.
        let _ = write!(names, "/CS{at} {number} 0 R ");
        let _ = write!(paints, "/CS{at} cs 0 0 10 10 re f ");
    }
    Conforming {
        resources: format!("/ColorSpace << {names}>>"),
        objects: spaces.to_vec(),
        binary_objects: streams,
        contents: Some((String::new(), paints.into_bytes())),
        ..base(target)
    }
    .build()
}

/// The three-colourant space over [`PROGRAM`], its transform at `transform`.
fn three_colourants(transform: usize) -> String {
    format!("[/DeviceN [/A /B /C] {CAL_RGB} {transform} 0 R]")
}

/// The page's colour space `/CS<at>` in `bytes`, resolved, with the document it was read from.
fn space(bytes: &[u8], at: usize) -> (Document, Vec<Object>) {
    let document =
        Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("the file opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let resources = document.get_key(&page.dict, "Resources");
    let spaces = document.get_key(resources.as_dict().expect("resources"), "ColorSpace");
    let space = document.get_key(spaces.as_dict().expect("a table"), &format!("CS{at}"));
    let items = space.as_array().expect("an array").to_vec();
    (document, items)
}

/// The `Colorants` dictionary of a resolved `DeviceN` array.
fn colorants(document: &Document, items: &[Object]) -> Dictionary {
    let attributes = document.resolve(items.get(4).expect("an attributes dictionary"));
    let colorants = document.get_key(attributes.as_dict().expect("a dictionary"), "Colorants");
    colorants.as_dict().expect("a Colorants dictionary").clone()
}

fn reason(report: &Report) -> &'static str {
    match decision(report, ROW) {
        Decision::Refused(Because::NotBuiltYet(reason)) => reason,
        other => panic!("refused as not built yet, not {other:?}"),
    }
}

#[test]
fn each_colourants_separation_draws_what_its_space_draws_for_it_alone() {
    for target in TARGETS {
        let source = naming(
            target,
            &[three_colourants(7)],
            vec![type_four("0 1 0.25 1 0 1", "0 1 0 1 0 1", PROGRAM)],
        );
        let (report, output) = convert(&source, target, Authorisations::default());
        assert_eq!(
            decision(&report, ROW),
            Decision::Mechanical(Rewrite::SpotColorantEntry),
            "{target}"
        );
        // Nothing is authorised, so every profile — `no-data-loss` included — takes it.
        let output = output.expect("nothing is lost");
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());

        let (document, items) = space(&output, 0);
        let combined = Function::parse(&document, &items[3]).expect("the producer's transform");
        let entries = colorants(&document, &items);
        for (index, name) in ["A", "B", "C"].into_iter().enumerate() {
            let entry = document.get_key(&entries, name);
            let entry = entry.as_array().expect("a Separation array");
            assert_eq!(
                document
                    .resolve(&entry[0])
                    .as_name()
                    .map(|n| n.as_bytes().to_vec()),
                Some(b"Separation".to_vec())
            );
            assert_eq!(
                document
                    .resolve(&entry[1])
                    .as_name()
                    .map(|n| n.as_bytes().to_vec()),
                Some(name.as_bytes().to_vec())
            );
            assert_eq!(
                document.resolve(&entry[2]),
                document.resolve(&items[2]),
                "the space's own alternate"
            );
            let alone =
                Function::parse(&document, &entry[3]).expect("a function this reader reads");
            for step in 0..=40_u8 {
                let tint = f32::from(step) / 40.0;
                let mut inputs = [0.0_f32; 3];
                inputs[index] = tint;
                assert_eq!(
                    alone.eval(&[tint]),
                    combined.eval(&inputs),
                    "{target}: {name} at a tint of {tint}"
                );
            }
        }
    }
}

#[test]
fn a_one_component_space_names_its_own_transform_whatever_its_type() {
    // One input is that colourant alone already, so the transform is the entry as it stands, and
    // a §7.10.3 exponential function needs no restriction to be exact.
    let exponential =
        "<< /FunctionType 2 /Domain [0 1] /C0 [1 1 1] /C1 [0 0.5 1] /N 1 >>".to_owned();
    let source = naming(
        Target::Four(Flavour::Plain),
        &[format!("[/DeviceN [/Spot] {CAL_RGB} 7 0 R]"), exponential],
        Vec::new(),
    );
    let (report, output) = to_part_four(&source);
    assert_eq!(
        decision(&report, ROW),
        Decision::Mechanical(Rewrite::SpotColorantEntry)
    );
    let output = output.expect("nothing is lost");
    let (document, items) = space(&output, 0);
    let entries = colorants(&document, &items);
    let entry = document.get_key(&entries, "Spot");
    assert_eq!(
        entry.as_array().expect("an array")[3],
        items[3],
        "the same transform object"
    );
}

#[test]
fn a_sampled_transform_over_two_colourants_stays_refused_by_its_type() {
    let sampled = stream(
        "/FunctionType 0 /Domain [0 1 0 1] /Range [0 1 0 1 0 1] /Size [2 2] /BitsPerSample 8 \
         /Length 12",
        &[0; 12],
    );
    let source = naming(
        Target::Four(Flavour::Plain),
        &[format!("[/DeviceN [/A /B] {CAL_RGB} 7 0 R]")],
        vec![sampled],
    );
    let (report, output) = to_part_four(&source);
    assert!(
        reason(&report).contains("sampled function"),
        "{}",
        reason(&report)
    );
    assert!(reason(&report).contains("FunctionType 0"));
    assert!(output.is_none(), "and a refusal writes no file");
}

#[test]
fn an_nchannel_spaces_colourant_stays_refused() {
    let source = naming(
        Target::Four(Flavour::Plain),
        &[format!(
            "[/DeviceN [/A /B /C] {CAL_RGB} 7 0 R << /Subtype /NChannel >>]"
        )],
        vec![type_four("0 1 0 1 0 1", "0 1 0 1 0 1", PROGRAM)],
    );
    let (report, output) = to_part_four(&source);
    assert!(reason(&report).contains("NChannel"), "{}", reason(&report));
    assert!(output.is_none());
}

#[test]
fn two_spaces_over_one_transform_share_one_separation_and_two_transforms_are_refused() {
    let same = naming(
        Target::Four(Flavour::Plain),
        &[three_colourants(8), three_colourants(8)],
        vec![type_four("0 1 0 1 0 1", "0 1 0 1 0 1", PROGRAM)],
    );
    let (_, output) = to_part_four(&same);
    let output = output.expect("one definition per colourant, so it is written");
    let (document, first) = space(&output, 0);
    let (_, second) = space(&output, 1);
    for name in ["A", "B", "C"] {
        assert_eq!(
            document.get_key(&colorants(&document, &first), name),
            document.get_key(&colorants(&document, &second), name),
            "ISO 19005 section 6.2.4.4: every Separation of {name} is the same object"
        );
    }

    let differing = naming(
        Target::Four(Flavour::Plain),
        &[three_colourants(8), three_colourants(9)],
        vec![
            type_four("0 1 0 1 0 1", "0 1 0 1 0 1", PROGRAM),
            type_four("0 1 0 1 0 1", "0 1 0 1 0 1", b"{ pop pop pop 0 0 0 }"),
        ],
    );
    let (report, output) = to_part_four(&differing);
    assert!(
        reason(&report).contains("two DeviceN colour spaces"),
        "{}",
        reason(&report)
    );
    assert!(output.is_none());
}

#[test]
fn an_attributes_dictionary_written_as_its_own_object_is_copied_into_the_space() {
    // pdf.js's issue9940 shape: `[/DeviceN [/IBM /None /None /None] … 9 0 R]` over `<< >>`.
    let source = naming(
        Target::Four(Flavour::Plain),
        &[
            format!("[/DeviceN [/IBM /None /None /None] {CAL_RGB} 8 0 R 7 0 R]"),
            "<< >>".to_owned(),
        ],
        vec![type_four(
            "0 1 0 1 0 1 0 1",
            "0 1 0 1 0 1",
            b"{4 3 roll pop }",
        )],
    );
    let (_, output) = to_part_four(&source);
    let output = output.expect("the copy is written");
    let held = holds(&output, Target::Four(Flavour::Plain));
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    let (document, items) = space(&output, 0);
    let entries = colorants(&document, &items);
    assert!(document.get_key(&entries, "IBM").as_array().is_some());
    assert!(
        matches!(items[4], Object::Dictionary(_)),
        "the space holds its own copy, so the shared object is not written into"
    );
}
