//! A page whose `Adobe-Identity` text no reader can name, photographed as the window draws it
//! (owner answer A570's second remedy, ADR 2394).
//!
//! # What is missing, and why no font answers it
//!
//! ISO 32000-2 §9.7.5.2:
//!
//! > The Identity-H and Identity-V CMaps shall not be used with a non-embedded font.
//!
//! Under `Identity` ordering a CID indexes the glyphs of the program that defined it (§9.7.4.2), and
//! that program is not in the file. Re-keying (`super::rekeyed`) answers the half whose `/ToUnicode`
//! states a character for every code; this module is the other half — no map, or a map that names
//! nothing the page shows. **It chooses no font**, because the font is not the missing piece: the
//! per-CID correspondence is, and assigning glyphs to the numbers without it would invent which
//! character each number names. So the page is photographed instead: the window's own rendering,
//! the marks it cannot draw left blank exactly as the window leaves them (ADR 0270), written as one
//! image covering the page. Its worth is salvage — the graphics, the images and every other mark
//! the window draws survive as pixels, where a refusal archives nothing.
//!
//! # The owner's gate, and how it is decided
//!
//! A570: the image is taken only where the input's text was never machine-readable anywhere, so
//! that nothing extractable is lost. The page is interpreted once, with its annotations left out,
//! because they stay annotations and keep whatever they carry; that one interpretation is both the
//! evidence for the gate and the picture that is written, so what was judged is what was drawn.
//! The page is refused by name where:
//!
//! - **its readback names any character** — §9.10.2's mapping, §14.9.4's replacement text, any font
//!   on the page: the image would turn text a reader can extract into pixels;
//! - **the interpreter reports anything but the fonts it could not draw**: a mark the window did
//!   not draw is one another reader might, and the photograph would fix its absence into the file;
//! - **the document states a structure tree** (§14.7), whose content items would be left pointing
//!   at marked content no longer on the page, unless the profile authorised
//!   [`crate::archive::Loss::TaggingOfPhotographedPages`] (below), **or optional content** (§8.11),
//!   whose other configurations a photograph of one would discard, unless the profile authorised
//!   [`crate::archive::Loss::OptionalContentOfPhotographedPages`] (below).
//!
//! # The wider gate (A694 item 1)
//!
//! Where the profile authorises [`crate::archive::Loss::PageAsImage`] (`as-if-printed`: what a company does when it
//! prints a letter and scans the print), the first condition is lifted and two populations join
//! the pages photographed: a page showing a font that any route of this program refused to embed
//! a face into, and a page holding a fault of its content stream the target refuses
//! (`super::super::fenced`). The page's text is then named — in the report and in the file's
//! history — because it is what the photograph turns into pixels.
//!
//! # A tagged document (A707)
//!
//! Where the profile also authorises [`crate::archive::Loss::TaggingOfPhotographedPages`], a
//! document that states a structure tree is photographed too, and the tree's leaves that name a
//! photographed page are taken out of it (`super::untagged`): the loss is then exactly the pages
//! the photograph already turned into pixels, and every other page keeps its tags.
//!
//! # A document with optional content (A717)
//!
//! Where the profile also authorises
//! [`crate::archive::Loss::OptionalContentOfPhotographedPages`], a document that states optional
//! content is photographed too, and the photograph bakes in the configuration this viewer shows:
//! the page is interpreted under `pdf_model::view::ViewState::of`, the state the window opens the
//! document in, which is §8.11.4.3's default configuration as §8.11.4.5 applies it for viewing,
//! and never under §8.11.4.4's `Print` usage, because a conversion imitates no press (A694
//! item 2). What the photograph discards is each group's other state on that page, so every group
//! the page's content names (`pdf_model::optional_content::groups_referenced_by_content`) is
//! recorded by its `/Name` with the state it was drawn in. The annotations stay annotations with
//! their own `/OC`, the other pages keep their optional content, and `/OCProperties` stays.
//!
//! # Where it is offered
//!
//! At Level B of ISO 19005-2 and at every flavour of ISO 19005-4, which ask no Unicode of a page;
//! Level U and Level A exist to keep a page's text, so there the font keeps the refusal it had.
//!
//! # The picture
//!
//! At [`DOTS_PER_INCH`], over §8.3.2.3's 72 units to the inch — a documented choice, since no clause
//! states a resolution for this, made so that a ten-point line of the text that does draw stays
//! legible on paper. The samples are the oracle backend's, composited onto white by §11.4.7 before
//! they leave it, and are written as `ICCBased` over the shipped sRGB profile: the colour the
//! window showed, which `super::super::default_rgb` explains is what that profile states on this
//! reader. The image is placed by the inverse of the page's own transform from default user space
//! (`pdf_model::content::base_transform`), so `/Rotate`, the crop box's origin and `/UserUnit` stay
//! the page's and the annotations keep their places over it.

use std::collections::{BTreeMap, BTreeSet};

use pdf_archive::survey::SelectedFont;
use pdf_archive::{Level, Target};
use pdf_model::Pages;
use pdf_model::content::Unsupported;
use pdf_model::content::ledger::{Ledger, Outcome};
use pdf_model::optional_content::OptionalContent;
use pdf_render::{Point, Rasterizer as _, TargetSpec, Transform};
use pdf_syntax::Document;
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId, Stream};
use pdf_syntax::serialize::flate_encode;
use render_cpu::CpuRasterizer;

use super::super::COMPRESSION_LEVEL;
use super::super::decision::{Because, Loss};
use super::super::prepare::{SRGB, Spare, profile_stream};
use super::Substitutes;

/// The resolution a page is photographed at, in dots per inch.
pub const DOTS_PER_INCH: u16 = 300;

/// The name the one image goes under in the photographed page's resources.
const IMAGE: &[u8] = b"Photograph";

