//! A standard font stated by name alone, given the program, descriptor and widths it is drawn by.
//!
//! ISO 32000-2 §9.6.2.1's Table 109 lets one of §9.6.2.2's fourteen leave out `/FirstChar`,
//! `/LastChar`, `/Widths` and `/FontDescriptor` — "(Required; optional in PDF 1.0-1.7 for the
//! standard 14 fonts)" — and ISO 19005-2 section 6.2.11.4.1 (NOTE 5) and ISO 19005-4 section
//! 6.2.10.4.1 take no exemption from embedding for them. So a file that names `/Helvetica` and
//! nothing more owes a program, and a program is embedded *into a descriptor*, which the file
//! never wrote. This module writes the three things such a font lacks, and every number in them
//! is either the viewer's own or the program's (`doc/adr/2146`).
//!
//! # Why the result is the page the viewer already draws
//!
//! The viewer draws such a font today from the face [`pdf_font::standard::shipped_face`] chooses —
//! the compiled-in, metric-compatible substitute ADR 0133 brought in and ADR 1441 holds to the
//! fourteen — at the advances [`pdf_font::LoadedFont::advance`] reads from the standard's
//! published metrics. The conversion embeds **that face, unmodified**, and states **those
//! advances** as `/Widths`. A reader then draws the same outlines at the same positions, and the
//! only thing that changed is that the page no longer depends on which reader opened it — which
//! is what ISO 19005-2 section 6.2.11.1 says the font requirements are for.
//!
//! **That is proved, not trusted.** [`proved`] writes the converted font into a document of its
//! own, loads it through the same reader the viewer uses, and compares it with the source font
//! code by code: the same glyph outline for every code the page shows, the same advance for every
//! one of the 256, and no horizontal scale on either side. A font that would draw differently —
//! because the viewer picked another face for its characters (ADR 0270), drew the face condensed
//! to the file's own widths, or would select another glyph once embedded — is refused by name
//! rather than converted into a page that looks different.
//!
//! # The two shapes the fourteen take
//!
//! Ten of the shipped faces are bare Compact Font Format programs, which §9.9's Table 124 admits
//! under the `Type1` dictionary the file wrote, as `/FontFile3` with `/Subtype /Type1C`. The four
//! Helvetica faces are Liberation Sans, a TrueType-flavoured sfnt, and Table 124 opens no key to a
//! `glyf` program under a `Type1` dictionary at all — so the dictionary is restated as the
//! `TrueType` font §9.6.3 describes, with `/FontFile2`. §9.6.3 lets a TrueType dictionary hold the
//! same entries as a Type 1 one, so nothing else in it moves; what it adds is §9.6.5.4's route
//! from a code to a glyph, which is a glyph name, the Adobe Glyph List and the program's
//! (3,1) `cmap` — the very route the viewer already takes into this face as a substitute. ISO
//! 19005-2 section 6.2.11.6 then wants a non-symbolic TrueType font's encoding to be
//! `MacRomanEncoding` or `WinAnsiEncoding` or a dictionary built on one: an encoding already of
//! that kind is carried untouched, and one that is not is restated as a `WinAnsiEncoding`
//! dictionary whose `/Differences` give every code the glyph name the original gave it, so the
//! code-to-name table — which is what an encoding *is* (§9.6.5.1) — does not change.

use std::collections::BTreeMap;

use pdf_archive::survey::SelectedFont;
use pdf_font::encoding::BaseEncoding;
use pdf_font::standard::ShippedFace;
use pdf_font::substitute::{Family, Format};
use pdf_font::{Code, LoadedFont};
use pdf_syntax::object::{Dictionary, Name, Object, ObjectId};
use pdf_syntax::serialize::{Assembly, Form, Options, serialize};
use pdf_syntax::{Document, Limits, Version};
use skrifa::MetadataProvider as _;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::FontRead as _;
use skrifa::raw::ps::cff::CffFontRef;
use skrifa::raw::ps::cff::dict::{Entry, entries};

use super::super::decision::{Because, Loss, Losses};
use super::super::prepare::Spare;
use super::{
    MetricRoute, SubstitutedFont, Substitutes, Table124, base_font, key_for, name_of, no_row,
    program_stream, subtype_dictionary,
};

/// What the conversion wrote for one font that stated no descriptor, for the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Described {
    /// A descriptor and `/Widths` beside a Compact Font Format program, the dictionary still the
    /// `Type1` one the file wrote.
    Type1,
    /// The same, with the dictionary restated as a `TrueType` one because the face is a `glyf`
    /// program, which §9.9's Table 124 admits under no `Type1` dictionary.
    TrueType,
    /// [`Self::Type1`], with the encoding restated as `/Differences` naming, for every code, the
    /// glyph it named before — a symbolic face, whose program's built-in encoding is not the
    /// Annex D table the page was drawn by.
    Type1EncodingRestated,
    /// [`Self::TrueType`], with the encoding restated as a `WinAnsiEncoding` dictionary that gives
    /// every code the glyph name the original gave it.
    TrueTypeEncodingRestated,
    /// The file's own descriptor received the face, and its `TrueType` dictionary is restated as
    /// the `Type1` one a Compact Font Format program goes under (`doc/adr/2147`).
    RestatedAsType1,
    /// The file's own descriptor received the face, and its `Type1` dictionary is restated as the
    /// `TrueType` one a `glyf` program goes under, the encoding restated where it must be
    /// (`doc/adr/2147`).
    RestatedAsTrueType,
}

impl Described {
    /// Whether the font stated no descriptor, so that this conversion may have written its
    /// `/FirstChar`, `/LastChar` and `/Widths` beside the one it wrote — the four Table 109 makes
    /// present or absent together.
    #[must_use]
    pub const fn wrote_widths(self) -> bool {
        matches!(
            self,
            Self::Type1
                | Self::Type1EncodingRestated
                | Self::TrueType
                | Self::TrueTypeEncodingRestated
        )
    }

    /// A stable word for the report's machine-readable form.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Type1 => "descriptor",
            Self::Type1EncodingRestated => "descriptor-encoding",
            Self::TrueType => "descriptor-truetype",
            Self::TrueTypeEncodingRestated => "descriptor-truetype-encoding",
            Self::RestatedAsType1 => "restated-type1",
            Self::RestatedAsTrueType => "restated-truetype",
        }
    }

    /// What was written, in one clause for a person.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Type1 => {
                "the font stated no descriptor, so one was written from the face's own program, \
                 with the widths the page is laid out by"
            }
            Self::Type1EncodingRestated => {
                "the font stated no descriptor, so one was written from the face's own program, \
                 with the widths the page is laid out by, and its encoding is restated with every \
                 code naming the glyph it named before"
            }
            Self::TrueType => {
                "the font stated no descriptor, so one was written from the face's own program, \
                 with the widths the page is laid out by, and the dictionary is a TrueType one \
                 because the face is"
            }
            Self::TrueTypeEncodingRestated => {
                "the font stated no descriptor, so one was written from the face's own program, \
                 with the widths the page is laid out by; the dictionary is a TrueType one \
                 because the face is, and its encoding is restated on WinAnsiEncoding with every \
                 code naming the glyph it named before"
            }
            Self::RestatedAsType1 => {
                "the dictionary is restated as a Type1 one, which is the kind of program the face \
                 is and the only kind ISO 32000-2 \u{a7}9.9's Table 124 admits that program under"
            }
            Self::RestatedAsTrueType => {
                "the dictionary is restated as a TrueType one, which is the kind of program the \
                 face is and the only kind ISO 32000-2 \u{a7}9.9's Table 124 admits that program \
                 under, its encoding restated on WinAnsiEncoding where it was not already one ISO \
                 19005-2 section 6.2.11.6 admits"
            }
        }
    }
}

