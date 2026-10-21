//! Annex O's fragment identifier: what a URI after `#` asks of the document, and what is refused.
//!
//! Every parameter this program does not carry out is named in the returned sentences rather than
//! dropped.

use pdf_model::Pages;
use pdf_model::action::ImportData;
use pdf_model::fragment::{Fragment, Parameter};

use super::{Highlighted, Open, OpeningFile, ZOOM_RANGE};

impl Open {
    /// Carries out ISO 32000-2 Annex O's fragment identifier, and names what it could not.
    ///
    /// The returned strings are for a person: every parameter this program refuses is refused *by
    /// name*, because a URI that says `#page=5` and opens page one has told its reader something
    /// false about the document. Nothing here fails and nothing here refuses to open the file.
    ///
    /// **In the fragment's own order**, which §O.2 makes a requirement rather than a convenience:
    ///
    /// > Fragment identifiers shall be processed and (if required) executed from left to right as
    /// > they appear in the character string that makes up the fragment identifier.
    ///
    /// and the `comment` parameter is what turns that into behaviour — it names an annotation on
    /// *the page selected so far*, so `page=3&comment=x` and `comment=x&page=3` mean different
    /// things and the annex says so in a NOTE.
    ///
    /// Called immediately after Table 29's `/OpenAction`, which is where §O.2.2 puts it: these
    /// "should be processed immediately after any other document-specified open parameters have
    /// been processed". A page and a view stated by both is the document's opinion overruled by
    /// the URI's, which is the order that makes a fragment identifier useful.
    ///
    /// # Where the coordinates are
    ///
    /// §O.2.2 states one rule for its own three rectangles and the rule has two halves that pull
    /// apart:
    ///
    /// > All coordinate values … shall be expressed in the default user space coordinate system.
    ///
    /// while each parameter's own row measures from "the top left corner of the page". Default
    /// user space has its origin at the *bottom* left (§8.3.2.3), so the two sentences together
    /// say: the *units* are default user space's, and the origin is the page's top-left corner.
    /// That is the reading this takes — `top` counts downward from `display_box`'s upper edge and
    /// `left` rightward from its left one — and it is the only one under which both sentences are
    /// true at once. `display_box` rather than the media box because §12.2's `/ViewArea` decides
    /// what "the page" is on a screen, and it is the crop box for every document that states none.
    ///
    /// `view`, alone among them, is *not* measured that way: its arguments "shall correspond to
    /// those found in 12.3.2.2", which are default user space's own, origin and all.
    pub(crate) fn apply_fragment(&mut self, fragment: &Fragment) -> Vec<String> {
        let mut notes = Vec::new();
        for parameter in &fragment.parameters {
            if let Some(reason) = parameter.unhonoured() {
                notes.push(format!(
                    "this URI's fragment asks for `{}`, which this program does not do: {reason}",
                    parameter.name()
                ));
                continue;
            }
            self.apply_parameter(parameter, &mut notes);
            // "Any remaining parameters after this parameter apply to the selected embedded
            // file." So everything after `ef` is about a document this program has not opened,
            // and applying it to *this* one would take a person somewhere the URI did not name.
            // `Fragment::parse` has already stopped reading there and kept the remainder whole;
            // it travels with the file as `Event::Extracted`'s fragment, for the host that opens
            // those bytes as a document to hand straight back as `Command::Open`'s (ADR 0431).
            // The loop still stops, because a `Fragment` this crate did not parse could carry
            // parameters here and applying them would be the wrong document's instruction.
            if matches!(parameter, Parameter::EmbeddedFile(_)) {
                if let Some(rest) = fragment.after_embedded_file.clone() {
                    notes.push(format!(
                        "and the fragment continues `{rest}` after it, which applies to that \
                         embedded file rather than to this document and travels with it"
                    ));
                    if let Some(file) = self.opening_file.as_mut() {
                        file.fragment = Some(rest);
                    }
                }
                break;
            }
        }
        for unread in &fragment.unread {
            notes.push(format!(
                "this URI's fragment says `{}`, which is {}",
                unread.name,
                unread.reason.as_str()
            ));
        }
        notes
    }