/// A font the window cannot draw, held until the pages that show it are known.
#[derive(Debug)]
pub(super) struct Deferred {
    /// How a content stream's lookup finds it: its object, or its dictionary as a direct entry.
    selected: Outcome,
    /// The `/BaseFont` the document states, for the report.
    requested: String,
    /// The refusal the font keeps where no page that shows it can be photographed.
    refusal: Because,
    /// Whether the font states a `/ToUnicode` stream, so that re-keying was tried and refused.
    ///
    /// Such a font's map names characters for some codes as a rule, and where those make the
    /// page's text extractable the re-keying route's own sentence says more than the gate's.
    mapped: bool,
    /// Whether the font is held by the wider gate: some route refused it, and it is photographed
    /// around only where the profile authorised [`crate::archive::Loss::PageAsImage`].
    anywhere: bool,
}

/// One page this conversion photographed: what the report and the file's history name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhotographedPage {
    /// The page's number, from one.
    pub page: usize,
    /// The fonts on it the window could not draw, by the `/BaseFont` each states.
    pub fonts: Vec<String>,
    /// The resolution it was photographed at.
    pub dots_per_inch: u16,
    /// The picture's width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// The text the page showed, with its white space collapsed, as far as [`MAX_TEXT_CHARS`] and
    /// no further: what the photograph turned into pixels. Empty for a page that showed none.
    pub text: String,
    /// How many characters of the page's text [`Self::text`] does not repeat.
    pub text_elided: usize,
    /// The requirements whose failure on this page the photograph answers, besides the fonts.
    pub requirements: Vec<String>,
    /// Whether the wider gate admitted the page, which the narrow one would have refused; the
    /// loss [`crate::archive::Loss::PageAsImage`] is incurred then, beside [`crate::archive::Loss::TextAsImage`].
    pub anywhere: bool,
    /// How many leaves of the document's structure tree named this page and were taken out of the
    /// tree with its content (ISO 32000-2 section 14.7.5): the loss
    /// [`crate::archive::Loss::TaggingOfPhotographedPages`] where it is not zero.
    pub structure_leaves: usize,
    /// The optional content groups the page's content named, each in the state the photograph
    /// drew it in, where the document states optional content (ISO 32000-2 section 8.11): the
    /// loss [`crate::archive::Loss::OptionalContentOfPhotographedPages`] where it is `Some`, even
    /// with no group listed, since the page was drawn in one configuration all the same.
    pub optional_content: Option<FixedGroups>,
}

/// The optional content groups one photographed page's content named, as far as
/// [`MAX_GROUPS_NAMED`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FixedGroups {
    /// Each group, in the order of its object number.
    pub groups: Vec<FixedGroup>,
    /// How many more groups the content named than [`Self::groups`] lists.
    pub left_out: usize,
}

/// One optional content group a photograph fixed in the state it was drawn in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedGroup {
    /// The group's `/Name` (ISO 32000-2 section 8.11.2.1's Table 96), or its object number where
    /// it states none.
    pub name: String,
    /// The state it was drawn in: `Some(true)` on, `Some(false)` off, and `None` for a group the
    /// configuration's `/Intent` does not consider, which decided nothing (section 8.11.2.3).
    pub drawn: Option<bool>,
}

impl FixedGroup {
    /// The state drawn, as the report and the history name it.
    #[must_use]
    pub const fn state_word(&self) -> &'static str {
        match self.drawn {
            Some(true) => "on",
            Some(false) => "off",
            None => "not considered",
        }
    }
}

impl FixedGroups {
    /// The groups as a clause for the report and the history: each name in quotation marks with
    /// its state, or a sentence saying the page's content named none.
    #[must_use]
    pub fn named(&self) -> String {
        if self.groups.is_empty() {
            return "its content's resources name no optional content group".to_owned();
        }
        let listed: Vec<String> = self
            .groups
            .iter()
            .map(|group| format!("\"{}\" {}", group.name, group.state_word()))
            .collect();
        let more = if self.left_out == 0 {
            String::new()
        } else {
            format!(" and {} more", self.left_out)
        };
        format!(
            "the optional content groups it was drawn with, fixed as drawn: {}{more}",
            listed.join(", ")
        )
    }
}

impl PhotographedPage {
    /// The losses this page incurred, in the order the report names them.
    #[must_use]
    pub fn losses(&self) -> Vec<Loss> {
        let mut losses = vec![Loss::TextAsImage];
        if self.anywhere {
            losses.push(Loss::PageAsImage);
        }
        if self.structure_leaves > 0 {
            losses.push(Loss::TaggingOfPhotographedPages);
        }
        if self.optional_content.is_some() {
            losses.push(Loss::OptionalContentOfPhotographedPages);
        }
        losses
    }

    /// [`Self::losses`] as the words a caller authorises them by, joined for a sentence.
    #[must_use]
    pub fn loss_words(&self) -> String {
        let words: Vec<&str> = self.losses().into_iter().map(Loss::word).collect();
        match words.split_last() {
            Some((last, [])) => (*last).to_owned(),
            Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
            None => String::new(),
        }
    }
}

/// What a photographed page's dictionary is given in place of its content.
#[derive(Debug)]
pub(in super::super) struct Photograph {
    /// The content stream that paints the image.
    pub(in super::super) contents: ObjectId,
    /// The resource dictionary naming the image and nothing else.
    pub(in super::super) resources: Dictionary,
    /// Whether the document's structure tree was pruned of this page's leaves, so that the page's
    /// `/StructParents` (ISO 32000-2 section 14.7.5.4), which named the marked content that is
    /// gone, goes with them.
    pub(in super::super) untagged: bool,
}

/// Whether a target admits a page photographed with no text on it: Level B of ISO 19005-2 and
/// every flavour of ISO 19005-4 (the module comment's *Where it is offered*).
pub(super) const fn offered(target: Target) -> bool {
    matches!(target, Target::Two(Level::B) | Target::Four(_))
}

/// How many characters of a photographed page's text the report and the history name; the rest is
/// counted, so that a page of a book does not become a megabyte of report.
pub const MAX_TEXT_CHARS: usize = 1_000;

/// How many optional content groups the report and the history name for one photographed page;
/// the rest are counted, so that a page naming a document's thousand layers does not become a
/// thousand names in every entry.
pub const MAX_GROUPS_NAMED: usize = 64;

/// How many characters of one group's `/Name` the report and the history repeat.
pub const MAX_GROUP_NAME_CHARS: usize = 200;

