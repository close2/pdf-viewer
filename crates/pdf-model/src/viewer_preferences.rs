//! ISO 32000-2 §12.2's viewer preferences: how a document asks to be presented.
//!
//! **And Table 29's two display entries**, which are §7.7.2's and are here because they are the
//! same question asked by the same table's neighbour and answered by the same host: `/PageMode`
//! says which panel is open when the document is opened and `/PageLayout` how the pages are laid
//! out, and §12.2's `/NonFullScreenPageMode` is defined by reference to the first of them.
//! [`Opening`] is that pair and is deliberately a separate type — one struct holding two tables
//! is how a row comes to claim what its neighbour implements.
//!
//! The catalog's `/ViewerPreferences` is a dictionary "controlling the way the document shall
//! be presented on the screen or in print", and §12.2 states the fallback for a document that
//! has none: "PDF processors should behave in accordance with their own current user
//! preference settings". So every field here has a default, and a document that states
//! nothing gets the same struct a document that states the defaults does.
//!
//! # Two of Table 147's entries decide pixels; the rest decide a window
//!
//! `/ViewArea` and `/ViewClip` name **page boundaries** (§14.11.2): which of the five is
//! displayed, and which the contents are clipped to. Those are read by [`crate::page`] and
//! reach the rasteriser, because they change what is on the screen. The rest — hiding a tool
//! bar, centring a window, a print dialogue's copy count — describe a user interface this
//! crate does not have and a printer it does not drive, and are read here as data for
//! whatever does.
//!
//! Both are *deprecated in PDF 2.0* and both are still read, for the same reason §8.6.5.1's
//! withdrawn `CalCMYK` is: a deprecated entry is one a file may still contain, and this
//! reader's job is to draw the file it was given. What deprecation changes is what a *writer*
//! should do.
//!
//! # What the corpus states
//!
//! 58 of the 974 documents state a `/ViewerPreferences` at all: `/Direction` 29 (27 of them
//! `L2R`, which is the default), `/DisplayDocTitle` 22, `/PrintScaling` 2, and one each of
//! `/HideWindowUI`, `/FitWindow`, `/CenterWindow`, `/HideMenubar` and `/HideToolbar` — plus
//! three that write a `/PageDirection`, which is not in Table 147 at all. **Not one states
//! `/ViewArea`, `/ViewClip`, `/PrintArea` or `/PrintClip`**, so the half of this clause that
//! can change a pixel has no corpus witness and is implemented from the clause alone.

use pdf_syntax::{Dictionary, Document, Name, Object};

use crate::page::Boundary;