/// A dictionary of one kind given a face of the other, restated as the face's kind.
pub(super) struct Restatement {
    /// The key and stream subtype the face goes under once the dictionary is the face's kind.
    pub(super) row: Table124,
    /// The entries the font dictionary gains: its new `/Subtype`, and an encoding where one is owed.
    pub(super) entries: Dictionary,
    /// The source font as the viewer draws it, which [`proved`] holds the restatement to.
    pub(super) source: LoadedFont,
    /// Which face the viewer draws it from, which decides what [`proved`] compares.
    pub(super) drawn: Drawn,
    /// The shipped face being embedded, whose glyphs [`proved`] reads where `drawn` is a
    /// machine's face.
    face: ShippedFace,
    /// Which of the two restatements it is, for the report.
    pub(super) described: Described,
    /// The entries the file's own descriptor gains in the copy the face is embedded into: the
    /// `/Flags` that make the restated `TrueType` font the non-symbolic one its encoding is, or
    /// nothing (`doc/adr/2278`).
    pub(super) descriptor: Dictionary,
}

/// The restatement a font whose descriptor states no program needs when Table 124 admits its
/// shipped face under no key of the dictionary's own `/Subtype` (`doc/adr/2147`).
///
/// `None` where the pairing is not one a restatement answers — a face of a kind other than the
/// two, or a dictionary other than a simple one — and the caller refuses with Table 124's own
/// sentence. The two answered: a `TrueType` dictionary given one of the ten Compact Font Format
/// faces, which becomes `Type1` with `/FontFile3 /Type1C`, and a `Type1` dictionary given a
/// Liberation face, which becomes `TrueType` with `/FontFile2` and, where its encoding is not one
/// ISO 19005-2 section 6.2.11.6 admits, the restated one [`truetype_encoding`] writes. §9.6.3 lets
/// the two dictionaries hold the same entries, so nothing else in the dictionary moves.
pub(super) fn restated_kind(
    document: &Document,
    used: &SelectedFont,
    subtype: &str,
    face: &ShippedFace,
) -> Result<Option<Restatement>, Because> {
    let other = match (subtype, face.format) {
        ("TrueType", Format::BareCff) => "Type1",
        ("Type1", Format::Sfnt) => "TrueType",
        _ => return Ok(None),
    };
    let Some(row) = key_for(other, face.program) else {
        return Ok(None);
    };
    let source = LoadedFont::load(document, &used.dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(super::FONT_NOT_READ))?;
    let drawn = drawn_from(&source, used, face)?;
    let mut entries = Dictionary::new();
    entries.insert(
        Name::new(&b"Subtype"[..]),
        Object::Name(Name::new(other.as_bytes())),
    );
    let mut descriptor = Dictionary::new();
    if other == "TrueType" {
        if let Some(restated) = truetype_encoding(document, &used.dict, &source) {
            entries.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(restated));
        }
        if let Some(flags) = nonsymbolic_flags(document, &used.dict) {
            descriptor.insert(Name::new(&b"Flags"[..]), Object::Integer(flags));
        }
    } else if other == "Type1" && document.get_key(&used.dict, "Encoding").is_null() {
        // With no Encoding, §9.6.5.1 uses the embedded program's built-in table. The source
        // TrueType substitute may instead use Annex D's symbolic names, so carrying the
        // absence across would select different glyphs. State the source's names first;
        // the unchanged outline proof still decides whether they reach the same shapes.
        entries.insert(
            Name::new(&b"Encoding"[..]),
            Object::Dictionary(restated_encoding(&source, None, &BTreeMap::new())),
        );
    }
    Ok(Some(Restatement {
        row,
        entries,
        source,
        drawn,
        face: *face,
        described: if other == "Type1" {
            Described::RestatedAsType1
        } else {
            Described::RestatedAsTrueType
        },
        descriptor,
    }))
}

/// The `/Flags` a `Type1` dictionary's own descriptor states once it is restated as `TrueType`
/// over a Liberation face, where they are not already the non-symbolic ones.
///
/// **The restated font is non-symbolic, and the descriptor is made to say so.** ISO 32000-2
/// §9.6.5.4 builds the code-to-name table a TrueType font reaches its glyphs through only where the
/// encoding is one of the two named ones or the descriptor's Nonsymbolic flag is set; for an
/// encoding *dictionary* beside a descriptor flagged Symbolic it builds none, and ISO 19005-2
/// section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 forbid a symbolic TrueType font an `/Encoding`
/// at all. The conversion states a `WinAnsiEncoding` one, the guideline §9.6.5.4 gives a
/// non-symbolic font, over a face whose every glyph that encoding reaches is in Annex D.2's Latin
/// set; Table 121 then makes the Nonsymbolic flag the true one and requires the two flags never
/// both set nor both clear. So Symbolic (bit 3) is cleared and Nonsymbolic (bit 6) set, and every
/// other bit is the file's. The descriptor this conversion writes for a font that stated none takes
/// the same reading ([`Measured::descriptor`]), and [`proved`] holds the font to the source with
/// these flags in place (`doc/adr/2278`).
fn nonsymbolic_flags(document: &Document, font: &Dictionary) -> Option<i64> {
    const SYMBOLIC: i64 = 1 << 2;
    const NONSYMBOLIC: i64 = 1 << 5;
    let descriptor = document.get_key(font, "FontDescriptor");
    let stated = descriptor
        .as_dict()
        .and_then(|descriptor| document.get_key(descriptor, "Flags").as_integer())
        .unwrap_or(0);
    let flags = (stated & !SYMBOLIC) | NONSYMBOLIC;
    (flags != stated).then_some(flags)
}

/// Holds a restated dictionary, beside its own descriptor and the program about to be embedded,
/// to the source font as the viewer draws it — [`proved`]'s check, over the file's descriptor.
pub(super) fn restatement_proved(
    document: &Document,
    used: &SelectedFont,
    restatement: &Restatement,
    program: &[u8],
) -> Result<(), Because> {
    let descriptor = document.get_key(&used.dict, "FontDescriptor");
    let Object::Dictionary(mut descriptor) = resolved(
        document,
        &Object::Dictionary(descriptor.as_dict().cloned().unwrap_or_default()),
        4,
    ) else {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    };
    // What the rewriter takes out of the descriptor it embeds into, taken out here too, and what
    // it writes into it, written here too.
    for key in ["FontFile", "FontFile2", "FontFile3", "CharSet", "CIDSet"] {
        descriptor.remove(key);
    }
    for (key, value) in restatement.descriptor.iter() {
        descriptor.insert(key.clone(), value.clone());
    }
    proved(
        document,
        used,
        HeldTo {
            source: &restatement.source,
            drawn: restatement.drawn,
            face: &restatement.face,
        },
        &restatement.entries,
        &descriptor,
        restatement.row,
        program,
    )
}

/// Which face the viewer draws a font from, and so what the converted font is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Drawn {
    /// The face being embedded: the converted font must draw every glyph the page shows with the
    /// source's own outline.
    FromTheShippedFace,
    /// A face of this machine's, which ADR 0270 draws instead where the document's encoding names
    /// characters the shipped face lacks. The converted font is held to the shipped face's own
    /// glyph for every code the page shows — the page as a reader without that machine's face
    /// draws it — and what changes is the shapes, which the loss `machine-face` names
    /// (`doc/adr/2162`).
    FromAMachineFace,
}

/// What [`proved`] holds the converted font to: the source as the viewer draws it, and where that
/// is from a machine's face, the shipped face's own glyphs.
#[derive(Clone, Copy)]
struct HeldTo<'a> {
    source: &'a LoadedFont,
    drawn: Drawn,
    face: &'a ShippedFace,
}

/// Which face the viewer draws `source` from, refused where it is a machine's and the shipped face
/// has no glyph for a code the page shows.
///
/// ADR 0270 widens to a machine's face over a code the document *declares*, which may be one no
/// page shows; the shipped face then draws every mark the page makes, in its own shapes. A code
/// the page does show and the shipped face cannot draw is a mark embedding it would lose, which no
/// answer here offers.
fn drawn_from(
    source: &LoadedFont,
    used: &SelectedFont,
    face: &ShippedFace,
) -> Result<Drawn, Because> {
    match source.substitute_program() {
        Some(drawn) if drawn == face.program => return Ok(Drawn::FromTheShippedFace),
        Some(_) => {}
        None => return Err(Because::NotBuiltYet(ANOTHER_FACE_IS_DRAWN)),
    }
    for text in used.shown.keys() {
        for code in source.decode(text) {
            // A code the encoding sends to `.notdef` draws nothing in either face; whether it is
            // given an empty glyph is [`super::empty`]'s question, asked where the program is.
            if shipped_glyph(face, code).is_none() && !super::empty::reaches_notdef(source, code) {
                return Err(Because::NotBuiltYet(ANOTHER_FACE_DRAWS_A_SHOWN_CODE));
            }
        }
    }
    Ok(Drawn::FromAMachineFace)
}

