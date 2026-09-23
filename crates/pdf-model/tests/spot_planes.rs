//! ISO 32000-2 §10.8.3 step a)'s separations, with a plane per spot colourant (ADR 1311).
//!
//! Under the reader's simulation a page naming a spot colourant is interpreted once per plane of
//! the simulated device, and each colour is resolved by §11.7.3's paragraph: "In effect, every
//! object paints every existing colour component, both process and spot. Where no value has been
//! explicitly specified for a given component in a given object, an additive value of 1.0 (or a
//! subtractive tint value of 0.0) shall be assumed." Every expected value below is that sentence,
//! §8.6.6.4's or §11.7.4.3's, applied by hand: a plane stores §11.3.4's additive complement, so a
//! tint `t` is `1 − t` in its channel and a component nothing names is 1.0.
#![expect(
    clippy::expect_used,
    reason = "test code: a malformed fixture should fail loudly"
)]
#![expect(
    clippy::doc_markdown,
    reason = "the tests quote §8.6.6.4, §11.7.3 and §11.7.4.3 verbatim, and a quotation may not \
              gain backticks"
)]

use std::fmt::Write as _;

use pdf_model::Interpretation;
use pdf_model::colour::Plane;
use pdf_model::colourants::{MAX_SPOT_COLOURANTS, Separation};
use pdf_render::{BlendMode, Color, Command, Paint};
use pdf_syntax::Document;

/// §8.6.6.4's EXAMPLE 2, object 12: the tint transform "maps tint values linearly into shades of a
/// CMYK colour value approximating the LogoGreen colour".
const LOGO_GREEN: &str = "[/Separation /LogoGreen /DeviceCMYK 5 0 R]";

/// Object 5: EXAMPLE 2's PostScript calculator function, byte for byte.
const LOGO_GREEN_TINT: &str = "<< /FunctionType 4 /Domain [0.0 1.0] \
     /Range [0.0 1.0 0.0 1.0 0.0 1.0 0.0 1.0] /Length 62 >>\nstream\n\
     {dup 0.84 mul exch 0.00 exch dup 0.44 mul exch 0.21 mul}\nendstream";

/// A tint transform into `DeviceCMYK` for the spaces whose alternate does not matter to a test:
/// §7.10.3's exponential function, the same for any number of inputs it is handed.
const TINT: &str = "<< /FunctionType 2 /Domain [0 1] /C0 [0 0 0 0] /C1 [0 1 1 0] /N 1 >>";