/// §12.2's viewer preferences dictionary, Table 147, with Table 147's own defaults applied.
///
/// One field per entry, which is what makes six of them booleans in a row. `struct_excessive_bools`
/// is asking for a bitflags type or an enum; Table 147 states six independent flags with six
/// independent defaults, and a reader that packed them would have to unpack them again at every
/// use — the shape of the type is the shape of the table on purpose.
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per Table 147 entry; the table states six independent flags"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewerPreferences {
    /// `/HideToolbar`: hide the processor's tool bars while the document is active.
    pub hide_toolbar: bool,
    /// `/HideMenubar`: hide its menu bar.
    pub hide_menubar: bool,
    /// `/HideWindowUI`: hide the window's own scroll bars and navigation controls.
    pub hide_window_ui: bool,
    /// `/FitWindow`: resize the window to the first displayed page.
    pub fit_window: bool,
    /// `/CenterWindow`: put the window in the centre of the screen.
    pub center_window: bool,
    /// `/DisplayDocTitle`: title the window from the document rather than from the file.
    ///
    /// The title itself is "taken from the `dc:title` element of the XMP metadata stream (see
    /// 14.3.2, "Metadata streams")", which nothing in this tree parses — so a caller that
    /// honours this has to ask §14.3.2 for the title and fall back to the file name, which is
    /// what the entry's `false` case asks for anyway. 22 corpus documents set it.
    pub display_doc_title: bool,
    /// `/NonFullScreenPageMode`: which panel is open on leaving full-screen mode.
    ///
    /// "This entry is meaningful only if the value of the `PageMode` entry in the catalog
    /// dictionary … is `FullScreen`; it shall be ignored otherwise", which is a condition on
    /// the *catalog* rather than on this dictionary — so it is read here and the condition is
    /// the caller's, which is the only place the two entries are both in view.
    pub non_full_screen_page_mode: PageMode,
    /// `/Direction`: the predominant logical content order for text.
    ///
    /// The clause is explicit that this is not about the content: it "has no direct effect on
    /// the document's contents or page numbering but may be used to determine the relative
    /// positioning of pages when displayed side by side or printed n-up". So a right-to-left
    /// document does not have its text reordered by this entry, and a viewer showing one page
    /// at a time has nothing to do with it at all.
    pub direction: Direction,
    /// `/ViewArea`, **deprecated in PDF 2.0**: the boundary displayed on screen.
    pub view_area: Boundary,
    /// `/ViewClip`, **deprecated in PDF 2.0**: the boundary screen contents are clipped to.
    pub view_clip: Boundary,
    /// `/PrintArea`, **deprecated in PDF 2.0**: the boundary rendered when printing.
    pub print_area: Boundary,
    /// `/PrintClip`, **deprecated in PDF 2.0**: the boundary print contents are clipped to.
    pub print_clip: Boundary,
    /// `/PrintScaling`: the scaling a print dialogue opens with.
    pub print_scaling: PrintScaling,
    /// `/Duplex`: the paper handling option printing should use.
    ///
    /// `None` is Table 147's own answer — the default value is "implementation dependent",
    /// which is a different thing from a stated default and is worth keeping distinguishable.
    pub duplex: Option<Duplex>,
    /// `/PickTrayByPDFSize`: choose the input tray from the page size. Implementation
    /// dependent by default, so `None` where the document says nothing.
    pub pick_tray_by_pdf_size: Option<bool>,
    /// `/PrintPageRange`: pairs of first and last pages, **one-based as the clause states**.
    ///
    /// Kept as the clause writes it rather than converted, because its own NOTE is that
    /// "although `PrintPageRange` uses 1-based page numbering, other features of PDF use
    /// zero-based page numbering" — a converted value would be indistinguishable from the
    /// other kind at the point of use. Empty where the document states none, and a malformed
    /// array — an odd number of integers, or a non-integer — states no range at all.
    pub print_page_range: Vec<(i64, i64)>,
    /// `/NumCopies`: how many copies a print dialogue opens with.
    pub num_copies: Option<i64>,
    /// Table 147's `/Enforce`: the preferences a processor may not let a person override.
    ///
    /// Only Table 148's `/PrintScaling` is defined for it, and only "if the corresponding entry in the
    /// viewer preferences dictionary … specifies a valid value other than `AppDefault`" —
    /// a condition this reader applies rather than storing a name that means nothing.
    pub enforce_print_scaling: bool,
}

/// Table 29's `/PageMode`, of which Table 147's `/NonFullScreenPageMode` is all but one.
///
/// **The two tables share a vocabulary and not a value set**, and the difference between them is
/// exactly one name. §7.7.2's `/PageMode` states six; §12.2's `/NonFullScreenPageMode` does not
/// state `FullScreen`, because that name is the entry's own condition. [`ViewerPreferences::in_catalog`]
/// refuses it where the clause does not offer it — a rule about *which* entry was read rather than
/// about what the name means, and so one that belongs at the reading site.
///
/// **This used to refuse `UseAttachments` there as well, and an erratum says otherwise.** As
/// printed, Table 147's value list is `UseNone`, `UseOutlines`, `UseThumbs`, `UseOC` and stops.
/// ISO 32000-2 errata issue #275, state Review/Completed, inserts a `UseAttachments` row — the
/// attachments panel, PDF 2.0 — into that very cell: `cargo run -p spec-errata -- emit doc/*.pdf`
/// reports it under §12.2, and the caret's own rectangle sits at the start of the line beginning
/// *This entry is meaningful only if*, which is the line directly after `UseOC`'s. So the two
/// lists differ by `FullScreen` alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageMode {
    /// `UseNone`: "[n]either document outline nor thumbnail images visible". The default.
    #[default]
    UseNone,
    /// `UseOutlines`: "[d]ocument outline visible".
    UseOutlines,
    /// `UseThumbs`: "[t]humbnail images visible".
    UseThumbs,
    /// `UseOC`: "[o]ptional content group panel visible".
    UseOptionalContent,
    /// `FullScreen`: "[f]ull-screen mode, with no menu bar, window controls, or any other
    /// window visible". Table 29 only.
    FullScreen,
    /// `UseAttachments`: "[a]ttachments panel visible". PDF 1.6 in Table 29, PDF 2.0 in Table 147
    /// by errata issue #275.
    UseAttachments,
}