/// How many pages the wider gate photographs in one conversion. Each is a raster at
/// [`DOTS_PER_INCH`] held until the file is written, so a document that fails on every page of a
/// long book is refused by name rather than held in memory. The pages the narrow gate admits are
/// not counted.
pub const MAX_PHOTOGRAPHED_PAGES: usize = 32;

/// How many pixels the pages the wider gate photographs hold together: what one page may be drawn
/// to (`Budget::max_pixels`), so that a document of hostile pages costs what one hostile page did.
/// About thirty-one letter-size pages at [`DOTS_PER_INCH`].
const MAX_PHOTOGRAPHED_PIXELS: u64 = 1 << 28;

/// What decides whether pages are photographed beyond the narrow gate.
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Gate<'a> {
    /// Whether the target admits a page with no text layer ([`offered`]).
    pub(in super::super) offered: bool,
    /// Whether the profile authorised [`crate::archive::Loss::PageAsImage`], so that a page is photographed
    /// whatever text it shows and the fonts the other routes refused are photographed around.
    pub(in super::super) anywhere: bool,
    /// The pages (zero-based) that hold a fault of their content stream, with the requirements,
    /// where `anywhere` (`super::super::fenced::pages`).
    pub(in super::super) fenced: &'a super::super::fenced::Fenced,
    /// What a photograph may take from a document that states more than its pages.
    pub(in super::super) beyond: Beyond,
}

/// The losses a photographed page carries in a document that states a structure tree or optional
/// content, each authorised only at a target that offers a photograph.
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct Beyond {
    /// Whether the profile authorised [`crate::archive::Loss::TaggingOfPhotographedPages`], so
    /// that a document stating a structure tree is photographed and the tree's leaves that name a
    /// photographed page are taken out of it.
    pub(in super::super) untags: bool,
    /// Whether the profile authorised [`crate::archive::Loss::OptionalContentOfPhotographedPages`],
    /// so that a document stating optional content is photographed in the configuration this
    /// viewer shows.
    pub(in super::super) layers: bool,
}

impl<'a> Gate<'a> {
    /// The gate for `target`: the wider gate only where the target offers a photographed page at
    /// all, and nothing fenced is photographed where it does not.
    pub(in super::super) fn new(
        target: Target,
        (anywhere, untags, layers): (bool, bool, bool),
        fenced: &'a super::super::fenced::Fenced,
    ) -> Self {
        Self {
            offered: offered(target),
            anywhere: anywhere && offered(target),
            fenced,
            beyond: Beyond {
                untags: untags && offered(target),
                layers: layers && offered(target),
            },
        }
    }

    /// Whether `target` offers a photographed page with no text layer.
    pub(in super::super) const fn admits(target: Target) -> bool {
        offered(target)
    }
}

/// The one sRGB profile every photograph of this document names, written at the first.
fn srgb_profile(
    document: &Document,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
    profile: &mut Option<ObjectId>,
) -> Result<ObjectId, Because> {
    if let Some(icc) = *profile {
        return Ok(icc);
    }
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(NO_SPARE_OBJECT))?;
    substitutes.written.insert(
        at,
        profile_stream(SRGB, pdf_archive::survey::DeviceFamily::Rgb),
    );
    *profile = Some(at);
    Ok(at)
}

/// Holds a font whose glyphs no face can be keyed to until its pages are known.
///
/// Nothing is refused here: the decision is [`photograph`]'s, once every font is in.
pub(super) fn defer(
    document: &Document,
    used: &SelectedFont,
    refusal: Because,
    anywhere: bool,
    substitutes: &mut Substitutes,
) {
    substitutes.deferred.push(Deferred {
        selected: super::selected(used),
        requested: super::base_font(document, &used.dict),
        refusal,
        mapped: matches!(document.get_key(&used.dict, "ToUnicode"), Object::Stream(_)),
        anywhere,
    });
}

/// Photographs every page that shows a deferred font or holds a fenced fault, or refuses the first
/// that may not be.
///
/// `gate.offered` is whether the target admits a page with no text at all (the module comment's
/// *Where it is offered*); where it does not, the first deferred font's own refusal stands.
///
/// # Errors
///
/// The gate's sentence for a page it refuses, a deferred font's own refusal where no page's content
/// shows it, and a sentence for a page that cannot be drawn within the budget.
pub(super) fn photograph(
    document: &Document,
    gate: Gate<'_>,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    if substitutes.deferred.is_empty() && gate.fenced.pages.is_empty() {
        return Ok(());
    }
    if !gate.offered {
        return match substitutes.deferred.first() {
            Some(first) => Err(first.refusal),
            None => Err(Because::NotBuiltYet(NOT_OFFERED)),
        };
    }
    let Stated {
        structured,
        layered,
        configuration,
    } = what_the_document_states(document, gate)?;
    let every = Pages::new(document).every();
    if gate.anywhere {
        refuse_fenced_pages_past_the_bounds(&every, gate)?;
    }
    let mut reached = vec![false; substitutes.deferred.len()];
    let mut profile = None;
    // Each page photographed here, by its object and its place in `substitutes.photographed`.
    let mut taken_here: Vec<(ObjectId, usize)> = Vec::new();
    for (index, page) in every.into_iter().enumerate() {
        let Some(page) = page else {
            continue;
        };
        let content = crate::render::page_to_draw(&page, None, false);
        let (interpretation, ledger) = pdf_model::content::interpret_ledgered(document, &content);
        if ledger.truncated() {
            return Err(Because::NotBuiltYet(LEDGER_TRUNCATED));
        }
        let shown = shown_here(&ledger, &substitutes.deferred);
        let fenced = gate.fenced.pages.get(&index);
        if shown.is_empty() && fenced.is_none() {
            continue;
        }
        // The bounds belong to the wider gate: a page the narrow gate admits is not counted.
        let wide = fenced.is_some()
            || interpretation.text.chars().any(|c| !c.is_whitespace())
            || shown
                .iter()
                .any(|&font| substitutes.deferred.get(font).is_some_and(|d| d.anywhere));
        let held = || substitutes.photographed.iter().filter(|p| p.anywhere);
        if wide && held().count() >= MAX_PHOTOGRAPHED_PAGES {
            return Err(Because::NotBuiltYet(TOO_MANY_PAGES));
        }
        admitted(&interpretation, &shown, &substitutes.deferred, gate)?;
        let id = page.id.ok_or(Because::NotBuiltYet(PAGE_NOT_AN_OBJECT))?;
        let pixels_left = pixels_left(held());
        let raster = taken(
            &interpretation.display_list,
            if wide { pixels_left } else { u64::MAX },
        )?;
        let placed = placement(
            &content,
            interpretation.display_list.page_size.height,
            &raster,
        )?;
        let icc = srgb_profile(document, spare, substitutes, &mut profile)?;
        let photograph = written(document, &raster, icc, placed, spare, substitutes)?;
        substitutes.photographs.insert(id, photograph);
        for &font in &shown {
            if let Some(seen) = reached.get_mut(font) {
                *seen = true;
            }
        }
        taken_here.push((id, substitutes.photographed.len()));
        let mut taken = photographed_page(
            index,
            &interpretation.text,
            (&shown, fenced),
            &raster,
            &substitutes.deferred,
        );
        if layered {
            taken.optional_content = Some(fixed_groups(document, &page, configuration.as_ref()));
        }
        substitutes.photographed.push(taken);
    }
    // A font no page's content showed was reached some other way — an annotation's appearance is
    // the one the content-only interpretation leaves out — and a photograph of a page cannot take
    // it out of the file, so it keeps the refusal it had.
    if let Some((font, _)) = substitutes
        .deferred
        .iter()
        .zip(&reached)
        .find(|(_, reached)| !**reached)
    {
        return Err(font.refusal);
    }
    if structured && !taken_here.is_empty() {
        untag(document, &taken_here, substitutes)?;
    }
    // A substituted font only the photographed content selected went with that content, so the
    // report and the file's history name only the fonts the output still holds (`doc/adr/2454`).
    if !substitutes.photographs.is_empty() {
        let held = still_held(document, &substitutes.photographs, &substitutes.done);
        substitutes.done.retain(|font| held.contains(&font.font));
    }
    Ok(())
}