/// The glyph a code selects in the shipped face, for a code a simple font can show.
fn shipped_glyph(face: &ShippedFace, code: Code) -> Option<u16> {
    u8::try_from(code.value())
        .ok()
        .and_then(|byte| face.glyph(byte))
}

/// What a font dictionary is told once its descriptor exists.
struct Restated<'a> {
    /// The descriptor the dictionary now refers to.
    descriptor_at: ObjectId,
    /// The subtype the shipped face's program goes under.
    subtype: &'static str,
    /// The subtype the file stated.
    stated_subtype: &'a str,
    /// The name the descriptor was written with.
    font_name: &'a [u8],
    /// Whether the file named the font itself or left the name to this viewer.
    layout: &'a Layout,
}

/// The entries a font dictionary gains or restates for its new descriptor.
fn restated_entries(
    document: &Document,
    used: &SelectedFont,
    restated: &Restated<'_>,
    encoding: Option<Dictionary>,
    source: &LoadedFont,
) -> Dictionary {
    let Restated {
        descriptor_at,
        subtype,
        stated_subtype,
        font_name,
        layout,
    } = restated;
    let mut entries = Dictionary::new();
    entries.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Reference(*descriptor_at),
    );
    if subtype != stated_subtype {
        entries.insert(
            Name::new(&b"Subtype"[..]),
            Object::Name(Name::new(subtype.as_bytes())),
        );
    }
    // Table 120's `/FontName` row: the descriptor's name "shall be the same as the value of
    // BaseFont in the font or CIDFont dictionary that refers to this font descriptor". A font that
    // stated no `/BaseFont` is given the name its descriptor was written with (`doc/adr/2504`).
    if layout.supplies_base_font() {
        entries.insert(
            Name::new(&b"BaseFont"[..]),
            Object::Name(Name::new(*font_name)),
        );
    }
    if let Some(restated) = encoding {
        entries.insert(Name::new(&b"Encoding"[..]), Object::Dictionary(restated));
    }
    // **The widths the page is laid out by, stated, because the descriptor obliges them.** Table
    // 109's `/FontDescriptor` row: "For the standard 14 fonts, the entries FirstChar, LastChar,
    // Widths , and FontDescriptor shall either all be present or all be absent." A file that wrote
    // `/Widths` keeps them; one that relied on the permission is given the numbers the viewer laid
    // it out with, which is the one `/Widths` that moves nothing (`doc/adr/2146`).
    if document.get_key(&used.dict, "Widths").is_null() {
        state_laid_out_widths(&mut entries, source);
    }
    entries
}

/// Writes `/FirstChar`, `/LastChar` and `/Widths` into `entries` from the advances `source` is laid
/// out by.
///
/// For a font that has a descriptor and no `/Widths`, which Table 109 forbids of the standard
/// fourteen in the sentence quoted above and which §9.6.2.1 leaves nobody else: the numbers stated
/// are the ones the viewer already draws with, so no glyph moves (`doc/adr/2535`).
pub(super) fn state_laid_out_widths(entries: &mut Dictionary, source: &LoadedFont) {
    let (first, last, widths) = laid_out_widths(source);
    entries.insert(Name::new(&b"FirstChar"[..]), Object::Integer(first.into()));
    entries.insert(Name::new(&b"LastChar"[..]), Object::Integer(last.into()));
    entries.insert(Name::new(&b"Widths"[..]), Object::Array(widths));
}

/// One font's descriptor, program and widths, prepared and proved.
///
/// The program stream is shared by every font of one document that embeds the same face, which
/// is why [`describe`] is handed the map of those already written.
pub(super) fn describe(
    document: &Document,
    used: &SelectedFont,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
    target: pdf_archive::Target,
) -> Result<(), Because> {
    let requested = base_font(document, &used.dict);
    let stated_subtype = name_of(document, &used.dict, "Subtype").unwrap_or_default();
    let layout = layout_of(document, used, &stated_subtype, &requested)?;
    let face = pdf_font::standard::shipped_face(document, &used.dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(super::NO_SHIPPED_FACE_COVERS_IT))?;
    let source = LoadedFont::load(document, &used.dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(super::FONT_NOT_READ))?;
    let drawn = drawn_from(&source, used, &face)?;
    let (program, empty) = with_empty_glyphs(&face, &source, used, target)?;
    let symbolic = matches!(face.request.family, Family::Symbol | Family::ZapfDingbats);
    // A dictionary that is not the `Type1` one a Compact Font Format program goes under, given such
    // a face with no encoding of its own, would lose the names its source drew by, because a
    // `Type1` font without one takes the program's built-in table (ISO 32000-2 §9.6.5.1).
    let loses_names =
        stated_subtype != "Type1" && document.get_key(&used.dict, "Encoding").is_null();
    let (subtype, encoding) = match face.format {
        Format::BareCff => (
            "Type1",
            (symbolic || loses_names || !empty.is_empty())
                .then(|| restated_encoding(&source, None, &empty)),
        ),
        Format::Sfnt => ("TrueType", truetype_encoding(document, &used.dict, &source)),
        // `Format` is non-exhaustive; a reader added to it is not a program Table 124 has been
        // read against here, and is refused by the table's own sentence.
        _ => return Err(Because::NotBuiltYet(super::NO_SHIPPED_FORMAT)),
    };
    let row = key_for(subtype, face.program)
        .ok_or_else(|| Because::NotBuiltYet(no_row(subtype, face.format, face.program)))?;
    let measured = Measured::of(&face).ok_or(Because::NotBuiltYet(PROGRAM_NOT_MEASURED))?;

    let program_at = if empty.is_empty() {
        shared_program(document, &face, row, spare, substitutes)?
    } else {
        own_program(document, &face, &program, row, spare, substitutes)?
    };
    let descriptor_at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let font_name = layout.font_name(&requested, &face);
    let descriptor = measured.descriptor(&font_name, &face, row, program_at);

    let restated_encoding = encoding.is_some();
    let entries = restated_entries(
        document,
        used,
        &Restated {
            descriptor_at,
            subtype,
            stated_subtype: &stated_subtype,
            font_name: &font_name,
            layout: &layout,
        },
        encoding,
        &source,
    );
    advances_are_the_faces(used, drawn, &source, &face)?;
    proved(
        document,
        used,
        HeldTo {
            source: &source,
            drawn,
            face: &face,
        },
        &entries,
        &descriptor,
        row,
        &program,
    )?;

    substitutes
        .written
        .insert(descriptor_at, Object::Dictionary(descriptor));
    super::gains(document, used, entries, spare, substitutes)?;
    let (mut losses, fitting) = losses_of(drawn, &source);
    if layout.is_this_viewers() {
        losses = losses.with(Loss::LayoutFromThisViewer);
    }
    substitutes.done.push(SubstitutedFont {
        resource: used.name.clone(),
        requested,
        face: face.describe().to_owned(),
        authority: super::FaceAuthority::Shipped,
        route: MetricRoute::FaceMetrics,
        described: Some(match (subtype, restated_encoding) {
            ("Type1", false) => Described::Type1,
            ("Type1", true) => Described::Type1EncodingRestated,
            (_, false) => Described::TrueType,
            (_, true) => Described::TrueTypeEncodingRestated,
        }),
        losses,
        fitting,
        empty_glyphs: empty.keys().copied().collect(),
        font: super::selected(used),
    });
    Ok(())
}

/// Who states the layout a font that stated no descriptor is laid out by.
///
/// ISO 32000-2 §9.6.2.1's Table 109 lets only the fourteen of §9.6.2.2 leave out `/Widths` and
/// `/FontDescriptor`, and Annex D publishes their metrics. Any other simple font that leaves both
/// out has had its layout stated by nothing in the file or the standard, and the viewer's choice
/// of the metrics its name resembles is the only layout the page was shown with
/// (`doc/questions/A687`, `doc/adr/2504`).
enum Layout {
    /// One of the fourteen, whose metrics the standard publishes.
    Standard,
    /// Any other simple font, laid out by this viewer.
    ThisViewers {
        /// The bytes of the `/BaseFont` the file states, or `None` where it states none.
        base_font: Option<Vec<u8>>,
    },
}