/// Table 29's `/PageLayout`: "the page layout shall be used when the document is opened".
///
/// Six names and one default, and the difference between them is *how many pages are on the
/// screen and whether they scroll*, which is a property of a window rather than of a page. Read
/// and handed over, like the rest of what a document says about a window it cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageLayout {
    /// `SinglePage`: "[d]isplay one page at a time". The default.
    #[default]
    SinglePage,
    /// `OneColumn`: "[d]isplay the pages in one column".
    OneColumn,
    /// `TwoColumnLeft`: "in two columns, with odd-numbered pages on the left".
    TwoColumnLeft,
    /// `TwoColumnRight`: "in two columns, with odd-numbered pages on the right".
    TwoColumnRight,
    /// `TwoPageLeft`: "two at a time, with odd-numbered pages on the left" (PDF 1.5).
    TwoPageLeft,
    /// `TwoPageRight`: "two at a time, with odd-numbered pages on the right" (PDF 1.5).
    TwoPageRight,
}

/// Table 29's two display entries: what the catalog asks of the window that opens it.
///
/// Both are *optional with a stated default*, so a document that says nothing gets the same
/// struct as one that says `UseNone` and `SinglePage` — the same rule §12.2 states for its own
/// dictionary, and the reason neither field is an `Option`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Opening {
    /// `/PageMode`: "how the document shall be displayed when opened".
    pub mode: PageMode,
    /// `/PageLayout`: "the page layout shall be used when the document is opened".
    pub layout: PageLayout,
}

impl Opening {
    /// Reads the catalog's `/PageMode` and `/PageLayout`.
    ///
    /// A name outside either table is not a value this processor can honour, so it produces the
    /// entry's default rather than an error — Table 29 states one for both, and `Use0` is what
    /// one corpus document writes.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        Self::in_catalog(document, &catalog)
    }

    /// The same, for a caller that already holds the catalog.
    #[must_use]
    pub fn in_catalog(document: &Document, catalog: &Dictionary) -> Self {
        Self {
            mode: named(document, catalog, "PageMode", page_mode).unwrap_or_default(),
            layout: named(document, catalog, "PageLayout", |name| {
                Some(match name.as_bytes() {
                    b"OneColumn" => PageLayout::OneColumn,
                    b"TwoColumnLeft" => PageLayout::TwoColumnLeft,
                    b"TwoColumnRight" => PageLayout::TwoColumnRight,
                    b"TwoPageLeft" => PageLayout::TwoPageLeft,
                    b"TwoPageRight" => PageLayout::TwoPageRight,
                    // Including `SinglePage` itself, and including a name Table 29 does not
                    // define: both are the entry's default.
                    _ => PageLayout::SinglePage,
                })
            })
            .unwrap_or_default(),
        }
    }
}

/// Table 29's `/PageMode` names, whole.
fn page_mode(name: &Name) -> Option<PageMode> {
    Some(match name.as_bytes() {
        b"UseNone" => PageMode::UseNone,
        b"UseOutlines" => PageMode::UseOutlines,
        b"UseThumbs" => PageMode::UseThumbs,
        b"UseOC" => PageMode::UseOptionalContent,
        b"FullScreen" => PageMode::FullScreen,
        b"UseAttachments" => PageMode::UseAttachments,
        _ => return None,
    })
}

/// Table 147's `/Direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// `L2R`, left to right. The default.
    #[default]
    LeftToRight,
    /// `R2L`, right to left, "including vertical writing systems, such as Chinese, Japanese,
    /// and Korean".
    RightToLeft,
}

/// Table 147's `/PrintScaling`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintScaling {
    /// `AppDefault`, the processor's own default, and what an unrecognised value means.
    #[default]
    AppDefault,
    /// `None`: no page scaling.
    NoScaling,
}

/// Table 147's `/Duplex`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Duplex {
    /// `Simplex`: print single-sided.
    Simplex,
    /// `DuplexFlipShortEdge`.
    FlipShortEdge,
    /// `DuplexFlipLongEdge`.
    FlipLongEdge,
}