/// What the gate's two conditions on the document as a whole found.
struct Stated {
    /// Whether it states a structure tree, whose leaves are taken out for each page photographed.
    structured: bool,
    /// Whether it states `/OCProperties`, so that each page photographed incurs
    /// [`crate::archive::Loss::OptionalContentOfPhotographedPages`].
    layered: bool,
    /// The configuration the pages are photographed in, `None` where §8.11.4.2's dictionary is not
    /// one the interpreter reads, so that no group decided anything.
    configuration: Option<OptionalContent>,
}

/// [`Stated`], or the gate's sentence for a document it may not photograph.
fn what_the_document_states(document: &Document, gate: Gate<'_>) -> Result<Stated, Because> {
    let catalog = document.catalog().unwrap_or_default();
    let structured = catalog.get("StructTreeRoot").is_some();
    if structured && !gate.beyond.untags {
        return Err(Because::NotBuiltYet(STRUCTURED));
    }
    let layered = catalog.get("OCProperties").is_some();
    if layered && !gate.beyond.layers {
        return Err(Because::NotBuiltYet(LAYERED));
    }
    Ok(Stated {
        structured,
        layered,
        // The configuration `interpret_ledgered` draws under, read as `ViewState::of` reads it.
        configuration: if layered {
            OptionalContent::read(document)
        } else {
            None
        },
    })
}

/// Every optional content group a photographed page's content names, each with the state the
/// photograph drew it in (`doc/questions/A717`): its other state is what the photograph discards.
///
/// The walk is `pdf_model`'s, bounded by its form depth and its visited set; what is kept of it is
/// bounded by [`MAX_GROUPS_NAMED`] names of at most [`MAX_GROUP_NAME_CHARS`] characters each.
fn fixed_groups(
    document: &Document,
    page: &pdf_model::Page,
    configuration: Option<&OptionalContent>,
) -> FixedGroups {
    let named = pdf_model::optional_content::groups_referenced_by_content(document, page);
    let left_out = named.len().saturating_sub(MAX_GROUPS_NAMED);
    let groups = named
        .into_iter()
        .take(MAX_GROUPS_NAMED)
        .map(|group| FixedGroup {
            name: group_name(document, group),
            drawn: configuration.and_then(|configuration| configuration.state(group)),
        })
        .collect();
    FixedGroups { groups, left_out }
}

/// A group's `/Name` as far as [`MAX_GROUP_NAME_CHARS`], or its object number where it states
/// none.
fn group_name(document: &Document, group: ObjectId) -> String {
    let stated =
        document
            .get(group)
            .as_dict()
            .and_then(|dict| match document.get_key(dict, "Name") {
                Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
                _ => None,
            });
    let Some(stated) = stated else {
        return format!("object {} {}", group.number, group.generation);
    };
    let mut name: String = stated.chars().take(MAX_GROUP_NAME_CHARS).collect();
    if stated.chars().nth(MAX_GROUP_NAME_CHARS).is_some() {
        name.push_str(" ...");
    }
    name
}

/// Takes the structure tree's leaves that name a photographed page out of the tree, writing each
/// element that lost one under its own object number, and counts them on the page
/// (`doc/questions/A707`).
fn untag(
    document: &Document,
    taken_here: &[(ObjectId, usize)],
    substitutes: &mut Substitutes,
) -> Result<(), Because> {
    let pages: BTreeSet<ObjectId> = taken_here.iter().map(|(id, _)| *id).collect();
    let untagged = super::untagged::untag(document, &pages)?;
    for (id, element) in untagged.written {
        substitutes.written.insert(id, element);
    }
    for (id, at) in taken_here {
        let leaves = untagged.leaves.get(id).copied().unwrap_or(0);
        if let Some(page) = substitutes.photographed.get_mut(*at) {
            page.structure_leaves = leaves;
        }
        if let Some(photograph) = substitutes.photographs.get_mut(id) {
            photograph.untagged = true;
        }
    }
    Ok(())
}