impl Layout {
    /// Whether the layout is this viewer's rather than the standard's.
    const fn is_this_viewers(&self) -> bool {
        matches!(self, Self::ThisViewers { .. })
    }

    /// Whether the font dictionary is given the `/BaseFont` its descriptor is named by.
    const fn supplies_base_font(&self) -> bool {
        matches!(self, Self::ThisViewers { base_font: None })
    }

    /// The `/FontName` of the descriptor, which Table 120 makes the dictionary's `/BaseFont`.
    ///
    /// The bytes the file's own `/BaseFont` holds, so that the two names are equal whatever bytes
    /// they are. A font that states none is given the PostScript name of the program the
    /// descriptor carries (`doc/adr/2505`), which the standard defines for both kinds.
    /// ISO 32000-2 §9.6.2.1, Table 109's `BaseFont` row, for a `Type1` font:
    ///
    /// > For Type 1 fonts, this is always the value of the FontName entry in the font program
    ///
    /// ISO 32000-2 §9.6.3, for a `TrueType` one:
    ///
    /// > If the TrueType or OpenType font program's "name" table contains a PostScript language name, it should be used.
    ///
    /// Only a program that states no usable name is given a name of this converter's choosing: the
    /// shipped face's description with its spaces hyphenated, because a PostScript name has no
    /// space.
    fn font_name(&self, requested: &str, face: &ShippedFace) -> Vec<u8> {
        match self {
            Self::Standard => requested.as_bytes().to_vec(),
            Self::ThisViewers {
                base_font: Some(stated),
            } => stated.clone(),
            Self::ThisViewers { base_font: None } => postscript_name(face).unwrap_or_else(|| {
                face.describe()
                    .bytes()
                    .map(|byte| if byte == b' ' { b'-' } else { byte })
                    .collect()
            }),
        }
    }
}

/// The PostScript name a shipped face's own program states: the `name` table's name ID 6 of an
/// sfnt program (ISO 32000-2 §9.6.3), the font name of a bare Compact Font Format program
/// (Table 109's `BaseFont` row). `None` where the program states none that a PDF name can carry
/// without a space or a control byte.
pub(super) fn postscript_name(face: &ShippedFace) -> Option<Vec<u8>> {
    let stated: Vec<u8> = match face.format {
        Format::Sfnt => {
            let font = skrifa::FontRef::new(face.program).ok()?;
            let name = font
                .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
                .english_or_first()?;
            name.chars().collect::<String>().into_bytes()
        }
        Format::BareCff => CffFontRef::new_cff(face.program, 0, None)
            .ok()?
            .metadata()?
            .name()?
            .as_bytes()
            .to_vec(),
        _ => return None,
    };
    (!stated.is_empty() && stated.iter().all(u8::is_ascii_graphic)).then_some(stated)
}

/// Which layout a font stating no descriptor is given, or why it is refused.
///
/// A `Type1` font naming one of the fourteen keeps the standard's layout. A `Type1`, `MMType1` or
/// `TrueType` font naming anything else, with no `/Widths` either, is the viewer's layout under
/// the loss `layout-from-this-viewer`. One that states `/Widths` but no descriptor, a `/BaseFont`
/// that is not a name, or a subtype of another kind, is refused: its layout is the file's, or its
/// name is no name, and neither is this route's to rewrite.
fn layout_of(
    document: &Document,
    used: &SelectedFont,
    subtype: &str,
    requested: &str,
) -> Result<Layout, Because> {
    if subtype == "Type1" && pdf_font::standard::is_standard_name(requested) {
        return Ok(Layout::Standard);
    }
    if !matches!(subtype, "Type1" | "MMType1" | "TrueType") {
        return Err(Because::NotBuiltYet(NOT_A_SIMPLE_FONT));
    }
    if !document.get_key(&used.dict, "Widths").is_null() {
        return Err(Because::NotBuiltYet(WIDTHS_WITHOUT_A_DESCRIPTOR));
    }
    let base_font = document.get_key(&used.dict, "BaseFont");
    if base_font.is_null() {
        return Ok(Layout::ThisViewers { base_font: None });
    }
    base_font
        .as_name()
        .map(|name| Layout::ThisViewers {
            base_font: Some(name.as_bytes().to_vec()),
        })
        .ok_or(Because::NotBuiltYet(BASE_FONT_NOT_A_NAME))
}

/// What embedding the shipped face loses for a font the viewer draws `drawn`, and the horizontal
/// scale it removes where it removes one.
///
/// A machine's face is the loss `machine-face` whatever it was fitted at, because the shapes the
/// output draws are another face's altogether (`doc/adr/2162`). The shipped face, fitted to the
/// file's widths, is the loss `substitute-stretch`: the output draws the same glyph of the same
/// face at the same position, at the face's own width (`doc/questions/A566`, `doc/adr/2262`).
pub(super) fn losses_of(drawn: Drawn, source: &LoadedFont) -> (Losses, Option<f32>) {
    match drawn {
        Drawn::FromAMachineFace => (Losses::one(Loss::MachineFace), None),
        Drawn::FromTheShippedFace if fitted(source) => {
            (Losses::one(Loss::SubstituteStretch), Some(source.stretch()))
        }
        Drawn::FromTheShippedFace => (Losses::default(), None),
    }
}

/// Whether the viewer draws this substitute's outlines at a horizontal scale (ADR 0358).
fn fitted(source: &LoadedFont) -> bool {
    source.stretch().to_bits() != 1.0_f32.to_bits()
}

/// Which face the viewer draws `source` from, where that is the shipped face or a machine's, for
/// the route into a file's own descriptor that restates nothing.
///
/// [`drawn_from`]'s question without its refusals: that route has already refused a shown code
/// the shipped face lacks (`super::face_widths`), and a font the viewer draws from no program at
/// all is not one embedding changes the drawing of.
pub(super) fn drawn_by_the_viewer(source: &LoadedFont, face: &ShippedFace) -> Option<Drawn> {
    source.substitute_program().map(|drawn| {
        if drawn == face.program {
            Drawn::FromTheShippedFace
        } else {
            Drawn::FromAMachineFace
        }
    })
}

/// The program stream a shipped face was written to in this document, written now if it was not.
///
/// One copy per face per document: fonts naming the same face share it, since a descriptor names
/// a stream and nothing in §9.9 makes a stream one descriptor's.
fn shared_program(
    document: &Document,
    face: &ShippedFace,
    row: Table124,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<ObjectId, Because> {
    if let Some(at) = substitutes.programs.get(face.describe()) {
        return Ok(*at);
    }
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let mut stream = program_stream(
        subtype_dictionary(row).as_ref(),
        face.program,
        row.key == "FontFile2",
    )
    .ok_or(Because::NotBuiltYet(super::PROGRAM_NOT_ENCODED))?;
    super::notices::attach(
        document,
        face.program,
        &mut stream,
        spare,
        &mut substitutes.written,
    )?;
    substitutes.written.insert(at, stream);
    substitutes.programs.insert(face.describe(), at);
    Ok(at)
}

/// A font's program, borrowed from the shipped face or written afresh, and the name of the empty
/// glyph each code it gained one for is drawn with.
type ProgramWithEmptyGlyphs<'face> = (std::borrow::Cow<'face, [u8]>, BTreeMap<u8, String>);

/// The program a font is written with, and the empty glyph each of its codes gained.
///
/// **A code the file's own encoding sends to `.notdef` is given an empty glyph** where the face
/// can carry one, `doc/questions/A48`'s permission (`doc/adr/2458`): the program written is then
/// this font's own, and the encoding names every such code's glyph. A font with no such code
/// keeps the shipped program, shared with every other font that selects it.
fn with_empty_glyphs<'face>(
    face: &'face ShippedFace,
    source: &LoadedFont,
    used: &SelectedFont,
    target: pdf_archive::Target,
) -> Result<ProgramWithEmptyGlyphs<'face>, Because> {
    let empty_codes = super::empty::codes(source, used, |code| shipped_glyph(face, code));
    if empty_codes.is_empty() {
        return Ok((std::borrow::Cow::Borrowed(face.program), BTreeMap::new()));
    }
    if !super::empty::offered(target) {
        return Err(Because::NotThisTarget(
            super::empty::NO_CHARACTER_AT_THIS_LEVEL,
        ));
    }
    let (program, names) = super::empty::written(face.program, face.format, &empty_codes)?;
    Ok((std::borrow::Cow::Owned(program), names))
}

