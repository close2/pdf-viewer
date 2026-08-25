//! §12.5.4's "completely inside the annotation rectangle", measured in pixels rather than read.
//!
//! `border_precedence_census` states two questions and answers one. Its *placement* half —
//!
//! > If present, the border shall be drawn completely inside the annotation rectangle.
//!
//! — is not a fact about a dictionary, so no amount of parsing can settle it: it is a statement
//! about where ink lands, and only a raster says. This census is that measurement, over both this
//! tree's render and a reference's, on every page of a population whose annotation states a border
//! and no `/AP`.
//!
//! ```sh
//! cargo run --release -p pdf-model --example border_overhang_census              # curated
//! cargo run --release -p pdf-model --example border_overhang_census -- --pdfjs
//! cargo run --release -p pdf-model --example border_overhang_census -- <file.pdf>...
//! ```
//!
//! # What it measures, and why the annotation's own colour is the instrument
//!
//! A border of width *w* drawn completely inside `/Rect` has its stroke's outer edge **on** the
//! rectangle, so its path is the rectangle inset by *w*/2. A border drawn *on* the rectangle is
//! half outside it. The two readings differ by *w*/2 of overhang, and that is the number here.
//!
//! Isolating a border's ink from the page under it is the whole difficulty, and the annotation
//! supplies the discriminator: Table 166's `/C` is "[t]he border of a link annotation" and Table
//! 191's `/MK` `/BC` "[t]he colour of the widget annotation's border", so the ink being looked for
//! has a colour the *file* states. A pixel counts as this border's when it **is** that colour —
//! see [`is_ink`] for why equality rather than nearness, and for the page a nearness test read
//! entirely wrong.
//!
//! Two bands are read around each rectangle. The **inner** one says whether the renderer drew a
//! border at all, which matters because `mupdf` constructs no appearance for a link and would
//! otherwise report a blameless zero. The **outer** one is the finding.
//!
//! # Two things a reader of the output has to keep apart
//!
//! - **The level is contaminated and the difference is not.** Page content in the border's own
//!   colour lands in the ring whoever drew the page, so it raises *both* renders' figures; a ring
//!   one renderer reaches and the other does not is the border. §8.4.1's black is where this bites,
//!   which is why the summary splits on it: a `/C [0 0 0]` border sits in the colour every page of
//!   text is already full of.
//! - **The two renderers do not scan-convert a thin line alike, and the population feels it.** This
//!   counts a border only where some pixel is the stated colour *exactly*, so a stroke narrow or
//!   fractionally placed enough that no pixel is ever covered whole is "drawn by nobody" here.
//!   `poppler` paints such a line solid (§10.7.4 as written) where this tree blends it, so the
//!   reference's population is the larger one — on `issue18030.pdf` its render holds 256 pixels of
//!   the border's `[0 0 .8]` and ours holds none, its nearest being `(159, 159, 236)`. That is a
//!   difference under §10.7.4, not under this clause.
//!
//! # What it will not measure, and says so instead of guessing
//!
//! A rotated page (`/Rotate`), because the device mapping below is the unrotated one and a census
//! that quietly measured the wrong band would be worse than one that declines. A border stated in
//! the paper's own colour, which no ink can be told from. And a band that leaves the raster, where
//! the figure beside it is printed as a floor.
//!
//! **Its population depends on the machine's load, which no other census here does.** A reference
//! render is a subprocess under a time budget, and one the budget kills is `Unmeasurable` — a run
//! on a loaded machine reported 72 of these where the same tree quiet reported 2. Read that count
//! before reading anything under it.

#![expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "an example whose entire output is a measurement"
)]

use std::path::{Path, PathBuf};

use pdf_render::{Raster, Rasterizer, TargetSpec};
use pdf_syntax::{Dictionary, Document, Object};
use pdfref::{Cache, Reference};
use rayon::prelude::*;
use render_cpu::CpuRasterizer;