    /// One of Annex O's parameters, applied. Split from [`Self::apply_fragment`] for its length.
    fn apply_parameter(&mut self, parameter: &Parameter, notes: &mut Vec<String>) {
        match parameter {
            // Table Annex O.3: "The first page in the document has a pageNum value of 1", against
            // this crate's own zero-based index.
            Parameter::Page(number) => {
                let index = usize::try_from(number.saturating_sub(1)).unwrap_or(usize::MAX);
                if index < self.page_count {
                    self.go_to(index);
                } else {
                    notes.push(format!(
                        "this URI's fragment asks for page {number} and this document has {}",
                        self.page_count
                    ));
                }
            }
            Parameter::NamedDestination(name) => self.go_to_named(name, notes),
            Parameter::StructureElement(id) => self.go_to_structure_element(id, notes),
            // Table 166's `/NM`, on the page chosen so far — which is what makes this parameter's
            // position in the fragment matter.
            Parameter::Comment(name) => {
                let found = self.page(self.page_index).and_then(|page| {
                    pdf_model::fragment::annotation_named(&self.document, &page, name)
                });
                if let Some(id) = found {
                    // "[T]he PDF processor shall open the document with the specified comment
                    // selected", and the annex says nothing about what selecting one looks like.
                    // This program has one way of indicating an annotation without changing the
                    // page — §12.5.1's focus, which is what the tab key moves and what a host
                    // draws a ring around — so that is what a selected comment is here. A choice,
                    // and written down as one.
                    self.focus = Some(id);
                } else {
                    notes.push(format!(
                        "this URI's fragment names the comment {}, which page {} does not carry",
                        text(name),
                        self.page_index.saturating_add(1)
                    ));
                }
            }
            // Table Annex O.4's `zoom` is Table 149's `/XYZ` in every respect but two: it states
            // a percentage where `/XYZ` states a factor, and it measures its corner from the top
            // left of the page where `/XYZ` measures from default user space's own origin.
            Parameter::Zoom { percent, offset } => {
                // "[T]he percentage to which the document should be zoomed" is a `should`, and a
                // magnification outside `ZOOM_RANGE` is the one a person could not reach either:
                // the view lands on the nearest bound and the fragment is told so rather than
                // magnified to a number the page then does not show (ADR 1523).
                let factor = percent / 100.0;
                if factor > 0.0 && !(ZOOM_RANGE.0..=ZOOM_RANGE.1).contains(&factor) {
                    notes.push(format!(
                        "this URI's fragment asks for a zoom of {percent}%, and this reader \
                         magnifies between {}% and {}%, so the page opens at the nearer of the two",
                        ZOOM_RANGE.0 * 100.0,
                        ZOOM_RANGE.1 * 100.0
                    ));
                }
                let corner = offset.map(|(left, top)| self.in_default_user_space(left, top));
                self.pending_views.push(pdf_model::destination::View::Xyz {
                    left: corner.map(|(left, _)| left),
                    top: corner.map(|(_, top)| top),
                    // Table 149 makes a zoom of zero the same as none, and this is the same
                    // magnification stated as a percentage, so zero says "leave it alone" here
                    // too rather than magnifying a page to nothing.
                    zoom: Some(factor).filter(|zoom| *zoom > 0.0),
                });
            }
            Parameter::View(view) => self.pending_views.push(*view),
            // "Open the document with the specified window view rectangle", which is what Table
            // 149's `/FitR` already means — "magnified just enough to fit the rectangle" — so the
            // rectangle is converted to that clause's corners and nothing new is invented.
            Parameter::ViewRect {
                left,
                top,
                width,
                height,
            } => {
                let (x, y) = self.in_default_user_space(*left, *top);
                self.pending_views.push(pdf_model::destination::View::FitR {
                    rect: [x, y - height, x + width, y],
                });
            }
            // Table Annex O.4: "Open the document and search for one or more words, selecting the
            // first matching word in the document."
            //
            // **Started here and finished by the host**, which is the same division every other
            // unit of work in this crate takes: `Event::NeedsRender` is a page this crate has
            // interpreted and not drawn, and this is a search it has planned and not run. The
            // alternative would be reading up to 1023 pages before `Command::Open` returns —
            // 5.84 s on the standard's own file, on the launch path, which `CLAUDE.md`'s startup
            // rules forbid outright.
            //
            // "In the document", so it begins at the first page and does not wrap: a match before
            // where the fragment's other parameters left the view is still the first one in the
            // document, and stopping at the end is what makes "first" mean anything.
            Parameter::Search(words) => {
                let needles: Vec<String> = words
                    .iter()
                    // Lossy for the reason [`text`] is: the annex gives its byte strings no
                    // character encoding, and a needle is compared against a readback that is
                    // already Unicode. A word whose bytes are not UTF-8 searches for what a
                    // person would see if they typed it.
                    .map(|word| String::from_utf8_lossy(word).into_owned())
                    .collect();
                let spelt = needles.join(" ");
                match crate::search::Searching::new(
                    needles,
                    crate::command::FindDirection::Forward,
                    (0, 0),
                    self.page_count,
                    false,
                ) {
                    Some(searching) => {
                        notes.push(format!(
                            "this URI's fragment asks for a search for `{spelt}`, which is \
                             running: {} page(s) to read",
                            searching.remaining
                        ));
                        self.searching = Some(searching);
                    }
                    None => notes.push(format!(
                        "this URI's fragment asks for a search for `{spelt}`, which names nothing \
                         to look for in a document of {} page(s)",
                        self.page_count
                    )),
                }
            }
            // Table Annex O.3, and the annex's one `shall` on a processor that is not about where
            // the window points:
            //
            // > When used as part of a PDF open parameter, the PDF processor shall open the
            // > embedded file contained within the EmbeddedFiles name tree identified by name .
            //
            // §7.11.4's bytes are inside the document, so nothing is fetched; what *opening* one
            // means is the host's, and the annex says so itself two sentences later —
            // "[s]ecurity should be strongly considered when opening an embedded file … a PDF
            // processor may choose to prompt the user or even prevent opening of the file", which
            // is a `should` and a `may` addressed to whoever faces the person. So this records the
            // name and `Viewer::open` hands the bytes over as `Event::Extracted`, where
            // `Command::Extract` and §12.5.6.15's annotation already put them (ADR 0310).
            //
            // Lossy for the reason [`text`] is, and for the same comparison `annotation_named`
            // makes: the annex gives its byte string no character encoding and §7.7.4's tree key
            // states one, so the match is against §7.9.2's *decoded* text string as UTF-8.
            Parameter::EmbeddedFile(name) => {
                self.opening_file = Some(OpeningFile {
                    name: String::from_utf8_lossy(name).into_owned(),
                    // Filled by [`Self::apply_fragment`], which is the only place the *rest* of
                    // the fragment is in scope.
                    fragment: None,
                });
            }
            // Table Annex O.4's `highlight` and `fdf`, each in a method of its own for the
            // reason `nameddest` and `structelem` are: the reading is longer than the arm.
            Parameter::Highlight {
                left,
                right,
                top,
                bottom,
            } => self.highlight(*left, *right, *top, *bottom),
            Parameter::Fdf(uri) => self.import_from(uri, notes),
        }
    }