/// The program object for one font whose face gained empty glyphs, which no other font shares:
/// the glyphs are named by this font's encoding and stated at this font's widths.
fn own_program(
    document: &Document,
    face: &ShippedFace,
    program: &[u8],
    row: Table124,
    spare: &mut Spare,
    substitutes: &mut Substitutes,
) -> Result<ObjectId, Because> {
    let at = spare
        .take(document)
        .ok_or(Because::NotBuiltYet(super::NO_SPARE_OBJECT))?;
    let mut stream = program_stream(
        subtype_dictionary(row).as_ref(),
        program,
        row.key == "FontFile2",
    )
    .ok_or(Because::NotBuiltYet(super::PROGRAM_NOT_ENCODED))?;
    super::notices::attach(
        document,
        face.program,
        &mut stream,
        spare,
        &mut substitutes.written,
    )?;
    substitutes.written.insert(at, stream);
    Ok(at)
}

/// Whether the face's own advance is the width every shown code is laid out by: route 1 alone.
///
/// A face whose advances are not the file's numbers would have to be restated, which modifies the
/// program, and Liberation's licence reserves its name for an unmodified one (`doc/adr/2146`).
fn advances_are_the_faces(
    used: &SelectedFont,
    drawn: Drawn,
    source: &LoadedFont,
    face: &ShippedFace,
) -> Result<(), Because> {
    for text in used.shown.keys() {
        for code in source.decode(text) {
            // The glyph the embedded face will draw: the one the viewer draws, or where that is a
            // machine's face, the shipped face's own for the code.
            let glyph = match drawn {
                Drawn::FromTheShippedFace => source.glyph_index(code),
                Drawn::FromAMachineFace => shipped_glyph(face, code),
            };
            // A code the encoding sends to `.notdef` has an empty glyph of the font's own width
            // by now, or has refused (`super::empty::written`); [`same_metrics`] holds it.
            if super::empty::reaches_notdef(source, code) {
                continue;
            }
            let glyph = glyph
                .filter(|glyph| *glyph != pdf_font::NOTDEF_GLYPH)
                .ok_or(Because::NotBuiltYet(super::NO_SHIPPED_FACE_COVERS_IT))?;
            let own = pdf_font::restate::advance(face.program, face.format, glyph);
            if own.is_none_or(|own| super::disagrees(source.advance(code), own)) {
                return Err(Because::NotBuiltYet(WIDTHS_NOT_THE_FACES));
            }
        }
    }
    Ok(())
}

/// The encoding a `TrueType` restatement of this font carries: `None` for the file's own,
/// untouched, and a restated one where the file's is not of the kind ISO 19005-2 section 6.2.11.6
/// admits for a non-symbolic TrueType font.
fn truetype_encoding(
    document: &Document,
    dict: &Dictionary,
    source: &LoadedFont,
) -> Option<Dictionary> {
    let stated = document.get_key(dict, "Encoding");
    let admitted = |name: Option<&Name>| {
        name.is_some_and(|name| matches!(name.as_bytes(), b"WinAnsiEncoding" | b"MacRomanEncoding"))
    };
    if admitted(stated.as_name())
        || stated.as_dict().is_some_and(|encoding| {
            admitted(document.get_key(encoding, "BaseEncoding").as_name())
                && differences_are_listed_names(document, encoding)
        })
    {
        return None;
    }
    Some(restated_encoding(
        source,
        Some((BaseEncoding::WinAnsi, "WinAnsiEncoding")),
        &BTreeMap::new(),
    ))
}

/// Whether every name an encoding dictionary's `/Differences` states is one the Adobe Glyph List
/// answers for.
///
/// ISO 19005-2 section 6.2.11.6 and ISO 19005-4 section 6.2.10.6 let a non-symbolic TrueType font
/// state a `/Differences` array only where every glyph name in it is in the Adobe Glyph List — and
/// `.notdef`, which producers write over the codes they leave unused, is not. A `Type1` font may
/// state one freely, so the restatement as `TrueType` is where the rule first binds it, and an
/// encoding that fails it is restated rather than carried ([`restated_encoding`] names no code
/// `.notdef`). The reading is the validator's own, [`pdf_font::encoding::text_for`], and a name
/// that is not text is not one the list holds (`doc/adr/2278`).
fn differences_are_listed_names(document: &Document, encoding: &Dictionary) -> bool {
    let differences = document.get_key(encoding, "Differences");
    differences.as_array().is_none_or(|items| {
        items.iter().all(|item| match document.resolve(item) {
            Object::Name(name) => std::str::from_utf8(name.as_bytes())
                .is_ok_and(|name| pdf_font::encoding::text_for(name).is_some()),
            _ => true,
        })
    })
}

/// The source's code-to-name table, written as an encoding dictionary on `base`.
///
/// §9.6.5.1 makes an encoding exactly that table, so this restates it rather than changing it:
/// `/Differences` names, for every code whose glyph name the source's encoding states and the
/// base does not, the name the source gave it. With no base, every named code is listed, which is
/// what a symbolic face needs: §9.6.5.1 makes the base of a dictionary without `/BaseEncoding` the
/// *program's* built-in encoding, and the Annex D.5 and D.6 table the viewer read the source's
/// names from is not the shipped program's. [`proved`] then checks that each name reaches the
/// same glyph.
fn restated_encoding(
    source: &LoadedFont,
    base: Option<(BaseEncoding, &str)>,
    empty: &BTreeMap<u8, String>,
) -> Dictionary {
    let mut differences = Vec::new();
    let mut run: Option<u8> = None;
    for code in 0..=u8::MAX {
        // A code given an empty glyph names it (`super::empty`); every other code names what it
        // named before.
        let wanted = empty.get(&code).map(String::as_str).or_else(|| {
            source
                .selected_glyph_name(Code::single_byte(code))
                .filter(|name| !name.is_empty() && *name != ".notdef")
        });
        let Some(name) = wanted else {
            continue;
        };
        if base.is_some_and(|(base, _)| base.glyph_name(code) == name) {
            continue;
        }
        // `/Differences` restarts its count at each integer; consecutive codes share one.
        if run.and_then(|last| last.checked_add(1)) != Some(code) {
            differences.push(Object::Integer(code.into()));
        }
        differences.push(Object::Name(Name::new(name.as_bytes())));
        run = Some(code);
    }
    let mut encoding = Dictionary::new();
    encoding.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Encoding"[..])),
    );
    if let Some((_, named)) = base {
        encoding.insert(
            Name::new(&b"BaseEncoding"[..]),
            Object::Name(Name::new(named.as_bytes())),
        );
    }
    if !differences.is_empty() {
        encoding.insert(Name::new(&b"Differences"[..]), Object::Array(differences));
    }
    encoding
}

/// `/FirstChar`, `/LastChar` and `/Widths` stating the advance the source is laid out by.
///
/// The range is every code whose advance is not the descriptor's `/MissingWidth` default of 0, so
/// a code outside it is laid out at 0 in the output exactly as in the source. A whole number of
/// glyph-space units is written as an integer, which is what the standard's own metrics are.
fn laid_out_widths(source: &LoadedFont) -> (u8, u8, Vec<Object>) {
    let advance = |code: u8| source.advance(Code::single_byte(code)) * 1000.0;
    let first = (0..=u8::MAX)
        .find(|code| advance(*code) != 0.0)
        .unwrap_or(0);
    let last = (0..=u8::MAX)
        .rev()
        .find(|code| advance(*code) != 0.0)
        .unwrap_or(0);
    let widths = (first..=last)
        .map(|code| {
            let width = advance(code);
            let whole = width.round();
            if (width - whole).abs() < f32::EPSILON * width.abs().max(1.0) {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "a glyph advance in thousandths of an em, rounded and checked whole"
                )]
                Object::Integer(whole as i64)
            } else {
                Object::Real(f64::from(width))
            }
        })
        .collect();
    (first, last.max(first), widths)
}