impl Default for ViewerPreferences {
    /// Table 147's defaults, which are also §12.2's answer for a document with no dictionary.
    fn default() -> Self {
        Self {
            hide_toolbar: false,
            hide_menubar: false,
            hide_window_ui: false,
            fit_window: false,
            center_window: false,
            display_doc_title: false,
            non_full_screen_page_mode: PageMode::UseNone,
            direction: Direction::LeftToRight,
            view_area: Boundary::Crop,
            view_clip: Boundary::Crop,
            print_area: Boundary::Crop,
            print_clip: Boundary::Crop,
            print_scaling: PrintScaling::AppDefault,
            duplex: None,
            pick_tray_by_pdf_size: None,
            print_page_range: Vec::new(),
            num_copies: None,
            enforce_print_scaling: false,
        }
    }
}

impl ViewerPreferences {
    /// Reads the catalog's `/ViewerPreferences`, or Table 147's defaults where there is none.
    #[must_use]
    pub fn read(document: &Document) -> Self {
        let Ok(catalog) = document.catalog() else {
            return Self::default();
        };
        Self::in_catalog(document, &catalog)
    }

    /// The same, for a caller that already holds the catalog.
    ///
    /// [`crate::Pages`] is that caller and is on the startup path: it reads the catalog to
    /// find `/Pages`, and asking for it a second time would copy the dictionary again to
    /// answer a question about a key most documents do not have.
    #[must_use]
    pub fn in_catalog(document: &Document, catalog: &Dictionary) -> Self {
        let preferences = document.get_key(catalog, "ViewerPreferences");
        let Some(preferences) = preferences.as_dict() else {
            return Self::default();
        };

        let flag = |key: &str| match document.get_key(preferences, key) {
            Object::Boolean(value) => Some(value),
            _ => None,
        };
        let boundary = |key: &str| {
            document
                .get_key(preferences, key)
                .as_name()
                .and_then(Boundary::read)
        };
        let print_scaling = named(document, preferences, "PrintScaling", |name| {
            match name.as_bytes() {
                b"None" => Some(PrintScaling::NoScaling),
                // "If this entry has an unrecognised value, AppDefault shall be used", which
                // includes the name `AppDefault` itself.
                _ => Some(PrintScaling::AppDefault),
            }
        })
        .unwrap_or_default();

        Self {
            hide_toolbar: flag("HideToolbar").unwrap_or(false),
            hide_menubar: flag("HideMenubar").unwrap_or(false),
            hide_window_ui: flag("HideWindowUI").unwrap_or(false),
            fit_window: flag("FitWindow").unwrap_or(false),
            center_window: flag("CenterWindow").unwrap_or(false),
            display_doc_title: flag("DisplayDocTitle").unwrap_or(false),
            non_full_screen_page_mode: named(
                document,
                preferences,
                "NonFullScreenPageMode",
                // Table 147 lists five of Table 29's six here — four as printed and
                // `UseAttachments` by errata issue #275 — and the one it leaves out is refused
                // rather than accepted: `FullScreen` would be the entry's own condition read as
                // its value, and a window cannot exit full-screen mode into full-screen mode.
                |name| page_mode(name).filter(|mode| !matches!(mode, PageMode::FullScreen)),
            )
            .unwrap_or_default(),
            direction: named(document, preferences, "Direction", |name| {
                match name.as_bytes() {
                    b"L2R" => Some(Direction::LeftToRight),
                    b"R2L" => Some(Direction::RightToLeft),
                    _ => None,
                }
            })
            .unwrap_or_default(),
            view_area: boundary("ViewArea").unwrap_or(Boundary::Crop),
            view_clip: boundary("ViewClip").unwrap_or(Boundary::Crop),
            print_area: boundary("PrintArea").unwrap_or(Boundary::Crop),
            print_clip: boundary("PrintClip").unwrap_or(Boundary::Crop),
            print_scaling,
            duplex: named(document, preferences, "Duplex", |name| {
                match name.as_bytes() {
                    b"Simplex" => Some(Duplex::Simplex),
                    b"DuplexFlipShortEdge" => Some(Duplex::FlipShortEdge),
                    b"DuplexFlipLongEdge" => Some(Duplex::FlipLongEdge),
                    _ => None,
                }
            }),
            pick_tray_by_pdf_size: flag("PickTrayByPDFSize"),
            print_page_range: page_range(document, preferences),
            num_copies: document.get_key(preferences, "NumCopies").as_integer(),
            enforce_print_scaling: enforced(document, preferences, "PrintScaling")
                && print_scaling != PrintScaling::AppDefault,
        }
    }
}