/// The resolution both renders are taken at: one device pixel per default user space unit.
///
/// The unit is where §12.5.4's arithmetic is stated, so a border of width 1 is one pixel wide and
/// a half-width overhang is half a pixel. Magnifying would make the overhang easier to see and
/// would also make it a measurement of two scan converters; this keeps it a measurement of two
/// placements.
const DPI: u32 = 72;

/// The largest raster either renderer is asked for, in pixels.
const PIXEL_BUDGET: u64 = 1 << 34;

/// How many witnessing pages are printed per finding before the list is truncated.
const MAX_NAMED: usize = 20;

/// How far from the stated colour a pixel may sit and still be the border: four levels a channel,
/// which is a colour conversion's rounding and nothing a blend could hide in.
const ROUNDING: u32 = 12;

/// One annotation whose border this population is about.
struct Candidate {
    /// The page it is on, one-based, as a renderer counts pages.
    page: u32,
    /// `/Subtype`, for reporting: the clause's populations are not one population.
    subtype: String,
    /// `/Rect`, normalised so that the first pair is the lower-left corner.
    rect: [f32; 4],
    /// The border width §12.5.4 gives it, in default user space units.
    width: f32,
    /// The colour it is stroked in, as eight-bit RGB.
    colour: [u8; 3],
    /// The page's crop box, which is the region both renderers are asked to draw.
    crop: [f32; 4],
}

/// What one raster says about one candidate's border.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Placement {
    /// No ink of the border's colour anywhere around the rectangle: this renderer drew no border.
    NotDrawn,
    /// Ink inside the rectangle, and this many whole pixels of it outside.
    Drawn {
        /// The furthest ring outside `/Rect` holding ink of the border's colour, in pixels.
        overhang: u32,
        /// Whether the search reached the edge of the raster before it ran out of rings, so that
        /// the figure beside it is a floor rather than the whole of what escaped.
        clipped: bool,
    },
    /// The rectangle's rings do not fit inside the raster, so nothing is claimed.
    Unmeasurable,
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let named: Vec<String> = arguments
        .iter()
        .filter(|argument| !argument.starts_with("--"))
        .cloned()
        .collect();
    let files = corpus(&arguments, &named);
    eprintln!("{} PDF(s) in the population", files.len());

    let candidates: Vec<(PathBuf, Candidate)> = files
        .par_iter()
        .flat_map_iter(|path| {
            let candidates = std::fs::read(path)
                .ok()
                .and_then(|bytes| Document::open(bytes).ok())
                .map(|document| document_candidates(&document))
                .unwrap_or_default();
            candidates
                .into_iter()
                .map(|candidate| (path.clone(), candidate))
                .collect::<Vec<_>>()
        })
        .collect();

    let documents: std::collections::BTreeSet<&Path> = candidates
        .iter()
        .map(|(path, _)| path.as_path())
        .collect::<std::collections::BTreeSet<_>>();
    println!(
        "{} annotation(s) in {} document(s) state a border this tree strokes and no /AP",
        candidates.len(),
        documents.len()
    );

    let cache = reference_cache();
    let work_root = std::env::temp_dir().join("border-overhang-census");
    let mut measured: Vec<(String, Candidate, Placement, Placement)> = candidates
        .into_par_iter()
        .map(|(path, candidate)| {
            let name = format!(
                "{} page {}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                candidate.page
            );
            let work_dir = work_root.join(format!(
                "{}-p{}",
                path.file_stem().unwrap_or_default().to_string_lossy(),
                candidate.page
            ));
            let ours = render_ours(&path, candidate.page)
                .map_or(Placement::Unmeasurable, |raster| {
                    placement(&raster, &candidate)
                });
            let theirs = cache
                .render(Reference::Poppler, &path, candidate.page, DPI, &work_dir)
                .map_or(Placement::Unmeasurable, |raster| {
                    placement(&raster, &candidate)
                });
            (name, candidate, ours, theirs)
        })
        .collect();
    measured.sort_by(|left, right| left.0.cmp(&right.0));

    report(&measured);
}