/// Writes the converted font into a document of its own and holds it to the source, code by code.
///
/// Three equalities, each of which a reader would see if it failed: the outline of every glyph the
/// page shows, the advance of every one of the 256 codes, and the horizontal scale outlines are
/// drawn at. And one inequality: the converted font is drawn from its own program, not a
/// substitute, which is the thing ISO 19005 asks for.
fn proved(
    document: &Document,
    used: &SelectedFont,
    held_to: HeldTo<'_>,
    entries: &Dictionary,
    descriptor: &Dictionary,
    row: Table124,
    program: &[u8],
) -> Result<(), Because> {
    let mut assembly = Assembly::new(Vec::new());
    let unproved = |_| Because::NotBuiltYet(NOT_PROVED);
    let program = program_stream(
        subtype_dictionary(row).as_ref(),
        program,
        row.key == "FontFile2",
    )
    .ok_or(Because::NotBuiltYet(super::PROGRAM_NOT_ENCODED))?;
    let program_at = assembly.add(program).map_err(unproved)?;
    let mut held = descriptor.clone();
    held.insert(Name::new(row.key.as_bytes()), Object::Reference(program_at));
    let descriptor_at = assembly.add(Object::Dictionary(held)).map_err(unproved)?;
    let mut font = Dictionary::new();
    for (key, value) in used.dict.iter() {
        // The source's own references name objects of the source; every value the reader needs
        // is resolved into the copy, and `/ToUnicode` — which selects no glyph — is left out.
        if key.as_bytes() != b"ToUnicode" {
            font.insert(key.clone(), resolved(document, value, 4));
        }
    }
    for (key, value) in entries.iter() {
        font.insert(key.clone(), value.clone());
    }
    font.insert(
        Name::new(&b"FontDescriptor"[..]),
        Object::Reference(descriptor_at),
    );
    let font_at = assembly.add(Object::Dictionary(font)).map_err(unproved)?;
    let mut pages = Dictionary::new();
    pages.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Pages"[..])),
    );
    pages.insert(Name::new(&b"Kids"[..]), Object::Array(Vec::new()));
    pages.insert(Name::new(&b"Count"[..]), Object::Integer(0));
    let pages_at = assembly.add(Object::Dictionary(pages)).map_err(unproved)?;
    let mut catalog = Dictionary::new();
    catalog.insert(
        Name::new(&b"Type"[..]),
        Object::Name(Name::new(&b"Catalog"[..])),
    );
    catalog.insert(Name::new(&b"Pages"[..]), Object::Reference(pages_at));
    let root = assembly
        .add(Object::Dictionary(catalog))
        .map_err(unproved)?;
    assembly.set_root(root);
    let mut bytes = Vec::new();
    serialize(
        &assembly,
        Version { major: 1, minor: 7 },
        Options::new(Form::Table),
        &mut bytes,
    )
    .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    let written = Document::open_with_limits(bytes, Limits::DEFAULT)
        .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    let dict = written
        .get(font_at)
        .as_dict()
        .cloned()
        .ok_or(Because::NotBuiltYet(NOT_PROVED))?;
    let converted = LoadedFont::load(&written, &dict, &used.name)
        .map_err(|_| Because::NotBuiltYet(NOT_PROVED))?;
    // Bit for bit, deliberately: each number is one reader's reading of one statement, so any
    // difference at all is a difference in what is drawn.
    // The converted font is held to no stretch on both routes: an embedded program is drawn as
    // its producer shipped it. Whatever scale the viewer fitted the source at — a machine's face
    // or the shipped one — is what embedding removes, and `losses_of` names it as the loss it is
    // rather than this proof refusing it (`doc/adr/2262`).
    let HeldTo {
        source,
        drawn,
        face,
    } = held_to;
    same_metrics(source, &converted)?;
    for text in used.shown.keys() {
        for code in source.decode(text) {
            let same = match drawn {
                Drawn::FromTheShippedFace if fitted(source) => {
                    match (&source.outline(code), &converted.outline(code)) {
                        (Some(before), Some(after)) => {
                            same_once_fitted(before, after, source.stretch())
                        }
                        (None, None) => true,
                        _ => false,
                    }
                }
                Drawn::FromTheShippedFace => {
                    match (&source.outline(code), &converted.outline(code)) {
                        (Some(before), Some(after)) => same_once_wound(before, after),
                        (None, None) => true,
                        // An empty glyph given where the encoding reached `.notdef` draws what no
                        // glyph drew (`super::empty`).
                        (None, Some(after)) => after.commands().is_empty(),
                        _ => false,
                    }
                }
                // The same program on both sides, so the same glyph is the same outline.
                Drawn::FromAMachineFace => {
                    let glyph = shipped_glyph(face, code);
                    glyph.is_some() && converted.glyph_index(code) == glyph
                }
            };
            if !same {
                return Err(Because::NotBuiltYet(DRAWN_DIFFERENTLY));
            }
        }
    }
    Ok(())
}

/// Holds every code's advance to the selected source, and the converted font to no stretch.
fn same_metrics(source: &LoadedFont, converted: &LoadedFont) -> Result<(), Because> {
    if converted.is_substituted() {
        return Err(Because::NotBuiltYet(NOT_PROVED));
    }
    if converted.stretch().to_bits() != 1.0_f32.to_bits() {
        return Err(Because::NotBuiltYet(FITTED_DIFFERENTLY));
    }
    for code in (0..=u8::MAX).map(Code::single_byte) {
        if converted.advance(code).to_bits() != source.advance(code).to_bits() {
            return Err(Because::NotBuiltYet(ADVANCES_DIFFER));
        }
    }
    Ok(())
}

/// Whether an embedded program's outline, wound the way the viewer winds a substitute's, is the
/// substitute's outline.
///
/// The one transformation the viewer applies to a substitute's outline and not to an embedded
/// program's: every substituted glyph is wound counter-clockwise, and an embedded one is left as
/// its producer drew it (ADR 0396). Reversing every subpath negates every winding number, and both
/// of §8.5.3.3's rules test a winding number's magnitude, so the glyph alone paints the same either
/// way — which is why the comparison undoes exactly that and nothing else.
fn same_once_wound(substituted: &pdf_render::Path, embedded: &pdf_render::Path) -> bool {
    if embedded.signed_area() < 0.0 {
        substituted.commands() == embedded.reversed().commands()
    } else {
        substituted.commands() == embedded.commands()
    }
}

/// Whether an embedded program's outline, drawn at the horizontal scale the viewer fitted the
/// substitute at, is the substitute's outline.
///
/// The viewer's pen places every point a program states at `(x · scale) · stretch` and an
/// embedded program's at `(x · scale) · 1`, so a stated point times the stretch is the
/// substitute's point **bit for bit**, and every vertical coordinate is equal outright. The one
/// thing that is not is a `TrueType` quadratic's elevation to a cubic, whose two control points
/// the pen computes from points already scaled: `s·a + ⅔(s·c − s·a)` against `s·(a + ⅔(c − a))`,
/// the same number up to the rounding of the two orders of operation. So a control point's
/// horizontal coordinate is held to [`ELEVATION_ROUNDING`] and nothing else is given any room —
/// this proves that the output draws the same glyph, and that what differs is the stretch alone,
/// which `losses_of` names (`doc/adr/2262`). Winding is undone as [`same_once_wound`] does.
fn same_once_fitted(
    substituted: &pdf_render::Path,
    embedded: &pdf_render::Path,
    stretch: f32,
) -> bool {
    use pdf_render::PathCommand;
    let embedded = if embedded.signed_area() < 0.0 {
        embedded.reversed()
    } else {
        embedded.clone()
    };
    let (before, after) = (substituted.commands(), embedded.commands());
    let stated = |source: pdf_render::Point, point: pdf_render::Point| {
        source.x.to_bits() == (point.x * stretch).to_bits()
            && source.y.to_bits() == point.y.to_bits()
    };
    let elevated = |source: pdf_render::Point, point: pdf_render::Point| {
        (source.x - point.x * stretch).abs() <= ELEVATION_ROUNDING
            && source.y.to_bits() == point.y.to_bits()
    };
    before.len() == after.len()
        && before.iter().zip(after).all(|pair| match pair {
            (PathCommand::MoveTo(a), PathCommand::MoveTo(b))
            | (PathCommand::LineTo(a), PathCommand::LineTo(b)) => stated(*a, *b),
            (PathCommand::CurveTo(a1, a2, a), PathCommand::CurveTo(b1, b2, b)) => {
                elevated(*a1, *b1) && elevated(*a2, *b2) && stated(*a, *b)
            }
            (PathCommand::Close, PathCommand::Close) => true,
            _ => false,
        })
}