/// What the report and the history name of one page just photographed.
fn photographed_page(
    index: usize,
    text: &str,
    (shown, fenced): (&[usize], Option<&BTreeSet<&'static str>>),
    raster: &pdf_render::Raster,
    deferred: &[Deferred],
) -> PhotographedPage {
    let (text, text_elided) = named_text(text);
    let requirements: Vec<String> = fenced
        .into_iter()
        .flatten()
        .map(|row| (*row).to_owned())
        .collect();
    let fonts: Vec<&Deferred> = shown
        .iter()
        .filter_map(|&font| deferred.get(font))
        .collect();
    let anywhere =
        !requirements.is_empty() || !text.is_empty() || fonts.iter().any(|font| font.anywhere);
    PhotographedPage {
        page: index.saturating_add(1),
        fonts: fonts.iter().map(|font| font.requested.clone()).collect(),
        dots_per_inch: DOTS_PER_INCH,
        width: raster.width,
        height: raster.height,
        text,
        text_elided,
        requirements,
        anywhere,
        structure_leaves: 0,
        optional_content: None,
    }
}

/// The keys a photographed page's dictionary loses, whose values the output therefore does not
/// reach through it: what [`Photograph`] replaces, and the group `rewrite` removes with them.
const PHOTOGRAPHED_AWAY: [&str; 3] = ["Contents", "Resources", "Group"];

/// Which of the substituted fonts an object the output holds still names, once every
/// photographed page has lost its content and its resources.
///
/// **Reached, the way the writer reaches what it copies**: `super::super::rewrite::convert` writes
/// what the catalog and the information dictionary lead to, so a font whose every route ran
/// through a photographed page's own resources is in no object of the output, while one an
/// annotation's appearance, another page or an inherited resource dictionary names is still
/// there, its face with it. The walk is over the source's objects, each read once, with an
/// explicit stack rather than recursion, because a document's graph is the file's to shape.
fn still_held(
    document: &Document,
    photographs: &BTreeMap<ObjectId, Photograph>,
    fonts: &[super::SubstitutedFont],
) -> BTreeSet<Outcome> {
    let direct: BTreeSet<&String> = fonts
        .iter()
        .filter_map(|font| match &font.font {
            Outcome::Direct(spelled) => Some(spelled),
            _ => None,
        })
        .collect();
    let wanted: BTreeSet<&Outcome> = fonts.iter().map(|font| &font.font).collect();
    let trailer = document.trailer();
    let mut pending: Vec<Object> = ["Root", "Info"]
        .into_iter()
        .filter_map(|key| trailer.get(key).cloned())
        .collect();
    let mut seen = BTreeSet::new();
    let mut held = BTreeSet::new();
    while let Some(object) = pending.pop() {
        match object {
            Object::Reference(id) => {
                if !seen.insert(id) {
                    continue;
                }
                let reached = Outcome::Reference(id);
                if wanted.contains(&reached) {
                    held.insert(reached);
                }
                let mut value = document.get(id);
                if photographs.contains_key(&id)
                    && let Object::Dictionary(page) = &mut value
                {
                    for key in PHOTOGRAPHED_AWAY {
                        page.remove(key);
                    }
                }
                pending.push(value);
            }
            Object::Dictionary(dict) => {
                // A direct font has no identity but its value (`Outcome::Direct`), so a dictionary
                // that could be one is spelled out only while a direct font is still looked for.
                if !direct.is_empty() && dict.get("Subtype").is_some() {
                    let spelled = format!("{:?}", Object::Dictionary(dict.clone()));
                    if direct.contains(&spelled) {
                        held.insert(Outcome::Direct(spelled));
                    }
                }
                pending.extend(dict.iter().map(|(_, value)| value.clone()));
            }
            Object::Array(items) => pending.extend(items),
            Object::Stream(stream) => pending.push(Object::Dictionary(stream.dict.clone())),
            _ => {}
        }
    }
    held
}

/// The owner's gate over one page's content-only interpretation: nothing a reader can extract
/// (unless `anywhere`, the wider gate), and nothing the window left undrawn but the fonts it could
/// not draw.
fn admitted(
    interpretation: &pdf_model::content::Interpretation,
    shown: &[usize],
    deferred: &[Deferred],
    gate: Gate<'_>,
) -> Result<(), Because> {
    if !gate.anywhere && interpretation.text.chars().any(|c| !c.is_whitespace()) {
        let keyed = shown
            .iter()
            .filter_map(|&font| deferred.get(font))
            .find(|font| font.mapped);
        return Err(keyed.map_or(Because::NotBuiltYet(TEXT_IS_READABLE), |font| font.refusal));
    }
    if interpretation
        .unsupported
        .iter()
        .any(|report| leaves_the_page_undrawn(report, gate))
    {
        return Err(Because::NotBuiltYet(NOT_DRAWN_WHOLE));
    }
    Ok(())
}

/// Whether a report leaves the page one the gate does not photograph: anything but the fonts the
/// window could not draw, and under the wide gate the operators the fence names and the faults of
/// the document's own stream ([`is_a_fault_of_the_documents_own_stream`]).
fn leaves_the_page_undrawn(report: &Unsupported, gate: Gate<'_>) -> bool {
    match report {
        Unsupported::Font { .. } | Unsupported::Text { .. } => false,
        // An operator no standard defines is the fault the photograph answers, and no reader
        // draws anything for it (`super::super::fenced`).
        Unsupported::Operator { operator } => {
            !(gate.anywhere && gate.fenced.operators.contains(operator))
        }
        other => !(gate.anywhere && is_a_fault_of_the_documents_own_stream(other)),
    }
}

/// Whether a report states a fault in the page's own content stream, which the wide gate bakes in
/// as the window shows it (A694 item 3: "we will just bake something into the pdf representing the
/// state we currently show the pdf").
///
/// The faults are an operator short of its operands (§7.8.2), a name no resource dictionary defines
/// (§7.8.3), a stream damaged part-way (§7.8.2), a path painted with no current point (§8.5), a
/// matrix that cannot be inverted, and a soft mask dictionary §11.6.5.1 does not describe: the
/// document asked for nothing the window could draw there.
///
/// Everything else stays refused, because the window's picture is then missing or approximating
/// something a well-formed document asked for, and the photograph would record this reader's gap
/// as the document's own loss: an image or a shading it cannot build, a stream it cannot decode, an
/// annotation, optional content, a page box or dictionary it does not read, a drawing stopped at a
/// bound, and the constructions §9.3.8, §11.4, §11.6.2 and §11.7.4 define that this reader composites
/// otherwise (`TextKnockout`, `TransparencyGroup`, `CompositedInParts`, `Overprint`). ADR 2516.
fn is_a_fault_of_the_documents_own_stream(report: &Unsupported) -> bool {
    matches!(
        report,
        Unsupported::SoftMask { .. }
            | Unsupported::UndefinedCurrentPoint { .. }
            | Unsupported::OperandShortfall { .. }
            | Unsupported::NoninvertibleMatrix { .. }
            | Unsupported::MissingResource { .. }
            | Unsupported::DamagedContentStream { .. }
    )
}

/// A page's text as the report names it: white space collapsed to single spaces, cut at
/// [`MAX_TEXT_CHARS`] characters, and the count of those left out.
fn named_text(text: &str) -> (String, usize) {
    let mut words = text.split_whitespace();
    let mut named = String::new();
    let mut kept = 0_usize;
    let mut total = 0_usize;
    for word in words.by_ref() {
        let separator = usize::from(!named.is_empty());
        let length = word.chars().count();
        total = total.saturating_add(separator).saturating_add(length);
        if kept.saturating_add(separator).saturating_add(length) <= MAX_TEXT_CHARS {
            if separator == 1 {
                named.push(' ');
            }
            named.push_str(word);
            kept = kept.saturating_add(separator).saturating_add(length);
        }
    }
    (named, total.saturating_sub(kept))
}

/// The wider gate's refusal by count and by size, before any page is drawn: the pages the fenced
/// rows name are known from the survey, and their boxes at [`DOTS_PER_INCH`] are a floor on the
/// pixels the photographs will hold, so a document past either bound costs one survey rather than
/// the drawing of the pages up to it. [`taken`] still holds each page to what is left.
fn refuse_fenced_pages_past_the_bounds(
    every: &[Option<pdf_model::Page>],
    gate: Gate<'_>,
) -> Result<(), Because> {
    let pixels = gate
        .fenced
        .pages
        .keys()
        .filter_map(|&index| every.get(index)?.as_ref())
        .map(pixels_of_the_box)
        .fold(0_u64, u64::saturating_add);
    past_the_bounds(gate.fenced.pages.len(), pixels)
}

/// Whether `pages` photographs holding `pixels` between them are more than one conversion holds.
fn past_the_bounds(pages: usize, pixels: u64) -> Result<(), Because> {
    if pages > MAX_PHOTOGRAPHED_PAGES {
        return Err(Because::NotBuiltYet(TOO_MANY_PAGES));
    }
    if pixels > MAX_PHOTOGRAPHED_PIXELS {
        return Err(Because::NotBuiltYet(PAGE_TOO_LARGE));
    }
    Ok(())
}

/// The pixels a page's display box takes at [`DOTS_PER_INCH`], rounded up to whole pixels a side.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a float cast saturates: NaN or a negative side is 0 and a huge one u64::MAX, which the bound reads"
)]
fn pixels_of_the_box(page: &pdf_model::Page) -> u64 {
    let [x0, y0, x1, y1] = page.display_box;
    let side = |low: f32, high: f32| ((high - low).abs() * scale()).ceil() as u64;
    side(x0, x1).saturating_mul(side(y0, y1))
}