/// A one-page file: `page` is extra page-dictionary entries, `resources` the resource dictionary's
/// body, and `extra` further objects numbered from 5.
fn file(page: &str, resources: &str, content: &str, extra: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 20 20] /Resources << {resources} >> \
             /Contents 4 0 R {page} >>"
        ),
        format!(
            "<< /Length {} >>\nstream\n{content}\nendstream",
            content.len().saturating_add(1)
        ),
    ];
    objects.extend(extra.iter().map(|object| (*object).to_owned()));
    let mut out = String::from("%PDF-2.0\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = offsets.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Page one of `bytes`, interpreted with §10.8.3's simulation on or off.
fn interpreted(bytes: Vec<u8>, simulate: bool) -> Interpretation {
    let document = Document::open(bytes).expect("the fixture is a valid PDF");
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let mut state = pdf_model::view::ViewState::of(&document);
    state.set_separation_simulation(simulate);
    pdf_model::content::interpret_with(&document, &page, &state)
}

/// Page one's separation under the simulation, which the fixture must have.
fn separated(bytes: Vec<u8>) -> Separation {
    interpreted(bytes, true)
        .separation
        .expect("a page naming a spot colourant is separated under the simulation")
}

/// The solid colours of the fills `commands` holds, groups entered, in drawing order.
fn fills(commands: &[Command]) -> Vec<Color> {
    let mut found = Vec::new();
    for command in commands {
        match command {
            Command::Fill {
                paint: Paint::Solid(colour),
                ..
            } => found.push(*colour),
            Command::Group { commands, .. } => found.extend(fills(commands)),
            _ => {}
        }
    }
    found
}

/// The fills of one plane of `separation`.
fn plane(separation: &Separation, plane: Plane) -> Vec<Color> {
    fills(
        separation
            .plane(plane)
            .expect("the device has this plane")
            .commands(),
    )
}

/// The blend modes of one plane's top-level fills.
fn blends(separation: &Separation, plane: Plane) -> Vec<BlendMode> {
    separation
        .plane(plane)
        .expect("the device has this plane")
        .commands()
        .iter()
        .filter_map(|command| match command {
            Command::Fill { blend, .. } => Some(*blend),
            _ => None,
        })
        .collect()
}

/// Colours compared to a sixteenth of an eight-bit level: what is stored is an `f32` complement.
fn close(found: Color, expected: [f32; 3]) -> bool {
    [found.r, found.g, found.b]
        .iter()
        .zip(expected)
        .all(|(found, expected)| (found - expected).abs() < 1.0 / 4096.0)
}

/// Asserts one fill's three channels.
#[track_caller]
fn assert_colour(found: Color, expected: [f32; 3], what: &str) {
    assert!(
        close(found, expected),
        "{what}: {found:?}, expected {expected:?}"
    );
}

/// No ink on a plane, in the additive form every plane stores.
const WHITE: [f32; 3] = [1.0, 1.0, 1.0];

/// The LogoGreen page, with `page` its extra entries.
fn logo_green(page: &str, content: &str) -> Vec<u8> {
    file(
        page,
        &format!("/ColorSpace << /LG {LOGO_GREEN} >>"),
        content,
        &[LOGO_GREEN_TINT],
    )
}

/// §8.6.6.4: where "the device has an available colourant corresponding to the name of the
/// requested space … the PDF processor shall ignore the alternateSpace and tintTransform
/// parameters; subsequent painting operations within the space shall apply the designated
/// colourant directly, according to the tint values supplied." And §11.7.3: "when painting an
/// object with a colour specified in a Separation colour space, the named spot colour shall be
/// painted as specified and all other components (both process colours and other spot colours)
/// shall be painted with an additive value of 1.0."
///
/// So EXAMPLE 2's LogoGreen at 0.5 is 0.5 on its own plane and no ink on either process plane —
/// on a page drawn on the device, whose separation is made on the simulated device's press, and
/// on a page whose group composites in `DeviceCMYK`.
#[test]
fn logo_green_paints_its_own_plane_and_no_process_ink() {
    for page in ["", "/Group << /S /Transparency /CS /DeviceCMYK >>"] {
        let separation = separated(logo_green(page, "/LG cs 0.5 scn 0 0 20 20 re f"));
        let names: Vec<&[u8]> = separation
            .colourants()
            .iter()
            .map(pdf_syntax::Name::as_bytes)
            .collect();
        assert_eq!(names, [b"LogoGreen".as_slice()], "{page}");
        assert_eq!(
            separation.plane_count(),
            3,
            "two process planes and one spot plane"
        );
        assert_colour(plane(&separation, Plane::Spot(0))[0], [0.5, 1.0, 1.0], page);
        assert_colour(plane(&separation, Plane::Chromatic)[0], WHITE, page);
        assert_colour(plane(&separation, Plane::Black)[0], WHITE, page);
        assert!(separation.without_a_plane().is_empty());
    }
}

/// The page a backend draws is the page it drew before: the separation sits beside it, and with
/// the simulation off there is none at all.
#[test]
fn the_drawn_page_is_unchanged_and_off_makes_no_separation() {
    let bytes = logo_green("", "/LG cs 0.5 scn 0 0 20 20 re f");
    let off = interpreted(bytes.clone(), false);
    let on = interpreted(bytes, true);
    assert!(
        off.separation.is_none(),
        "no separation without the reader's request"
    );
    assert!(on.separation.is_some());
    assert_eq!(
        off.display_list, on.display_list,
        "the drawn list does not move"
    );
}

/// A page naming no spot colourant has nothing to separate, simulation or not.
#[test]
fn a_page_of_process_colours_is_not_separated() {
    let page = interpreted(file("", "", "0 0 1 0 k 0 0 20 20 re f", &[]), true);
    assert!(page.separation.is_none());
}

/// §8.6.6.5's `DeviceN` "may contain an arbitrary number of colour components", and five spot
/// colourants are two spot planes: the first three in the first, the last two in the second and
/// its third channel empty.
#[test]
fn a_devicen_of_five_colourants_paints_two_spot_planes() {
    let separation = separated(file(
        "",
        &format!(
            "/ColorSpace << /Five [/DeviceN [/Orange /Green /Violet /Gold /Silver] /DeviceCMYK \
             {TINT}] >>"
        ),
        "/Five cs 0.1 0.2 0.3 0.4 0.5 scn 0 0 20 20 re f",
        &[],
    ));
    assert_eq!(separation.plane_count(), 4);
    assert_colour(
        plane(&separation, Plane::Spot(0))[0],
        [0.9, 0.8, 0.7],
        "first",
    );
    assert_colour(
        plane(&separation, Plane::Spot(1))[0],
        [0.6, 0.5, 1.0],
        "second",
    );
    assert_colour(
        plane(&separation, Plane::Chromatic)[0],
        WHITE,
        "no process ink",
    );
    assert_colour(plane(&separation, Plane::Black)[0], WHITE, "no process ink");
}

/// §8.6.6.4: "The special colourant name All shall refer collectively to all colourants available
/// on an output device, including those for the standard process colourants. When a Separation
/// space with this colourant name is the current colour space, painting operators shall apply tint
/// values to all available colourants at once." So 0.25 of `All` is 0.25 on every process
/// component and every spot colourant with a plane, and nothing in a channel no colourant fills.
#[test]
fn all_paints_every_plane() {
    let separation = separated(file(
        "",
        &format!(
            "/ColorSpace << /A [/Separation /Orange /DeviceCMYK {TINT}] \
             /B [/Separation /Green /DeviceCMYK {TINT}] /R [/Separation /All /DeviceCMYK {TINT}] >>"
        ),
        "/R cs 0.25 scn 0 0 20 20 re f",
        &[],
    ));
    assert_colour(
        plane(&separation, Plane::Chromatic)[0],
        [0.75; 3],
        "process",
    );
    assert_colour(plane(&separation, Plane::Black)[0], [0.75; 3], "black");
    assert_colour(
        plane(&separation, Plane::Spot(0))[0],
        [0.75, 0.75, 1.0],
        "two spot colourants and an empty channel",
    );
}

/// §8.6.6.4 reserves "Cyan , Magenta , Yellow and Black … to name the process colourants of a
/// CMYK device", which §10.8.3's simulated device is, so a `Separation` named `Cyan` has an
/// available colourant and ignores its tint transform — here one that would paint magenta.
#[test]
fn a_process_named_separation_paints_the_process_plane_directly() {
    let separation = separated(file(
        "",
        &format!("/ColorSpace << /C [/Separation /Cyan /DeviceCMYK {TINT}] /LG {LOGO_GREEN} >>"),
        "/C cs 0.6 scn 0 0 20 20 re f",
        &[LOGO_GREEN_TINT],
    ));
    assert_colour(
        plane(&separation, Plane::Chromatic)[0],
        [0.4, 1.0, 1.0],
        "cyan's tint alone, not the transform's magenta and yellow",
    );
    assert_colour(plane(&separation, Plane::Black)[0], WHITE, "no black");
    assert_colour(plane(&separation, Plane::Spot(0))[0], WHITE, "no LogoGreen");
}

/// §11.7.3 paints a process colour the same way from the other side: "when painting an object with
/// a colour specified in a DeviceCMYK or ICCBased colour space, the process colour components
/// shall be painted as specified and the spot colour components shall be painted with an additive
/// value of 1.0".
#[test]
fn a_process_colour_paints_no_spot_ink() {
    let separation = separated(logo_green("", "0.2 0.3 0.4 0.5 k 0 0 20 20 re f"));
    assert_colour(
        plane(&separation, Plane::Chromatic)[0],
        [0.8, 0.7, 0.6],
        "CMY",
    );
    assert_colour(plane(&separation, Plane::Black)[0], [0.5; 3], "K");
    assert_colour(plane(&separation, Plane::Spot(0))[0], WHITE, "no LogoGreen");
}

/// A spot colourant inside a knockout group: §11.7.3's first bullet has the group "maintain a
/// separate colour value for each spot colour component, independently of the group's colour
/// space", so the spot plane holds the group with its two LogoGreen elements at their tints, and
/// the process planes hold the same group painting no ink.
#[test]
fn a_spot_mark_in_a_knockout_group_keeps_its_plane() {
    let separation = separated(file(
        "",
        &format!("/ColorSpace << /LG {LOGO_GREEN} >> /XObject << /K 6 0 R >>"),
        "/K Do",
        &[
            LOGO_GREEN_TINT,
            &format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] \
                 /Group << /S /Transparency /K true >> \
                 /Resources << /ColorSpace << /LG {LOGO_GREEN} >> >> /Length 50 >>\nstream\n\
                 /LG cs 1 scn 0 0 10 10 re f 0.5 scn 5 5 10 10 re f\nendstream"
            ),
        ],
    ));
    let spot = separation.plane(Plane::Spot(0)).expect("the spot plane");
    let Some(Command::Group {
        knockout, commands, ..
    }) = spot.commands().first()
    else {
        panic!("the spot plane holds the group: {:?}", spot.commands());
    };
    assert!(*knockout, "§11.4.6's knockout attribute is the plane's too");
    let inside = fills(commands);
    assert_colour(inside[0], [0.0, 1.0, 1.0], "full LogoGreen");
    assert_colour(inside[1], [0.5, 1.0, 1.0], "half LogoGreen");
    for colour in plane(&separation, Plane::Chromatic) {
        assert_colour(colour, WHITE, "no process ink in the group");
    }
}