/// The rounding two orders of one quadratic-to-cubic elevation can differ by, in ems: a few
/// units in the last place of a coordinate within a glyph's few ems, 2⁻²⁰ — a millionth of the
/// em, a thousandth of a pixel on a glyph a thousand pixels tall.
const ELEVATION_ROUNDING: f32 = 1.0 / 1_048_576.0;

/// A value with every reference in it replaced by what it names, to a depth.
///
/// Deep enough for what a font dictionary holds — an encoding dictionary whose `/Differences` is
/// itself an object, a `/Widths` array of indirect numbers — and bounded, because a reference
/// cycle is a malformed file and not a reason to recurse forever. A stream is left out: the only
/// one a simple font dictionary names that selects a glyph is the program, which [`proved`] writes
/// itself.
fn resolved(document: &Document, value: &Object, depth: u8) -> Object {
    let Some(deeper) = depth.checked_sub(1) else {
        return Object::Null;
    };
    match document.resolve(value) {
        Object::Array(items) => Object::Array(
            items
                .iter()
                .map(|item| resolved(document, item, deeper))
                .collect(),
        ),
        Object::Dictionary(dict) => {
            let mut out = Dictionary::new();
            for (key, item) in dict.iter() {
                out.insert(key.clone(), resolved(document, item, deeper));
            }
            Object::Dictionary(out)
        }
        Object::Stream(_) | Object::Reference(_) => Object::Null,
        other => other,
    }
}

/// The numbers §9.8.1's Table 120 requires of a descriptor, read off one face's program.
///
/// Every one is a statement the program makes or a measurement of its outlines, in the
/// thousandths of an em §9.2.4 puts glyph space in:
///
/// - `/FontBBox`, `/ItalicAngle` and whether the face is fixed-pitch are the program's own
///   statements: a Compact Font Format Top DICT's `FontBBox`, `ItalicAngle` and `isFixedPitch`, an
///   sfnt's `head` bounding box and `post` table.
/// - `/Ascent`, `/Descent` and `/CapHeight` are measured, because Table 120 defines them as
///   measurements — "\[t]he maximum height above the baseline reached by glyphs in this font. The
///   height of glyphs for accented characters shall be excluded", "the maximum depth below the
///   baseline", "the vertical coordinate of the top of flat capital letters" — and a Compact Font
///   Format program states none of them. Over a Latin face the glyphs are those whose names the
///   Adobe Glyph List takes to a printable ASCII character, which holds no accented character and
///   every unaccented one of the standard Latin set's letters, digits and punctuation; over the two
///   symbolic faces, every glyph but `.notdef`; and the capital is `H`.
/// - `/StemV` is the Compact Font Format Private DICT's `StdVW`, the program's own statement of
///   its dominant vertical stem, and 0 for an sfnt, which states none — Table 120's own "\[a] value
///   of 0 indicates an unknown stem thickness".
struct Measured {
    bbox: [f32; 4],
    italic_angle: f32,
    fixed_pitch: bool,
    ascent: f32,
    descent: f32,
    cap_height: Option<f32>,
    stem_v: f32,
}

impl Measured {
    /// Reads and measures one face, `None` where its program does not open.
    fn of(face: &ShippedFace) -> Option<Self> {
        let latin = !matches!(face.request.family, Family::Symbol | Family::ZapfDingbats);
        match face.format {
            Format::Sfnt => Self::of_sfnt(face.program, latin),
            Format::BareCff => Self::of_cff(face.program, latin),
            _ => None,
        }
    }

    fn of_sfnt(program: &[u8], latin: bool) -> Option<Self> {
        let font = skrifa::FontRef::new(program).ok()?;
        let stated = font.metrics(Size::unscaled(), LocationRef::default());
        let units = f32::from(stated.units_per_em);
        if units <= 0.0 {
            return None;
        }
        let scale = 1000.0 / units;
        let bounds = stated.bounds?;
        let outlines = font.outline_glyphs();
        let mut extent = Extent::default();
        let mut cap_height = None;
        let charmap = font.charmap();
        let measured: Vec<(char, skrifa::GlyphId)> = if latin {
            ('!'..='~')
                .filter_map(|character| Some((character, charmap.map(character)?)))
                .collect()
        } else {
            charmap
                .mappings()
                .filter_map(|(character, glyph)| Some((char::from_u32(character)?, glyph)))
                .collect()
        };
        for (character, glyph) in measured {
            if glyph.to_u32() == 0 {
                continue;
            }
            let mut pen = Bounds::default();
            let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
            outlines.get(glyph)?.draw(settings, &mut pen).ok()?;
            extent.include(&pen);
            if latin && character == 'H' {
                cap_height = pen.y_max();
            }
        }
        Some(Self {
            bbox: [bounds.x_min, bounds.y_min, bounds.x_max, bounds.y_max].map(|v| v * scale),
            italic_angle: stated.italic_angle,
            fixed_pitch: stated.is_monospace,
            ascent: extent.y_max * scale,
            descent: extent.y_min * scale,
            cap_height: cap_height.map(|height| height * scale),
            stem_v: 0.0,
        })
    }

    fn of_cff(program: &[u8], latin: bool) -> Option<Self> {
        let font = CffFontRef::new_cff(program, 0, None).ok()?;
        let units = f32::from(u16::try_from(font.upem()).ok()?);
        if units <= 0.0 {
            return None;
        }
        let scale = 1000.0 / units;
        let meta = font.metadata()?;
        let bbox = meta.bbox();
        let mut extent = Extent::default();
        let mut cap_height = None;
        for (glyph, sid) in font.charset()?.iter() {
            if glyph.to_u32() == 0 {
                continue;
            }
            let name = font.string(sid)?;
            let character = std::str::from_utf8(name)
                .ok()
                .and_then(skrifa::raw::ps::agl::name_to_char);
            if latin && !character.is_some_and(|it| it.is_ascii_graphic()) {
                continue;
            }
            let subfont = font.subfont(font.subfont_index(glyph)?, &[]).ok()?;
            let mut pen = Bounds::default();
            font.draw(&subfont, glyph, &[], None, &mut pen).ok()?;
            extent.include(&pen);
            if latin && character == Some('H') {
                cap_height = pen.y_max();
            }
        }
        Some(Self {
            bbox: [bbox.x_min, bbox.y_min, bbox.x_max, bbox.y_max].map(|v| v.to_f32() * scale),
            italic_angle: meta.italic_angle().to_f32(),
            fixed_pitch: meta.is_fixed_pitch(),
            ascent: extent.y_max * scale,
            descent: extent.y_min * scale,
            cap_height: cap_height.map(|height| height * scale),
            stem_v: std_vw(program).map_or(0.0, |stem| stem * scale),
        })
    }