/// How far outside `/Rect` one border's ink went, as a printed phrase.
fn overhang(placement: Placement) -> u32 {
    match placement {
        Placement::Drawn { overhang, .. } => overhang,
        Placement::NotDrawn | Placement::Unmeasurable => 0,
    }
}

/// Prints which renderer reached further outside `/Rect`, in the two halves that read apart.
///
/// **The difference between the two is what discriminates, and the level is not.** Page content in
/// the border's own colour lands in the band whoever drew the page, so it raises *both* figures; a
/// band one renderer reaches and the other does not is the border, because the page under the two
/// is the same page.
///
/// The halves are split on §8.4.1's black for that reason: a `/C [0 0 0]` border sits in the one
/// colour every page of text is already full of, so nothing around it can be attributed. The other
/// half is where this reads cleanly, and both are printed because a caution nobody can count is
/// trap 11 with the sign reversed.
fn summarise(both: &[&(String, Candidate, Placement, Placement)]) {
    for (label, wanted) in [
        ("in a colour of its own", false),
        ("in §8.4.1's black", true),
    ] {
        let group: Vec<&&(String, Candidate, Placement, Placement)> = both
            .iter()
            .filter(|(_, candidate, _, _)| is_initial_black(candidate.colour) == wanted)
            .collect();
        let poppler_further = group
            .iter()
            .filter(|(_, _, ours, theirs)| overhang(*theirs) > overhang(*ours))
            .count();
        let ours_further = group
            .iter()
            .filter(|(_, _, ours, theirs)| overhang(*ours) > overhang(*theirs))
            .count();
        println!(
            "    of the {} {label}: poppler reaches further outside than ours on \
             {poppler_further}, ours further on {ours_further}, and they agree on {}",
            group.len(),
            group
                .len()
                .saturating_sub(poppler_further)
                .saturating_sub(ours_further)
        );
    }
}

/// How far past `/Rect` one render put a border, as a printed phrase.
fn beyond(placement: Placement) -> String {
    match placement {
        Placement::Drawn {
            overhang: 0,
            clipped: _,
        } => "inside".to_owned(),
        Placement::Drawn { overhang, clipped } => {
            format!(
                "{overhang} px beyond{}",
                if clipped { " (a floor)" } else { "" }
            )
        }
        Placement::NotDrawn => "no border".to_owned(),
        Placement::Unmeasurable => "unmeasurable".to_owned(),
    }
}