/// §11.7.4.3's first bullet under `/OP true /OPM 1`, on a `DeviceCMYK` page group: "process colour
/// components with nonzero values shall replace the corresponding component values of the
/// backdrop; components with zero values leave the existing backdrop value unchanged … For spot
/// colour components, the value shall always be 𝐶𝑏 ." And its second, for the `Separation` mark
/// under the same state: "𝐶𝑠 for all colour components specified in the current colour space,
/// otherwise 𝐶𝑏" — LogoGreen replaced, everything else kept.
#[test]
fn overprint_mode_one_leaves_the_spot_backdrop() {
    let separation = separated(file(
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
        &format!(
            "/ColorSpace << /LG {LOGO_GREEN} >> /ExtGState << /O << /OP true /op true /OPM 1 >> >>"
        ),
        "/O gs /LG cs 1 scn 0 0 20 20 re f 0 0 1 0 k 0 0 20 20 re f",
        &[LOGO_GREEN_TINT],
    ));
    let kept = |plane| -> Vec<[bool; 3]> {
        blends(&separation, plane)
            .into_iter()
            .map(|blend| match blend {
                BlendMode::Overprint(overprint) => overprint.kept(),
                other => panic!("the special mode, not {other:?}"),
            })
            .collect()
    };
    assert_eq!(
        kept(Plane::Spot(0)),
        [[false, true, true], [true, true, true]],
        "LogoGreen replaced by its own mark and kept under the CMYK one"
    );
    assert_eq!(
        kept(Plane::Chromatic),
        [[true, true, true], [true, true, false]],
        "the Separation specifies no process component; the CMYK mark's zero cyan and magenta \
         keep the backdrop"
    );
    assert_eq!(
        kept(Plane::Black),
        [[true; 3], [true; 3]],
        "black is zero in both"
    );
}