    /// The descriptor dictionary, naming the program at `program` under `row`'s key.
    ///
    /// `/Flags` is §9.8.2's Table 121 read from the face: `FixedPitch` where the program says
    /// so, `Serif` for the face this program ships as Times, `Italic` where the program's angle is
    /// not zero, and exactly one of `Symbolic` and `Nonsymbolic` — the first for Symbol and
    /// `ZapfDingbats`, whose character sets are their own (Annex D.5, D.6), the second for a Latin
    /// face set with a Latin encoding, the reading ADR 1099's appended-page font already takes.
    fn descriptor(
        &self,
        font_name: &[u8],
        face: &ShippedFace,
        row: Table124,
        program: ObjectId,
    ) -> Dictionary {
        let symbolic = matches!(face.request.family, Family::Symbol | Family::ZapfDingbats);
        let mut flags = 0_i64;
        if self.fixed_pitch {
            flags |= 1;
        }
        if face.request.family == Family::Serif {
            flags |= 1 << 1;
        }
        flags |= if symbolic { 1 << 2 } else { 1 << 5 };
        if self.italic_angle != 0.0 {
            flags |= 1 << 6;
        }
        let number = |value: f32| {
            let whole = value.round();
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a font metric in thousandths of an em, rounded to the whole unit a \
                          descriptor states it in and clamped far inside i64"
            )]
            Object::Integer(whole.clamp(-1.0e6, 1.0e6) as i64)
        };
        let mut dict = Dictionary::new();
        dict.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"FontDescriptor"[..])),
        );
        dict.insert(
            Name::new(&b"FontName"[..]),
            Object::Name(Name::new(font_name)),
        );
        dict.insert(Name::new(&b"Flags"[..]), Object::Integer(flags));
        dict.insert(
            Name::new(&b"FontBBox"[..]),
            Object::Array(self.bbox.iter().map(|side| number(*side)).collect()),
        );
        dict.insert(
            Name::new(&b"ItalicAngle"[..]),
            Object::Real(f64::from(self.italic_angle)),
        );
        dict.insert(Name::new(&b"Ascent"[..]), number(self.ascent));
        dict.insert(Name::new(&b"Descent"[..]), number(self.descent));
        if let Some(cap_height) = self.cap_height {
            dict.insert(Name::new(&b"CapHeight"[..]), number(cap_height));
        }
        dict.insert(Name::new(&b"StemV"[..]), number(self.stem_v));
        dict.insert(Name::new(row.key.as_bytes()), Object::Reference(program));
        dict
    }
}

/// A Compact Font Format program's Private DICT `StdVW`, where it states one.
fn std_vw(program: &[u8]) -> Option<f32> {
    let cff = skrifa::raw::tables::cff::Cff::read(skrifa::raw::FontData::new(program)).ok()?;
    let top = cff.top_dicts().get(0).ok()?;
    let private = entries(top, None).find_map(|entry| match entry {
        Ok(Entry::PrivateDictRange(range)) => Some(range),
        _ => None,
    })?;
    let private = program.get(private)?;
    entries(private, None).find_map(|entry| match entry {
        Ok(Entry::StdVw(stem)) => Some(stem.to_f32()),
        _ => None,
    })
}

/// The union of several glyphs' vertical extents, in the program's units.
#[derive(Default)]
struct Extent {
    y_min: f32,
    y_max: f32,
}

impl Extent {
    fn include(&mut self, glyph: &Bounds) {
        if let Some((low, high)) = glyph.vertical() {
            self.y_min = self.y_min.min(low);
            self.y_max = self.y_max.max(high);
        }
    }
}

/// One glyph's outline, collected so that its exact extent can be taken.
///
/// The extent of the *shape*, not of its control points: Table 120's quantities are heights a
/// glyph reaches, and a curve's off-curve points can lie beyond what it draws. `kurbo` computes a
/// path's extrema exactly, which is the reading this tree already takes of its geometry (ADR 1236).
#[derive(Default)]
struct Bounds {
    path: kurbo::BezPath,
}

impl Bounds {
    /// The lowest and highest the shape reaches, `None` for a glyph that draws nothing.
    fn vertical(&self) -> Option<(f32, f32)> {
        use kurbo::Shape as _;
        if self.path.elements().is_empty() {
            return None;
        }
        let rect = self.path.bounding_box();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "glyph coordinates in a program's own units, which an f32 holds exactly"
        )]
        Some((rect.y0 as f32, rect.y1 as f32))
    }

    fn y_max(&self) -> Option<f32> {
        self.vertical().map(|(_, high)| high)
    }
}

impl OutlinePen for Bounds {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path.move_to((f64::from(x), f64::from(y)));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to((f64::from(x), f64::from(y)));
    }
    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.path
            .quad_to((f64::from(cx), f64::from(cy)), (f64::from(x), f64::from(y)));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.path.curve_to(
            (f64::from(cx0), f64::from(cy0)),
            (f64::from(cx1), f64::from(cy1)),
            (f64::from(x), f64::from(y)),
        );
    }
    fn close(&mut self) {
        self.path.close_path();
    }
}

// ----------------------------------------------------------------------------------------------
// The refusals, each naming what stopped it.
// ----------------------------------------------------------------------------------------------

/// Why a font whose subtype is not a simple font's is not given a descriptor.
const NOT_A_SIMPLE_FONT: &str = "this rendered font states no FontDescriptor and its Subtype is \
     none of Type1, MMType1 and TrueType, the three kinds of simple font ISO 32000-2 \u{a7}9.6 \
     describes a program for, so the standard says nothing of what it is drawn or laid out by and \
     there is no description to embed a program against";

/// Why a font that states `/Widths` but no descriptor, outside the fourteen, is not described.
const WIDTHS_WITHOUT_A_DESCRIPTOR: &str = "this rendered font states /Widths and no \
     FontDescriptor and is not a Type1 font naming one of the fourteen ISO 32000-2 \u{a7}9.6.2.2 \
     lists, so its layout is the file's own and the face this program would embed is not the one \
     the widths were stated for; Table 109 lets only the standard 14 leave the descriptor out";

/// Why a font whose `/BaseFont` is present but is no name is not described.
const BASE_FONT_NOT_A_NAME: &str = "this rendered font states no FontDescriptor and a BaseFont \
     that is not a name, so there is no name for the descriptor's FontName to equal, and \
     replacing the entry is not an interpretation the standard defines";

/// Why a font the viewer draws from another face is not given this one.
const ANOTHER_FACE_IS_DRAWN: &str = "this rendered font names one of the standard 14 and does \
     not embed it, and this viewer draws it from no substitute face at all, so no face this \
     program ships is the one the page shows and none is embedded";

/// Why a font the viewer draws from a machine's face is not given the shipped one.
const ANOTHER_FACE_DRAWS_A_SHOWN_CODE: &str = "this rendered font names one of the standard 14 \
     and does not embed it, and this viewer draws it from a face of this machine's rather than the \
     one this program ships, because the document's encoding names characters the shipped face \
     lacks (ADR 0270) — and the page shows one of them. Embedding the shipped face would lose \
     that mark, and the machine's face is not this program's to embed, so the font is refused \
     rather than given a face that draws less than the page";

/// Why a face whose advances are not the widths the page is laid out by is not embedded.
const WIDTHS_NOT_THE_FACES: &str = "this rendered font names one of the standard 14 and states no \
     FontDescriptor, and for a code the page shows the face this program ships states an advance \
     more than ISO 19005-2 section 6.2.11.5's thousandth of an em away from the width the page is \
     laid out by. Embedding it would mean restating the program's advance, which modifies a face \
     whose licence reserves its name for an unmodified program, or restating the width, which \
     moves the glyph";

/// Why a face whose program cannot be measured is not described.
const PROGRAM_NOT_MEASURED: &str = "the face this program ships for this standard font could not \
     be read for the bounding box, italic angle and glyph extents ISO 32000-2 \u{a7}9.8.1's Table \
     120 requires of the descriptor this conversion would write";

/// Why a converted font that could not be read back is not written.
const NOT_PROVED: &str = "the font this conversion would write could not be read back as an \
     embedded font by this program's own font reader, so whether it draws the page this reader \
     shows is not known, and it is not written";

/// Why a converted font whose glyph or outline proof failed is not written.
const DRAWN_DIFFERENTLY: &str = "the embedded font read back by this program does not retain \
     the selected glyph or outline for a code the page shows, so it is not written: the \
     conversion would change the page";

/// An embedded program is drawn at no stretch, so one read back at another is not the program
/// this conversion wrote.
const FITTED_DIFFERENTLY: &str = "the embedded font read back by this program is drawn at a \
     horizontal scale, which an embedded program never is, so what it would draw is not known and \
     it is not written";

/// Width preservation is checked separately from the painted outlines.
const ADVANCES_DIFFER: &str = "the embedded font read back by this program advances at least one \
     character code differently from the source font, so it is not written: the conversion would \
     change text positioning";