/// Prints what the population says, in the three groups a reader has to keep apart.
fn report(measured: &[(String, Candidate, Placement, Placement)]) {
    let drawn_by_us = measured
        .iter()
        .filter(|(_, _, ours, _)| matches!(ours, Placement::Drawn { .. }))
        .count();
    let drawn_by_them = measured
        .iter()
        .filter(|(_, _, _, theirs)| matches!(theirs, Placement::Drawn { .. }))
        .count();
    let unmeasurable = measured
        .iter()
        .filter(|(_, _, ours, theirs)| {
            *ours == Placement::Unmeasurable || *theirs == Placement::Unmeasurable
        })
        .count();
    println!("  ours draws a border on {drawn_by_us}, poppler on {drawn_by_them}");
    println!("  {unmeasurable} could not be measured on one side or the other");

    let both: Vec<&(String, Candidate, Placement, Placement)> = measured
        .iter()
        .filter(|(_, _, ours, theirs)| {
            matches!(ours, Placement::Drawn { .. }) && matches!(theirs, Placement::Drawn { .. })
        })
        .collect();
    println!(
        "  {} where both draw one, which is the comparison",
        both.len()
    );
    for (name, candidate, ours, theirs) in &both {
        println!(
            "      {name}: {} width {} in {:?} — ours {}, poppler {}",
            candidate.subtype,
            candidate.width,
            candidate.colour,
            beyond(*ours),
            beyond(*theirs)
        );
    }

    summarise(&both);
    let ours_outside = both
        .iter()
        .filter(|(_, _, ours, _)| overhang(*ours) > 0)
        .count();
    let theirs_outside = both
        .iter()
        .filter(|(_, _, _, theirs)| overhang(*theirs) > 0)
        .count();
    println!(
        "    of those, ink outside /Rect: ours on {ours_outside}, poppler on {theirs_outside}"
    );

    for (label, side) in [("ours", 0_u8), ("poppler", 1)] {
        let mut named = 0_usize;
        println!("    {label} outside /Rect:");
        for (name, candidate, ours, theirs) in &both {
            let pixels = overhang(if side == 0 { *ours } else { *theirs });
            if pixels == 0 {
                continue;
            }
            named = named.saturating_add(1);
            if named <= MAX_NAMED {
                let clipped = matches!(
                    if side == 0 { ours } else { theirs },
                    Placement::Drawn { clipped: true, .. }
                );
                let floor = if clipped {
                    " (a floor: the raster ends)"
                } else {
                    ""
                };
                println!(
                    "      {name}: {pixels} px beyond /Rect{floor}, {} width {} in {:?}",
                    candidate.subtype, candidate.width, candidate.colour
                );
            }
        }
        if named > MAX_NAMED {
            println!("      … and {} more", named.saturating_sub(MAX_NAMED));
        }
        if named == 0 {
            println!("      none");
        }
    }
}

/// Where the reference renders are remembered, on the oracle's own rules (`pdfref::cache`).
fn reference_cache() -> Cache {
    match std::env::var("PDFREF_CACHE") {
        Ok(value) if value.eq_ignore_ascii_case("off") => Cache::disabled(),
        Ok(value) if !value.trim().is_empty() => Cache::at(value),
        _ => Cache::disabled(),
    }
}

/// Every PDF this census measures over, in the scope the command line asked for.
fn corpus(arguments: &[String], named: &[String]) -> Vec<PathBuf> {
    if !named.is_empty() {
        return named.iter().map(PathBuf::from).collect();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let roots: &[&str] = if arguments.iter().any(|argument| argument == "--pdfjs") {
        &["doc/pdf.js/test/pdfs"]
    } else {
        &["doc/pdf.js/test/pdfs", "doc/corpora", "doc/corpora-own"]
    };
    let mut files = Vec::new();
    for relative in roots {
        collect(&root.join(relative), &mut files);
    }
    files.sort();
    files.dedup();
    files
}

/// Every `.pdf` under one directory, recursively.
fn collect(dir: &Path, into: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, into);
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            into.push(path);
        }
    }
}

/// Every annotation in one document whose border this tree constructs and strokes.
fn document_candidates(document: &Document) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let pages = pdf_model::Pages::new(document);
    for index in 0..pages.len() {
        let Some(page) = pages.get(index) else {
            continue;
        };
        // A rotated page's rectangle reaches the raster through a transform this census does not
        // apply, and measuring the wrong band silently is the failure this declines.
        if page.rotate != 0 {
            continue;
        }
        let entry = document.get_key(&page.dict, "Annots");
        let Some(list) = entry.as_array() else {
            continue;
        };
        for item in list {
            let object = document.resolve(item);
            let Some(annotation) = object.as_dict() else {
                continue;
            };
            // §12.5.2: a stored appearance is drawn "without regard to any other keys", so a
            // border nothing constructs cannot be misplaced by this tree or by a reference.
            if !matches!(document.get_key(annotation, "AP"), Object::Null) {
                continue;
            }
            let Some(candidate) = candidate(document, annotation, &page, index) else {
                continue;
            };
            candidates.push(candidate);
        }
    }
    candidates
}