/// What [`MAX_PHOTOGRAPHED_PIXELS`] leaves after the pages already photographed.
fn pixels_left<'a>(held: impl Iterator<Item = &'a PhotographedPage>) -> u64 {
    MAX_PHOTOGRAPHED_PIXELS.saturating_sub(
        held.map(|page| u64::from(page.width).saturating_mul(u64::from(page.height)))
            .fold(0_u64, u64::saturating_add),
    )
}

/// The page's display list drawn by the oracle backend at [`DOTS_PER_INCH`], to no more than
/// `pixels_left` pixels.
fn taken(list: &pdf_render::DisplayList, pixels_left: u64) -> Result<pdf_render::Raster, Because> {
    let target = TargetSpec::for_page(
        list,
        scale(),
        crate::Budget::default().max_pixels.min(pixels_left),
    )
    .map_err(|_| Because::NotBuiltYet(PAGE_TOO_LARGE))?;
    CpuRasterizer::new()
        .rasterize(list, target)
        .map_err(|_| Because::NotBuiltYet(PAGE_NOT_DRAWN))
}

/// Device pixels to a unit of the display list's space, §8.3.2.3's 72 to the inch.
fn scale() -> f32 {
    f32::from(DOTS_PER_INCH) / 72.0
}

/// The image and the content stream painting it, written as new objects, and the resources that
/// name the image.
fn written(
    document: &Document,
    raster: &pdf_render::Raster,
    icc: ObjectId,
    placed: Transform,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<Photograph, Because> {
    let missing = Because::NotBuiltYet(NO_SPARE_OBJECT);
    let image = spare.take(document).ok_or(missing)?;
    let contents = spare.take(document).ok_or(missing)?;
    substitutes
        .written
        .insert(image, image_stream(raster, icc).ok_or(missing)?);
    substitutes
        .written
        .insert(contents, contents_stream(placed).ok_or(missing)?);
    let mut images = Dictionary::new();
    images.insert(Name::new(IMAGE), Object::Reference(image));
    let mut resources = Dictionary::new();
    resources.insert(Name::new(&b"XObject"[..]), Object::Dictionary(images));
    Ok(Photograph {
        contents,
        resources,
        untagged: false,
    })
}

/// The deferred fonts a page's content selected, by their place in the list.
fn shown_here(ledger: &Ledger, deferred: &[Deferred]) -> Vec<usize> {
    deferred
        .iter()
        .enumerate()
        .filter(|(_, font)| {
            ledger.selections().any(|(selection, _)| {
                selection.category == "Font" && selection.outcome == font.selected
            })
        })
        .map(|(index, _)| index)
        .collect()
}

/// The matrix that puts the image's unit square where the raster's pixels were, in default user
/// space.
///
/// `TargetSpec::for_page` anchors the raster at the top-left corner of the display list's page and
/// lets the fraction of a pixel it rounds up to fall off the right and the bottom edges, so the
/// image is `width / scale` by `height / scale` units of the list's space with its top edge at the
/// page's: §8.9.4's image space then reaches the list through `[w 0 0 h 0 top − h]`, and default
/// user space through the inverse of the page's own transform.
fn placement(
    page: &pdf_model::Page,
    top: f32,
    raster: &pdf_render::Raster,
) -> Result<Transform, Because> {
    let scale = scale();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a raster's extent is bounded by the pixel budget, far inside f32's exact integers"
    )]
    let (across, down) = (raster.width as f32 / scale, raster.height as f32 / scale);
    let into_list = Transform::new(across, 0.0, 0.0, down, 0.0, top - down);
    let into_user = pdf_model::content::base_transform(page)
        .invert()
        .ok_or(Because::NotBuiltYet(PAGE_NOT_DRAWN))?;
    let placed = into_list.then(into_user);
    // A quarter turn's inverse is exact in principle and a few ulps off in `f32`; the corner is
    // checked to be finite so that no NaN reaches the content stream.
    let corner = placed.apply(Point::new(1.0, 1.0));
    if corner.x.is_finite() && corner.y.is_finite() {
        Ok(placed)
    } else {
        Err(Because::NotBuiltYet(PAGE_NOT_DRAWN))
    }
}