/// Reads a name-valued entry through a table of the names the clause defines.
///
/// `None` for an absent entry *and* for a name outside the table, which are the same thing
/// here: Table 147 gives every one of these a default, and a name the standard does not
/// define is not a value this processor can honour.
fn named<T>(
    document: &Document,
    preferences: &Dictionary,
    key: &str,
    read: impl Fn(&Name) -> Option<T>,
) -> Option<T> {
    read(document.get_key(preferences, key).as_name()?)
}

/// Table 147's `/PrintPageRange`, as the pairs the clause defines.
///
/// "The array shall contain an even number of integers to be interpreted in pairs, with each
/// pair specifying the first and last pages in a sub-range" — so an odd-length array, or one
/// holding anything but integers, has not stated a range and produces none rather than half of
/// one.
fn page_range(document: &Document, preferences: &Dictionary) -> Vec<(i64, i64)> {
    let array = document.get_key(preferences, "PrintPageRange");
    let Some(items) = array.as_array() else {
        return Vec::new();
    };
    if items.is_empty() || !items.len().is_multiple_of(2) {
        return Vec::new();
    }
    let mut numbers = Vec::with_capacity(items.len());
    for item in items {
        let Some(number) = document.resolve(item).as_integer() else {
            return Vec::new();
        };
        numbers.push(number);
    }
    numbers
        .chunks_exact(2)
        .filter_map(|pair| match pair {
            [first, last] => Some((*first, *last)),
            _ => None,
        })
        .collect()
}