/// One annotation's border, or `None` where nothing would be stroked.
fn candidate(
    document: &Document,
    annotation: &Dictionary,
    page: &pdf_model::Page,
    index: usize,
) -> Option<Candidate> {
    let entry = document.get_key(annotation, "Subtype");
    let subtype = String::from_utf8_lossy(entry.as_name()?.as_bytes()).into_owned();
    // The two subtypes whose `/BS` is §12.5.4's rectangle around `/Rect`. The clause gives "line,
    // square, circle, and ink annotations" a `/BS` that states the width of the annotation's *own*
    // marks instead, and those have no rectangle to be inside of.
    let characteristics = document.get_key(annotation, "MK");
    let colour = match subtype.as_str() {
        // Table 166's `/C`: "The border of a link annotation".
        "Link" => colour(document, annotation, "C")?,
        // Table 191's `/MK` `/BC`: "The colour of the widget annotation's border".
        "Widget" => colour(document, characteristics.as_dict()?, "BC")?,
        _ => return None,
    };
    let width = border_width(document, annotation);
    if width <= 0.0 {
        return None;
    }
    // A border the colour of the paper cannot be told from the paper, so where its ink lands is
    // not a question this instrument can be asked. `160F-2019.pdf` states three widgets with
    // `/MK << /BC [1.0] >>`, and reading them found a white border everywhere on the page — in
    // *both* renders, which is the tell rather than a finding (trap 11).
    if is_paper(colour) {
        return None;
    }
    let rect = rectangle(document, &document.get_key(annotation, "Rect"))?;
    Some(Candidate {
        page: u32::try_from(index.saturating_add(1)).ok()?,
        subtype,
        rect,
        width,
        colour,
        crop: page.crop_box,
    })
}

/// §12.5.4's width, by Table 166's precedence: a `/BS` present means `/Border` is ignored.
fn border_width(document: &Document, annotation: &Dictionary) -> f32 {
    /// §12.5.4: "If neither the Border nor the BS entry is present, the border shall be drawn as a
    /// solid line with a width of 1 point."
    const DEFAULT: f32 = 1.0;

    let width = if let Some(style) = document.get_key(annotation, "BS").as_dict() {
        number(document, Some(&document.get_key(style, "W"))).unwrap_or(DEFAULT)
    } else {
        document
            .get_key(annotation, "Border")
            .as_array()
            .map_or(DEFAULT, |border| {
                border
                    .get(2)
                    .and_then(|item| number(document, Some(&document.resolve(item))))
                    .unwrap_or(DEFAULT)
            })
    };
    if width.is_finite() {
        width.max(0.0)
    } else {
        0.0
    }
}

/// A colour entry as eight-bit RGB, or `None` where the entry states no colour at all.
///
/// Table 166 gives an *empty* array the meaning "No colour; transparent", which is the one case
/// that has to be told apart from an absent entry rather than defaulted.
fn colour(document: &Document, dictionary: &Dictionary, key: &'static str) -> Option<[u8; 3]> {
    let entry = document.get_key(dictionary, key);
    let values = entry.as_array()?;
    let components: Vec<f32> = values
        .iter()
        .filter_map(|item| number(document, Some(&document.resolve(item))))
        .map(|value| value.clamp(0.0, 1.0))
        .collect();
    let rgb = match components.as_slice() {
        [grey] => [*grey, *grey, *grey],
        [red, green, blue] => [*red, *green, *blue],
        // §10.4.2.5's conversion, which is what an annotation colour in four components is.
        [cyan, magenta, yellow, black] => [
            1.0 - (cyan + black).min(1.0),
            1.0 - (magenta + black).min(1.0),
            1.0 - (yellow + black).min(1.0),
        ],
        _ => return None,
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "each component is clamped into 0.0..=1.0 before it is scaled"
    )]
    Some(rgb.map(|component| (component * 255.0).round() as u8))
}

/// A rectangle entry, normalised so the first pair is the lower-left corner.
fn rectangle(document: &Document, entry: &Object) -> Option<[f32; 4]> {
    let values = entry.as_array()?;
    let numbers: Vec<f32> = values
        .iter()
        .filter_map(|item| number(document, Some(&document.resolve(item))))
        .collect();
    let [x0, y0, x1, y1] = <[f32; 4]>::try_from(numbers.as_slice()).ok()?;
    Some([x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)])
}