    /// Annex O's `highlight`, kept until a host asks where it is on the screen.
    ///
    /// Table Annex O.4: "Open the document with the specified rectangle highlighted. Each argument
    /// shall be an integer or floating point value representing the rectangle measured from the top
    /// left corner of the page. The nature of the highlighting is implementation-dependent."
    ///
    /// **Not the selection**, and the annex is what says so: the two neighbouring parameters that
    /// point a person at something both say *selected* — `comment`'s "with the specified comment
    /// selected", `search`'s "selecting the first matching word" — and this one says *highlighted*,
    /// with a nature left to the processor that the other two are not given. So it is a shape of
    /// its own, kept here and answered by `Query::Highlight`; what it then looks like is the
    /// host's, which is exactly what "implementation-dependent" leaves open and what this boundary
    /// already does with every other quadrilateral it hands over (ADR 0357).
    ///
    /// The corners go through the same conversion `zoom` and `viewrect` take, because the clause
    /// states one rule for all three: default user space's units with the page's top-left corner
    /// as the origin.
    fn highlight(&mut self, left: f32, right: f32, top: f32, bottom: f32) {
        let (x0, y0) = self.in_default_user_space(left, top);
        let (x1, y1) = self.in_default_user_space(right, bottom);
        // Normalised, because the annex names four edges and states no order between them: a URI
        // that writes its bottom above its top has still named a rectangle.
        self.highlights.push(Highlighted {
            page: self.page_index,
            rect: [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)],
        });
    }

    /// Annex O's `fdf`: §12.7.8's form data, asked of the host that owns the filesystem.
    ///
    /// Table Annex O.4: "Open the document and then import the data from the specified FDF or XFDF
    /// file. The URI shall be either a relative or absolute URI to an FDF or XFDF file."
    ///
    /// **The same request §12.7.6.4's action makes**, and it is made the same way: this crate has
    /// no filesystem (`doc/ui-boundary.md`'s rule 2), so the name crosses as `Event::NeedsFile`
    /// with the purpose the bytes are wanted for and a host resolves it — against the document's
    /// own URI for a relative one, and by whatever policy it has for an absolute one. That division
    /// is the annex's too: §O.1 defines these as a *URI's* parameters, and resolving a URI
    /// reference is the business of whoever holds the base.
    ///
    /// The format is the file name's, read once for both clauses by
    /// [`pdf_model::action::data_format`]. The annex names both formats and this program reads
    /// both, exactly as `crate::interact::request_file` accepts both for the action.
    fn import_from(&mut self, uri: &[u8], notes: &mut Vec<String>) {
        use pdf_model::action::DataFormat;

        // Lossy for the reason [`text`] is: the annex states no character encoding for the
        // argument, and what a host resolves is a name a person typed.
        let file = String::from_utf8_lossy(uri).into_owned();
        let format = pdf_model::action::data_format(&file);
        if matches!(format, DataFormat::Fdf | DataFormat::Xfdf) {
            notes.push(format!(
                "this URI's fragment asks for the form data in {file}, which the host is being \
                 asked for"
            ));
            self.importing = Some(ImportData { file, format });
        } else {
            notes.push(format!(
                "this URI's fragment asks for the form data in {file}, which is neither \
                 §12.7.8's FDF nor ISO 19444-1's XFDF, and no other data format is read"
            ));
        }
    }

    /// Annex O's `nameddest`, through §12.3.2.4's lookup.
    ///
    /// Both of the tables that clause defines are asked, in the order it introduces them, which is
    /// what `Destination::read` does for a name or a string alike.
    fn go_to_named(&mut self, name: &[u8], notes: &mut Vec<String>) {
        let destination = pdf_model::destination::Destination::read(
            &self.document,
            &pdf_syntax::Object::String(name.into()),
        );
        let Some(destination) = destination else {
            notes.push(format!(
                "this URI's fragment names the destination {}, which this document does not \
                 define",
                text(name)
            ));
            return;
        };
        let index = {
            let pages = Pages::new(&self.document);
            destination.page_index(&self.document, &pages)
        };
        if let Some(index) = index.filter(|index| *index < self.page_count) {
            self.go_to(index);
        } else {
            // A destination naming a page in another file (§12.3.2.2's remote form) or naming
            // nothing. Its view still stands: it says how to look at whatever page is showing.
            notes.push(format!(
                "this URI's fragment names the destination {}, which does not name a page of \
                 this document",
                text(name)
            ));
        }
        self.pending_views.push(destination.view);
    }

    /// Annex O's `structelem`: §14.7.2's `/IDTree` for the element, §12.3.2.3's algorithm for its
    /// page.
    fn go_to_structure_element(&mut self, id: &[u8], notes: &mut Vec<String>) {
        let index = pdf_model::structure::Tree::of(&self.document)
            .and_then(|tree| tree.element_by_id(&self.document, id))
            .and_then(|element| {
                let pages = Pages::new(&self.document);
                pdf_model::destination::structure_element_page(&self.document, &element, &pages)
            });
        if let Some(index) = index.filter(|index| *index < self.page_count) {
            self.go_to(index);
            return;
        }
        // The annex states this outcome rather than leaving it open: "[i]f no content is
        // contained within the hierarchy of the structure element or structID does not match a
        // structure element, the first page in the document shall be identified." So page one is
        // the answer and not a failure — said out loud because a person who typed an identifier
        // wants to know that it matched nothing.
        self.go_to(0);
        notes.push(format!(
            "this URI's fragment names the structure element {}, which identifies no page here, \
             so the first page is shown",
            text(id)
        ));
    }

    /// Shows a page named by a fragment, with the state a page turn resets.
    fn go_to(&mut self, index: usize) {
        if index == self.page_index {
            return;
        }
        self.page_index = index;
        self.scroll = (0.0, 0.0);
        self.shown_for = 0.0;
    }

    /// Annex O's corner — measured from the page's top left — as a point in default user space.
    ///
    /// The page's own upper-left corner is `display_box`'s left edge and its *top* edge, and the
    /// axis runs downward from there while default user space's runs up.
    fn in_default_user_space(&self, left: f32, top: f32) -> (f32, f32) {
        let box_ = self
            .page(self.page_index)
            .map_or([0.0, 0.0, 0.0, 0.0], |page| page.display_box);
        (box_[0] + left, box_[3] - top)
    }
}

/// A byte string out of a URI's fragment, worded for a person to read.
///
/// Lossy, and this is the one place that is right: Annex O gives its byte strings no character
/// encoding — which is why `pdf_model::fragment` keeps them as bytes — and a sentence somebody
/// reads is where a guess costs nothing, while a comparison against the document's own bytes is
/// where it would cost everything.
fn text(bytes: &[u8]) -> String {
    format!("`{}`", String::from_utf8_lossy(bytes))
}