/// The image `XObject`: the raster's colour samples, deflated, in the sRGB `ICCBased` space.
///
/// The alpha is dropped because every sample's is 255: §11.4.7 composited the page onto white
/// before the raster left the backend (`crate::render::encode` relies on the same fact).
fn image_stream(raster: &pdf_render::Raster, space: ObjectId) -> Option<Object> {
    let samples: Vec<u8> = raster
        .data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect();
    let data = flate_encode(&samples, COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    let name = |word: &[u8]| Object::Name(Name::new(word));
    dict.insert(Name::new(&b"Type"[..]), name(b"XObject"));
    dict.insert(Name::new(&b"Subtype"[..]), name(b"Image"));
    dict.insert(
        Name::new(&b"Width"[..]),
        Object::Integer(i64::from(raster.width)),
    );
    dict.insert(
        Name::new(&b"Height"[..]),
        Object::Integer(i64::from(raster.height)),
    );
    dict.insert(
        Name::new(&b"ColorSpace"[..]),
        Object::Array(vec![name(b"ICCBased"), Object::Reference(space)]),
    );
    dict.insert(Name::new(&b"BitsPerComponent"[..]), Object::Integer(8));
    dict.insert(Name::new(&b"Filter"[..]), name(b"FlateDecode"));
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// The content stream that paints the image where [`placement`] puts it, and nothing else.
fn contents_stream(placed: Transform) -> Option<Object> {
    let operands = [placed.a, placed.b, placed.c, placed.d, placed.e, placed.f]
        .map(number)
        .join(" ");
    let image = std::str::from_utf8(IMAGE).ok()?;
    let text = format!("q {operands} cm /{image} Do Q\n");
    let data = flate_encode(text.as_bytes(), COMPRESSION_LEVEL)?;
    let mut dict = Dictionary::new();
    dict.insert(
        Name::new(&b"Filter"[..]),
        Object::Name(Name::new(&b"FlateDecode"[..])),
    );
    dict.insert(
        Name::new(&b"Length"[..]),
        Object::Integer(i64::try_from(data.len()).ok()?),
    );
    Some(Object::Stream(std::sync::Arc::new(Stream {
        dict,
        data: data.into(),
        decryption_failed: false,
    })))
}

/// One operand as §7.3.3 writes a real: four places, which is under a thousandth of a pixel at
/// [`DOTS_PER_INCH`], with the trailing zeros and a negative zero taken off.
fn number(value: f32) -> String {
    let written = format!("{value:.4}");
    let trimmed = written.trim_end_matches('0').trim_end_matches('.');
    match trimmed {
        "-0" | "" => "0".to_owned(),
        other => other.to_owned(),
    }
}

/// Every photographed page, as the file's `xmpMM:History` states it.
pub(super) fn recorded(pages: &[PhotographedPage]) -> String {
    pages
        .iter()
        .map(|page| {
            let untagged = format!(
                "{}{}",
                leaves_named(page.structure_leaves),
                groups_fixed(page.optional_content.as_ref())
            );
            if !page.anywhere {
                return format!(
                    "page {} was replaced by a photograph of itself at {} dpi, the text of {} \
                     that no reader can name left blank as this viewer drew it, losing {}{untagged}",
                    page.page,
                    page.dots_per_inch,
                    page.fonts.join(", "),
                    page.loss_words(),
                );
            }
            let mut reasons = Vec::new();
            if !page.fonts.is_empty() {
                reasons.push(format!("for the font {}", page.fonts.join(", ")));
            }
            if !page.requirements.is_empty() {
                reasons.push(format!(
                    "for the content stream's {}",
                    page.requirements.join(", ")
                ));
            }
            reasons.push(if page.text.is_empty() {
                "the page showing no text".to_owned()
            } else {
                format!(
                    "and with it the page's text, which was \"{}\"{}",
                    page.text,
                    elided(page.text_elided),
                )
            });
            format!(
                "page {} was replaced by a photograph of itself at {} dpi, losing {}, {}{untagged}",
                page.page,
                page.dots_per_inch,
                page.loss_words(),
                reasons.join(", "),
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// What follows a page's history entry when the structure tree lost leaves that named it.
pub(in super::super) fn leaves_named(leaves: usize) -> String {
    if leaves == 0 {
        String::new()
    } else {
        format!(", and {leaves} leaves of the structure tree that named the page")
    }
}

/// What follows a page's history entry and its report line when the page fixed optional content.
fn groups_fixed(groups: Option<&FixedGroups>) -> String {
    groups.map_or_else(String::new, |groups| format!(", and {}", groups.named()))
}

/// What follows a page's named text when characters of it are not repeated.
pub(in super::super) fn elided(left_out: usize) -> String {
    if left_out == 0 {
        String::new()
    } else {
        format!(" and {left_out} more characters")
    }
}

/// The wider gate was not authorised and no deferred font is held.
const NOT_OFFERED: &str = "a page of this document holds a fault of its content stream that the \
     target refuses, and the target is one at which a page photographed with no text layer is not \
     offered";

/// More pages than a conversion photographs.
const TOO_MANY_PAGES: &str = "more pages of this document would be photographed than one \
     conversion photographs, each a raster held until the file is written";

/// The gate: a page whose text a reader can already extract.
const TEXT_IS_READABLE: &str = "this document renders an Adobe-Identity composite font it does not \
     embed, whose codes no ToUnicode CMap names, on a page that also shows text a reader can \
     extract; a photograph of the page would turn that text into pixels, so the page is not \
     photographed. Name the producer's font program with --font to keep the page as text";

/// The gate: a page the window does not draw whole.
const NOT_DRAWN_WHOLE: &str = "a page that would be photographed has content this reader does not \
     draw whole; a photograph would fix what the window could not draw into the file, so the page \
     is not photographed";

/// The gate: a structure tree a photograph would leave pointing at nothing, where the profile did
/// not authorise taking the page's leaves out of it.
const STRUCTURED: &str = "this document has a page that would be photographed, and states a \
     structure tree (ISO 32000-2 section 14.7) whose content items a photographed page would \
     leave pointing at marked content no longer on it; authorise tagging-of-photographed-pages \
     to take that page's leaves out of the tree";

/// The gate: optional content a photograph of one configuration would discard.
const LAYERED: &str = "this document has a page that would be photographed, and states optional \
     content (ISO 32000-2 section 8.11), whose other configurations a photograph of the one this \
     viewer shows would discard; authorise optional-content-of-photographed-pages to photograph \
     the page in that configuration and name the groups it fixes";

/// The ledger stopped before the page's lookups were all kept.
const LEDGER_TRUNCATED: &str = "a page of this document names so many resources that the record of \
     which fonts it shows stopped short, so whether it shows a font that is to be photographed \
     around is not known and no page is photographed";

/// The page has no object to give new content to.
const PAGE_NOT_AN_OBJECT: &str = "a page that would be photographed is one this reader recovered \
     rather than one the page tree states as an object, so there is no page object to give the \
     photograph to";

/// The picture would pass the pixel budget.
const PAGE_TOO_LARGE: &str = "a page that would be photographed is too large to be photographed at \
     the resolution a page is photographed at within the pixel budget a rendered page is held to, \
     or the pages photographed together would pass the pixels one conversion holds";

/// The backend refused the page, or its transform has no inverse.
const PAGE_NOT_DRAWN: &str = "a page that would be photographed could not be drawn to be \
     photographed: the oracle backend refused it, or the page's own \
     transform from user space has no inverse";

/// No object number was free.
const NO_SPARE_OBJECT: &str = "no unused object number could be found for the photograph of a page \
     this conversion would have written";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_operand_is_written_without_its_trailing_zeros() {
        assert_eq!(number(612.0), "612");
        assert_eq!(number(0.25), "0.25");
        assert_eq!(number(-0.000_01), "0");
        assert_eq!(number(-1.5), "-1.5");
    }

    fn page_of(width: u32, height: u32) -> PhotographedPage {
        PhotographedPage {
            page: 1,
            fonts: Vec::new(),
            dots_per_inch: DOTS_PER_INCH,
            width,
            height,
            text: String::new(),
            text_elided: 0,
            requirements: Vec::new(),
            anywhere: true,
            structure_leaves: 0,
            optional_content: None,
        }
    }

    #[test]
    fn the_pages_held_take_their_pixels_from_the_budget_and_it_never_goes_below_nothing() {
        let held = [page_of(16_384, 8_192), page_of(10, 10)];
        assert_eq!(
            pixels_left(held.iter()),
            MAX_PHOTOGRAPHED_PIXELS - 16_384 * 8_192 - 100
        );
        let past = [page_of(u32::MAX, u32::MAX), page_of(u32::MAX, u32::MAX)];
        assert_eq!(pixels_left(past.iter()), 0);
    }

    #[test]
    fn a_construction_this_reader_composites_otherwise_leaves_the_page_refused() {
        let fenced = super::super::super::fenced::Fenced::default();
        let wide = Gate::new(Target::Two(Level::B), (true, false, false), &fenced);
        // §9.3.8, §11.4, §11.6.2 and §11.7.4 define these; a photograph would record this reader's
        // shortfall as the document's loss.
        for gap in [
            Unsupported::TextKnockout { glyphs: 2 },
            Unsupported::TransparencyGroup {
                detail: String::new(),
            },
            Unsupported::CompositedInParts { detail: "a path" },
            Unsupported::Overprint {
                detail: String::new(),
            },
        ] {
            assert!(leaves_the_page_undrawn(&gap, wide), "{gap:?}");
        }
        // A fault of the document's own stream is baked in under the wide gate and only there.
        let fault = Unsupported::OperandShortfall {
            operator: String::from("re"),
            given: 1,
            takes: 4,
        };
        assert!(!leaves_the_page_undrawn(&fault, wide));
        let narrow = Gate::new(Target::Two(Level::B), (false, false, false), &fenced);
        assert!(leaves_the_page_undrawn(&fault, narrow));
    }

    #[test]
    fn fenced_pages_past_either_bound_are_refused_before_any_is_drawn() {
        assert!(past_the_bounds(MAX_PHOTOGRAPHED_PAGES, MAX_PHOTOGRAPHED_PIXELS).is_ok());
        assert!(matches!(
            past_the_bounds(MAX_PHOTOGRAPHED_PAGES + 1, 0),
            Err(Because::NotBuiltYet(TOO_MANY_PAGES))
        ));
        assert!(matches!(
            past_the_bounds(1, MAX_PHOTOGRAPHED_PIXELS + 1),
            Err(Because::NotBuiltYet(PAGE_TOO_LARGE))
        ));
    }

    #[test]
    fn a_page_larger_than_the_pixels_left_is_not_drawn() {
        let list = pdf_render::DisplayList::new(pdf_render::Size::new(200.0, 200.0));
        let fits = 834_u64 * 834;
        assert!(taken(&list, fits).is_ok());
        assert!(matches!(
            taken(&list, fits - 1),
            Err(Because::NotBuiltYet(PAGE_TOO_LARGE))
        ));
    }
}