/// `count` colourant names, `/Ink00` onwards, for a `DeviceN` names array.
fn inks(count: usize) -> String {
    (0..count).fold(String::new(), |mut names, index| {
        let _ = write!(names, "/Ink{index:02} ");
        names
    })
}

/// A page naming more spot colourants than the planes carry: the first
/// [`MAX_SPOT_COLOURANTS`] have planes, and a mark in the next one reverts — §11.7.3's second
/// bullet, "[t]he spot colour shall be converted to its alternate colour space" — and is named.
/// On the process planes it is EXAMPLE 2's tint transform at 0.5, worked by hand: `0.5 dup 0.84
/// mul` leaves 0.42 under the tint, `exch 0.00 exch` puts 0.0 beneath it, `dup 0.44 mul exch 0.21
/// mul` gives 0.22 and 0.105 — CMYK `[0.42 0.0 0.22 0.105]`.
#[test]
fn a_colourant_past_the_bound_reverts_and_is_named() {
    let many = inks(MAX_SPOT_COLOURANTS);
    let separation = separated(file(
        "",
        &format!("/ColorSpace << /A [/DeviceN [{many}] /DeviceCMYK {TINT}] /LG {LOGO_GREEN} >>"),
        "/LG cs 0.5 scn 0 0 20 20 re f",
        &[LOGO_GREEN_TINT],
    ));
    assert_eq!(separation.colourants().len(), MAX_SPOT_COLOURANTS);
    let named: Vec<&[u8]> = separation
        .without_a_plane()
        .iter()
        .map(pdf_syntax::Name::as_bytes)
        .collect();
    assert_eq!(
        named,
        [b"LogoGreen".as_slice()],
        "named by the mark that painted it"
    );
    assert_colour(
        plane(&separation, Plane::Chromatic)[0],
        [0.58, 1.0, 0.78],
        "the tint transform's cyan, magenta and yellow",
    );
    assert_colour(plane(&separation, Plane::Black)[0], [0.895; 3], "its black");
    assert_colour(plane(&separation, Plane::Spot(0))[0], WHITE, "no spot ink");
}

/// A colourant the page names past the bound and no mark paints is not named: the report is the
/// mark's, never the page's.
#[test]
fn an_unpainted_colourant_past_the_bound_is_not_named() {
    let many = inks(MAX_SPOT_COLOURANTS + 1);
    let separation = separated(file(
        "",
        &format!("/ColorSpace << /A [/DeviceN [{many}] /DeviceCMYK {TINT}] >>"),
        "0 0 0 1 k 0 0 20 20 re f",
        &[],
    ));
    assert!(separation.without_a_plane().is_empty());
}