/// One number out of an object, where it is finite.
fn number(document: &Document, object: Option<&Object>) -> Option<f32> {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a border width or a rectangle edge is a small number of user space units"
    )]
    let value = document.resolve(object?).as_number()? as f32;
    value.is_finite().then_some(value)
}

/// Our own render of a page at [`DPI`].
fn render_ours(path: &Path, page: u32) -> Option<Raster> {
    let index = usize::try_from(page.saturating_sub(1)).ok()?;
    let document = Document::open(std::fs::read(path).ok()?).ok()?;
    let page = pdf_model::Pages::new(&document).get(index)?;
    let list = pdf_model::interpret(&document, &page).display_list;
    let target = TargetSpec::for_page(&list, 1.0, PIXEL_BUDGET).ok()?;
    CpuRasterizer::new().rasterize(&list, target).ok()
}

/// Where one raster puts one border's ink, relative to the rectangle it must be inside.
fn placement(raster: &Raster, candidate: &Candidate) -> Placement {
    let (width, height) = (
        candidate.crop[2] - candidate.crop[0],
        candidate.crop[3] - candidate.crop[1],
    );
    if width <= 0.0 || height <= 0.0 || raster.width == 0 || raster.height == 0 {
        return Placement::Unmeasurable;
    }
    // Each raster's own size against the crop box, rather than one assumed scale: two renderers
    // asked for the same region can round its pixel count differently, and a band measured
    // through the other one's rounding is a band in the wrong place.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster dimension is far below f32's exact integer range"
    )]
    let (scale_x, scale_y) = (raster.width as f32 / width, raster.height as f32 / height);
    let device = [
        (candidate.rect[0] - candidate.crop[0]) * scale_x,
        (candidate.crop[3] - candidate.rect[3]) * scale_y,
        (candidate.rect[2] - candidate.crop[0]) * scale_x,
        (candidate.crop[3] - candidate.rect[1]) * scale_y,
    ];
    if !fits(raster, device) {
        return Placement::Unmeasurable;
    }

    // How far each search runs: the border's own thickness inwards, since that is where its ink
    // is, and the same outwards, since a stroke centred on the rectangle's edge reaches exactly
    // half of it beyond — plus one pixel, so that a border one unit wide has somewhere to be
    // found whichever way its ends round.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the width is finite and positive, and the result is clamped"
    )]
    let reach = ((candidate.width * scale_x.max(scale_y)).ceil() as u32).clamp(1, 4096);

    let mut inside = false;
    for distance in 0..=reach {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a search distance is bounded by 4097 above"
        )]
        let offset = distance as f32;
        let box_ = [
            device[0] + offset,
            device[1] + offset,
            device[2] - offset,
            device[3] - offset,
        ];
        if box_[0] >= box_[2] || box_[1] >= box_[3] {
            break;
        }
        inside = inside || holds_ink(raster, box_, candidate.colour);
    }
    if !inside {
        // Ink outside with none inside is not this border overhanging; it is page content that
        // happens to share the colour, and reporting it would be trap 11's shape.
        return Placement::NotDrawn;
    }

    let mut overhang = 0;
    let mut clipped = false;
    for distance in 1..=reach.saturating_add(1) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a search distance is bounded by 4098 above"
        )]
        let offset = distance as f32;
        let box_ = [
            device[0] - offset,
            device[1] - offset,
            device[2] + offset,
            device[3] + offset,
        ];
        // A ring the raster does not hold whole cannot be searched, and every ring past it is
        // further out still — so the search stops and says it was cut short rather than
        // reporting the absence of what it could not look at.
        if !fits(raster, box_) {
            clipped = true;
            break;
        }
        if holds_ink(raster, box_, candidate.colour) {
            overhang = distance;
        }
    }
    Placement::Drawn { overhang, clipped }
}