/// Whether Table 147's `/Enforce` array names this preference.
fn enforced(document: &Document, preferences: &Dictionary, name: &str) -> bool {
    let array = document.get_key(preferences, "Enforce");
    let Some(items) = array.as_array() else {
        return false;
    };
    items.iter().any(|item| {
        document
            .resolve(item)
            .as_name()
            .is_some_and(|named| named.as_bytes() == name.as_bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::{Direction, Opening, PageLayout, PageMode, PrintScaling, ViewerPreferences};
    use crate::page::Boundary;
    use pdf_syntax::Document;

    fn document(catalog: &str) -> Document {
        use std::fmt::Write as _;
        let mut out = String::from("%PDF-2.0\n");
        let mut offsets = Vec::new();
        for (index, body) in [catalog, "<< /Type /Pages /Count 0 /Kids [] >>"]
            .iter()
            .enumerate()
        {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(out, "xref\n0 3\n0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
        );
        Document::open(out.into_bytes()).expect("a valid file")
    }

    /// A document with no `/ViewerPreferences` gets Table 147's defaults.
    ///
    /// §12.2: "If no such dictionary is specified, PDF processors should behave in accordance
    /// with their own current user preference settings." This program's are the table's.
    #[test]
    fn a_document_that_states_nothing_gets_the_tables_defaults() {
        let doc = document("<< /Type /Catalog /Pages 2 0 R >>");
        assert_eq!(ViewerPreferences::read(&doc), ViewerPreferences::default());
        assert_eq!(
            ViewerPreferences::default().view_area,
            Boundary::Crop,
            "Table 147's default for both view entries"
        );
    }

    /// Every entry Table 147 defines, read at once.
    #[test]
    fn table_147s_entries_are_read() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << \
             /HideToolbar true /HideMenubar true /HideWindowUI true /FitWindow true \
             /CenterWindow true /DisplayDocTitle true /NonFullScreenPageMode /UseOC \
             /Direction /R2L /ViewArea /BleedBox /ViewClip /TrimBox /PrintArea /ArtBox \
             /PrintClip /MediaBox /PrintScaling /None /Duplex /DuplexFlipLongEdge \
             /PickTrayByPDFSize false /PrintPageRange [1 4 9 9] /NumCopies 3 \
             /Enforce [/PrintScaling] >> >>",
        );
        let preferences = ViewerPreferences::read(&doc);
        assert!(preferences.hide_toolbar);
        assert!(preferences.hide_menubar);
        assert!(preferences.hide_window_ui);
        assert!(preferences.fit_window);
        assert!(preferences.center_window);
        assert!(preferences.display_doc_title);
        assert_eq!(
            preferences.non_full_screen_page_mode,
            PageMode::UseOptionalContent
        );
        assert_eq!(preferences.direction, Direction::RightToLeft);
        assert_eq!(preferences.view_area, Boundary::Bleed);
        assert_eq!(preferences.view_clip, Boundary::Trim);
        assert_eq!(preferences.print_area, Boundary::Art);
        assert_eq!(preferences.print_clip, Boundary::Media);
        assert_eq!(preferences.print_scaling, PrintScaling::NoScaling);
        assert_eq!(preferences.duplex, Some(super::Duplex::FlipLongEdge));
        assert_eq!(preferences.pick_tray_by_pdf_size, Some(false));
        assert_eq!(preferences.print_page_range, vec![(1, 4), (9, 9)]);
        assert_eq!(preferences.num_copies, Some(3));
        assert!(preferences.enforce_print_scaling);
    }

    /// "If this entry has an unrecognised value, `AppDefault` shall be used" — and Table 148
    /// then forbids enforcing it, because `/Enforce` may name `/PrintScaling` only where the
    /// dictionary "specifies a valid value other than `AppDefault`".
    #[test]
    fn an_unrecognised_print_scaling_is_app_default_and_cannot_be_enforced() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences \
             << /PrintScaling /Shrink /Enforce [/PrintScaling] >> >>",
        );
        let preferences = ViewerPreferences::read(&doc);
        assert_eq!(preferences.print_scaling, PrintScaling::AppDefault);
        assert!(!preferences.enforce_print_scaling);
    }

    /// An odd-length `/PrintPageRange` states no range rather than half of one.
    #[test]
    fn a_page_range_that_is_not_pairs_states_nothing() {
        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << /PrintPageRange [1 4 9] >> >>",
        );
        assert!(ViewerPreferences::read(&doc).print_page_range.is_empty());
    }

    /// Table 29's two display entries, including the one name §12.2 does not share.
    ///
    /// The last two assertions are what a single shared enum could hide, and they now go opposite
    /// ways. `FullScreen` is Table 29's alone, because it is `/NonFullScreenPageMode`'s own
    /// condition ("meaningful only if the value of the `PageMode` entry in the catalog dictionary
    /// … is `FullScreen`") — a reader that accepted it there would let a document say "when you
    /// leave full screen, go full screen". `UseAttachments` is **both** tables', which the printed
    /// Table 147 does not show and errata issue #275 restores.
    #[test]
    fn table_29s_display_entries_are_not_table_147s() {
        let doc = document("<< /Type /Catalog /Pages 2 0 R >>");
        assert_eq!(
            Opening::read(&doc),
            Opening::default(),
            "the stated defaults"
        );

        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /PageMode /UseAttachments /PageLayout /TwoPageRight >>",
        );
        let opening = Opening::read(&doc);
        assert_eq!(opening.mode, PageMode::UseAttachments);
        assert_eq!(opening.layout, PageLayout::TwoPageRight);

        // A name outside either table is the entry's default, not an error: one corpus catalog
        // writes `/PageMode /Use0`.
        let doc =
            document("<< /Type /Catalog /Pages 2 0 R /PageMode /Use0 /PageLayout /Sideways >>");
        assert_eq!(Opening::read(&doc), Opening::default());

        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /PageMode /FullScreen \
             /ViewerPreferences << /NonFullScreenPageMode /FullScreen >> >>",
        );
        assert_eq!(Opening::read(&doc).mode, PageMode::FullScreen);
        assert_eq!(
            ViewerPreferences::read(&doc).non_full_screen_page_mode,
            PageMode::UseNone,
            "Table 147 does not offer FullScreen, so the entry falls to its default"
        );

        let doc = document(
            "<< /Type /Catalog /Pages 2 0 R /PageMode /FullScreen \
             /ViewerPreferences << /NonFullScreenPageMode /UseAttachments >> >>",
        );
        assert_eq!(
            ViewerPreferences::read(&doc).non_full_screen_page_mode,
            PageMode::UseAttachments,
            "errata issue #275 adds UseAttachments to Table 147's own list"
        );
    }
}