/// Whether a device rectangle's one-pixel outline lies within a raster.
fn fits(raster: &Raster, box_: [f32; 4]) -> bool {
    box_[0] >= 0.0
        && box_[1] >= 0.0
        && box_[2] <= pixels(raster.width)
        && box_[3] <= pixels(raster.height)
        && box_[0] < box_[2]
        && box_[1] < box_[3]
}

/// A raster dimension as a coordinate.
fn pixels(dimension: u32) -> f32 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster dimension is far below f32's exact integer range"
    )]
    let value = dimension as f32;
    value
}

/// Whether the one-pixel outline of a device rectangle holds ink of a colour.
fn holds_ink(raster: &Raster, box_: [f32; 4], colour: [u8; 3]) -> bool {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "each coordinate is inside the raster by the caller's own test"
    )]
    let (left, top, right, bottom) = (
        box_[0].floor().max(0.0) as u32,
        box_[1].floor().max(0.0) as u32,
        (box_[2].ceil() as u32)
            .saturating_sub(1)
            .min(raster.width.saturating_sub(1)),
        (box_[3].ceil() as u32)
            .saturating_sub(1)
            .min(raster.height.saturating_sub(1)),
    );
    if left > right || top > bottom {
        return false;
    }
    (left..=right).any(|x| is_ink(raster, x, top, colour) || is_ink(raster, x, bottom, colour))
        || (top..=bottom)
            .any(|y| is_ink(raster, left, y, colour) || is_ink(raster, right, y, colour))
}

/// Whether a colour is §8.4.1's initial black, which every page of text is already full of.
fn is_initial_black(colour: [u8; 3]) -> bool {
    colour
        .iter()
        .all(|component| u32::from(*component) * 3 <= ROUNDING)
}

/// Whether a colour is the page's own background, which no ink can be told from.
fn is_paper(colour: [u8; 3]) -> bool {
    colour
        .iter()
        .all(|component| u32::from(255_u8.abs_diff(*component)) * 3 <= ROUNDING)
}

/// Whether one pixel *is* a border's stated colour, within eight-bit rounding.
///
/// A pixel the border covers whole is that colour exactly; anything else is a blend with whatever
/// is under it. So the test is equality rather than nearness, and both halves of that matter:
///
/// - **Conservative.** A pixel a stroke covers half of blends halfway to the paper and fails this,
///   so a measured overhang is a lower bound on the real one and a sub-pixel border is reported as
///   drawn by nobody.
/// - **Discriminating.** *Nearness* is what this asked first, and on `issue17056.pdf` — 31 links
///   whose `/C` is `[0 0 0.5]` over a page of black text — it called every black glyph within two
///   pixels of a rectangle that annotation's border, and reported this tree two pixels outside
///   `/Rect` on all 31. A dark colour is nearer any dark ink than it is to white, which makes
///   nearness a test of the *page* rather than of the border (trap 11).
fn is_ink(raster: &Raster, x: u32, y: u32, colour: [u8; 3]) -> bool {
    let Some(pixel) = pixel(raster, x, y) else {
        return false;
    };
    let distance: u32 = (0..3)
        .map(|channel| u32::from(pixel[channel].abs_diff(colour[channel])))
        .sum();
    distance <= ROUNDING
}

/// One pixel of a raster, as eight-bit RGB.
fn pixel(raster: &Raster, x: u32, y: u32) -> Option<[u8; 3]> {
    let channels = match raster.format {
        pdf_render::RasterFormat::Rgba8 => 4_usize,
    };
    let index = usize::try_from(y)
        .ok()?
        .checked_mul(usize::try_from(raster.width).ok()?)?
        .checked_add(usize::try_from(x).ok()?)?
        .checked_mul(channels)?;
    let bytes = raster.data.get(index..index.checked_add(3)?)?;
    Some([bytes[0], bytes[1], bytes[2]])
}
