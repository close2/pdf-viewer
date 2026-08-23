//! What interpreting a page hands back: the display list, the readback, and the reports.
//!
//! The types here are the crate's public answer to "what did this page contain" — the
//! resolved [`Interpretation`] and every statement about what could not be drawn. They are
//! defined together because they are read together: a caller that takes the display list
//! without [`Unsupported`] is exactly the silent-drop failure the module doc of
//! [`super`] forbids.

use pdf_render::DisplayList;
use pdf_syntax::ObjectId;

/// Something the interpreter met but could not draw.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Unsupported {
    /// Text-showing operators were present.
    Text {
        /// How many show operations were skipped.
        operations: usize,
    },
    /// An image `XObject` was drawn.
    Image {
        /// The resource name, for diagnosis.
        name: String,
    },
    /// A shading or pattern was used as paint.
    Shading {
        /// The resource name.
        name: String,
    },
    /// A shading pattern stated §8.7.4.3 Table 77's `/Background` and it was not painted.
    ///
    /// The shading itself is drawn; what is missing is the fill *around* it. Table 77 makes
    /// that a `shall` — "this colour shall be used, before any painting operation involving
    /// the shading, to fill those portions of the area to be painted that lie outside the
    /// bounds of the shading object" — and confines it to a shading used as a pattern, "not
    /// when painted directly with the `sh` operator", which is why this is raised where a
    /// pattern is resolved and never on `sh`.
    ShadingBackground {
        /// The pattern resource name, and how many components the entry states.
        detail: String,
    },
    /// An operator this interpreter does not implement.
    Operator {
        /// The operator, as written.
        operator: String,
    },
    /// A font could not be loaded, so its text was not drawn.
    Font {
        /// Why, from `pdf-font`.
        detail: String,
    },
    /// A page's content stream could not be decoded, so its drawing is missing.
    ///
    /// The page still renders — just without whatever that stream described. Without this
    /// report a page compressed with a filter we do not implement is indistinguishable from
    /// a page the producer meant to leave sparse.
    Content {
        /// What went wrong with which part of `/Contents`.
        issue: crate::page::ContentIssue,
    },
    /// An annotation carried something that could not be drawn.
    ///
    /// Almost always an annotation with no appearance stream, which would have to be
    /// synthesised from its type-specific entries — see `crate::annotation`.
    Annotation {
        /// The subtype and what was wrong with it.
        detail: String,
    },
    /// A bound was reached and interpretation stopped early.
    LimitReached {
        /// Which bound.
        limit: &'static str,
    },
    /// A text object whose glyphs should have knocked one another out (§9.3.8).
    ///
    /// `Tk`'s initial value is true, which makes a text object a non-isolated knockout
    /// transparency group, so a later glyph overwrites an earlier one where they overlap.
    /// This renderer composites each glyph against what is already there — the `Tk` false
    /// model — which differs only where two glyphs of one object overlap under a constant
    /// alpha below one or a blend mode other than Normal. Reported for exactly those
    /// objects; see [`Interpreter::end_text_object`].
    TextKnockout {
        /// How many glyphs the object marked the page with under such a paint.
        glyphs: usize,
    },
    /// One object painted in more than one part, which §11.6.2 says is one element.
    ///
    /// > Portions of an object shall not be composited with one another, even if they are
    /// > described in a way that would seem to cause overlaps (such as a self-intersecting
    /// > path, combined fill and stroke of a path, or a shading pattern containing an overlap
    /// > or fold-over).
    ///
    /// `B` and its three relatives fill *and* stroke one path, and a centred stroke always
    /// covers the edge of what the fill painted. This renderer emits two commands, so under a
    /// paint that composites the overlap is painted twice where the clause asks for once.
    /// Reported for exactly those paths; see [`Interpreter::end_path`]. §9.3.8's text knockout
    /// is the same sentence's exception for a text object, reported separately because its
    /// condition is different.
    CompositedInParts {
        /// What painted the object in parts.
        detail: &'static str,
    },
    /// A graphics state's `/SMask` that could not be evaluated (§11.6.5.1).
    ///
    /// A soft mask *is* implemented — a transparency group evaluated for its alpha (§11.5.2)
    /// or its luminosity (§11.5.3) — so what reaches this is a dictionary the clause does
    /// not describe: no `/G`, a subtype that is neither of the two Table 142 names, or a
    /// `/TR` that is not a readable function. The object is then painted *unmasked*, which
    /// is more ink than the document asked for and never less, and saying so is what keeps
    /// that from passing as a page we drew.
    SoftMask {
        /// What was wrong, and with which resource.
        detail: String,
    },
    /// A transparency group asking for something §11.4 defines and this does not do.
    ///
    /// A `/Group` is composited onto the backdrop Table 145's `/I` names — §11.4.5's
    /// transparency for an isolated one, the page for a non-isolated one — and the result
    /// painted once, under the constant alpha and blend mode in force at `Do`. `/K` reaches
    /// the display list too, for the elements whose shape a backend can draw or state. What
    /// is reported is the residue of each, plus a group blending colour space that is not
    /// the device's, and each only where it can change a pixel. See
    /// [`Interpreter::note_group_departures`] for each condition and the clause it is from.
    TransparencyGroup {
        /// What the group asked for.
        detail: String,
    },
    /// A name a content stream used that §7.8.3's current resource dictionary does not define.
    ///
    /// > A content stream's named resources shall be defined by a resource dictionary, which
    /// > shall enumerate the named resources needed by the operators in the content stream and
    /// > the names by which they can be referred to.
    ///
    /// So this is a **malformed file** rather than an unimplemented clause, and it is here for
    /// the reason trap 5 gives: the page draws less than the producer asked for, and nothing
    /// about the result says which. It is reported by *category* rather than by what the
    /// resource would have been, because a name the file never defines has no subtype, no
    /// dictionary and no size — all that can be said is which of Table 34's subdictionaries was
    /// asked and what it answered.
    ///
    /// A resource whose *definition* is unusable is a different report and stays where it is:
    /// `Font` names a font program that would not parse, `Image` an image whose samples are
    /// malformed, `Shading` a shading that could not be built. This is the case one step
    /// earlier, where there is nothing to try.
    MissingResource {
        /// Table 34's resource category the name was looked up in.
        ///
        /// `XObject`, `Pattern` or `ExtGState` — the three of Table 34's eight that reach the
        /// page through [`Interpreter::resource`]. The other five already say it in their own
        /// words: `Font` reports "no /Font resource named /F1", `Shading` reports "/Sh0 is not
        /// in /Shading", `ColorSpace` reports the space by name, `ProcSet` is deprecated in
        /// PDF 2.0 and names nothing that can be drawn, and a `Properties` list that is missing
        /// costs no mark at all — ISO 32000-2 §8.11.3.2 makes a section optional content "only if
        /// the tag is OC and the dictionary operand is a valid optional content group", so a name
        /// nobody defined leaves ordinary content, and the section's own operators still draw.
        ///
        /// What it does leave behind is silent and is not a mark: §14.9's `/ActualText`, `/Alt`,
        /// `/E` and `/Lang`, §14.7.5.2's `/MCID`, Table 363's artifact entries and §14.13.5's
        /// associated files all arrive through [`Interpreter::property_list`], and a `BDC` naming
        /// a list the resource dictionary does not define loses every one of them. §14.6.2's
        /// ledger row carries that as half of what it owes; this comment named only the two
        /// accessibility entries and the group for as long as the readers for the rest existed,
        /// which is what a list of what is read does when nobody maintains it.
        category: &'static str,
        /// The name, as the content stream wrote it, and what the lookup found instead.
        detail: String,
    },
    /// One of §7.8.2's *other* content streams, drawn as far as its damage.
    ///
    /// §7.8.2 defines what a content stream is, and the definition is the whole argument:
    ///
    /// > A content stream is a PDF stream object whose data consists of a sequence of
    /// > instructions describing the graphical elements to be painted on a page.
    ///
    /// and then names the objects that are one without being a page's `/Contents`:
    ///
    /// > Content streams shall also be used to package sequences of instructions as
    /// > self-contained graphical elements, such as forms (see 8.10, "Form XObjects"), patterns
    /// > (8.7, "Patterns"), certain fonts (9.6.4, "Type 3 fonts"), and annotation appearances
    /// > (12.5.5, "Appearance streams").
    ///
    /// So ADR 0343's reading of a damaged `/Contents` transfers to all five word for word: a
    /// prefix of a sequence of instructions is a shorter sequence of the same kind, made of
    /// bytes the producer's own encoder emitted, and the marks it does not make are simply
    /// absent rather than standing in place of the producer's. That is the opposite of a
    /// damaged *font program*, whose prefix is a table directory pointing at bytes that are not
    /// there — trap 5's substitutive test, which these five pass and that one fails.
    ///
    /// **The tenth place this program reports while drawing**, and trap 5's test is met in both
    /// directions: draw nothing and a form, a pattern cell, a glyph or an annotation vanishes
    /// whole where the file carries most of it; say nothing and the page is
    /// indistinguishable from one whose producer drew less. [`crate::page::ContentIssue::Damaged`]
    /// is the same sentence for the stream Table 31 names — a separate variant because that one
    /// is indexed by its position in `/Contents` and these are reached by resource name.
    /// ADR 0359.
    DamagedContentStream {
        /// Which stream, why it stopped, and how much of it is on the page.
        stream: DamagedStream,
    },
    /// Optional content whose visibility could not be decided, so it was drawn.
    ///
    /// ISO 32000-2 §8.11. Only a visibility expression nested past the interpreter's bound
    /// reaches this: everything else the clause defines has an answer. Drawing is the
    /// deliberate choice of the two ways to be wrong — content that should be hidden is
    /// visible on the page, where content that should be visible would be missing without a
    /// trace — and saying so is what keeps it from being the second kind of failure.
    OptionalContent {
        /// What could not be decided.
        detail: String,
    },
    /// A §10.5 transfer function is inside a composited colour §11.7.5.2 keeps it out of.
    ///
    /// The function itself is implemented — read from Table 57's `/TR2` or `/TR`, or from a
    /// halftone dictionary's `TransferFunction`, and applied on the way to the device to every
    /// component value a mark carries, a shading's samples and an image's included.
    ///
    /// **§11.7.5.2 makes the parameter a property of a *region* rather than of an object.**
    ///
    /// > The halftone and transfer function to be used at any given point on the page shall be
    /// > those in effect at the time of painting the last (topmost) elementary graphics object
    /// > enclosing that point, but only if the object is fully opaque.
    ///
    /// and, closing the same paragraph:
    ///
    /// > For portions of the page whose topmost object is not fully opaque or that are never
    /// > painted at all, the default halftone and transfer function for the page shall be used
    ///
    /// Half of that is a per-object rule wearing a per-region one's clothes, and is implemented:
    /// an object the clause does not call fully opaque is never the one whose function is chosen
    /// at any point, so it is handed the page's default instead —
    /// [`Interpreter::transfer_for_mark`] carries the deduction, the six conditions and the
    /// ancestry they rest on.
    ///
    /// **What is left needs a point, and it is what this reports.** The clause composites raw
    /// colours and maps the result once, per point, with the topmost object's function; this tree
    /// maps each fully opaque contributor's colour before compositing. So where a mark the clause
    /// does not call fully opaque covers an earlier one that *was* fully opaque and carried a
    /// function, the composite this tree builds has that function already inside it and the clause
    /// asks for a composite without it.
    TransferFunction {
        /// Which clause, which condition matched, and what it costs the page.
        detail: String,
    },
    /// The page's ancestry states no usable `/MediaBox`, so its geometry is this program's.
    ///
    /// ISO 32000-2 §7.7.3.3 Table 31 makes the entry "( Required; inheritable )" and §7.7.3.4
    /// says where a required inheritable entry may be written instead:
    ///
    /// > If such an attribute is omitted from a page object, its value shall be inherited from
    /// > an ancestor node in the page tree. If the attribute is a required one, a value shall be
    /// > supplied in an ancestor node.
    ///
    /// So this is a **malformed file**, like [`Self::MissingResource`], and the standard states
    /// no recovery. [`crate::Page::DEFAULT_MEDIA_BOX`] is substituted, which is a choice this
    /// project made rather than a reading it took.
    ///
    /// **The eleventh place this program reports while drawing** (trap 5), and it meets that
    /// test in both directions. Refusing the page would throw away marks nothing else in the
    /// file can supply — the crawl's one witness is a 22-page worksheet that draws perfectly
    /// well — while saying nothing makes a page whose size, and therefore whose every mark's
    /// place on the raster, is a guess indistinguishable from one the producer stated. It is
    /// **not** the additive-or-substitutive question ADR 0106 asks, because what is substituted
    /// is not a mark but the frame every mark is measured in: the same content stream drawn
    /// against a guessed box lands somewhere else, and `1407606.pdf` is 50 points down and
    /// 16 short of its width for exactly this reason.
    ///
    /// One page of the 974, one of `pdfbox`'s 64, three of `format-corpus`' 167 — two of them
    /// built to carry this defect and nothing else — and **one document of the 65 703 crawled
    /// pages that open**. `examples/media_box_census` prints today's. ADR 0389.
    MediaBox {
        /// Which of [`crate::page::MediaBoxSubstitution`]'s three it was, and what stood in.
        detail: String,
    },
    /// Marks stated under a matrix with no inverse (§8.3.4), which paint nothing.
    ///
    /// §8.3.4's third NOTE is the whole of what the standard says about one:
    ///
    /// > Not all transformations are invertible, however. For example, if a matrix contains a,
    /// > b, c, and d elements that are all zero, all user coordinates map to the same device
    /// > coordinates and there is no unique inverse transformation. Such noninvertible
    /// > transformations are not very useful and generally arise from unintended operations,
    /// > such as scaling by 0. Use of a noninvertible matrix when painting graphics objects can
    /// > result in unpredictable behaviour.
    ///
    /// It says the *mark* is undefined and says nothing at all about the page it is on. A page
    /// transform is invertible, so a command under such a matrix has its whole path carried
    /// onto a line or a point: it covers no area at any scale on any device, and all three
    /// backends refuse it by the one rule `pdf_render::paint_space` states.
    ///
    /// **The twelfth place this program reports while drawing** (trap 5), and it is here rather
    /// than in a rasteriser because the condition is a property of the file: it is decidable
    /// from the display list, before any target exists, and every other per-mark refusal in this
    /// tree is named on this enum. Until ADR 0482 each backend answered it fatally and
    /// differently — `render-cpu` and `render-gpu` with an `UnsupportedPaint` raised while
    /// inverting the transform to place a paint, `render-quorra` with an `InvalidStroke` raised
    /// on a width multiplied by a stretch of zero — and any of the three cost the reader the
    /// **whole page**. 293 commands of `4605705.pdf`'s went that way for one `cm` in a damaged
    /// stream.
    ///
    /// What this does *not* claim is that §10.7.4 is satisfied. That subclause's "no shape ever
    /// disappears" would give such a mark the run of whole device pixels its collapsed image
    /// passes through, exactly as `pdf_render::split_collapsed_fill` does for a path that
    /// collapses in its *own* space; a path collapsed by its transform instead is unbuilt and
    /// unwitnessed, and `doc/todo/11` item 8 carries it.
    NoninvertibleMatrix {
        /// How many marking commands the page states under one.
        commands: usize,
    },
    /// Path segments the stream issued with no current point, which state no geometry (§8.5.2.1).
    ///
    /// ISO 32000-2 §8.5.2.1 defines the current point and then makes this an error outright:
    ///
    /// > The trailing endpoint of the segment most recently added to the current path is referred
    /// > to as the current point. If the current path is empty, the current point shall be
    /// > undefined. Most operators that add a segment to the current path start at the current
    /// > point; if the current point is undefined, an error shall be generated.
    ///
    /// The clause states the error and no recovery, and it names no substitute first point — the
    /// paragraph above it says "the first one invoked shall be m or re to begin a new subpath",
    /// so `l`, `c`, `v` and `y` cannot begin one. What this program does is therefore a
    /// **documented choice** and not a reading: the operator adds nothing, and since the current
    /// point is "[t]he trailing endpoint of the segment most recently added", it stays undefined
    /// and the segments after it are refused for the same reason, until an `m` or an `re`. So a
    /// run of segments after the error vanishes whole rather than hanging off a point the file
    /// never stated. See [`crate::content::path::extend_subpath`] for the other two candidates
    /// and why the clause rejects them.
    ///
    /// **The thirteenth place this program reports while drawing** (trap 5), and it is the *error*
    /// the clause asks for: §7.8.2's neighbouring "an error shall occur", for an operator a reader
    /// does not recognise, is raised on this same enum as [`Self::Operator`]. The page keeps every
    /// mark the file did state, and what it loses is named rather than drawn from the corner of
    /// user space, which is where `tiny_skia::PathBuilder` used to begin it.
    ///
    /// **Table 58's `h` is deliberately not counted here, and that asymmetry is the finding.** One
    /// sentence, two costs: `h` on an empty path has no "starting point of the subpath" to close
    /// to either, so it adds no segment on that invocation and
    /// [`crate::content::path::close_subpath`] already pushes nothing — the page is complete, and
    /// saying otherwise would take it out of the oracle's judgement for a mark nobody lost
    /// (trap 11). ADR 0563.
    UndefinedCurrentPoint {
        /// How many `l`, `c`, `v` and `y` operators the page's content streams issued with none.
        segments: usize,
    },
}

/// A content stream that decoded only as far as its damage, on its way to being drawn.
///
/// [`Unsupported::DamagedContentStream`]'s payload as a value of its own, because one of the five
/// places that report it cannot report where it reads: §12.7.4.3's regeneration reads an
/// annotation's appearance stream and hands back a *spliced copy* of those bytes, so what the
/// decode found has to travel from the read to the draw. ADR 0359.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DamagedStream {
    /// Which of §7.8.2's kinds it was, and how the page reached it.
    pub detail: String,
    /// Why the decode stopped short.
    pub damage: pdf_syntax::Damage,
    /// How many bytes did decode, which is what was drawn.
    pub kept: usize,
}

/// The result of interpreting a page.
#[derive(Debug, Clone)]
pub struct Interpretation {
    /// The drawing commands, ready for any backend.
    pub display_list: DisplayList,
    /// Whether this page's marks depend on the magnification it is drawn at.
    ///
    /// True where an annotation sets §12.5.3's `NoZoom`, and only then: everything else in a
    /// display list is resolution-independent by construction, which is what lets a zoom
    /// re-rasterise without re-interpreting. A host that zooms should re-interpret a page this
    /// is set on — `ViewState::set_magnification` is what it would say — and 923 of the corpus's
    /// 974 documents never set it, so the cost falls only where the clause asks for it.
    pub view_dependent: bool,
    /// What could not be drawn. Empty means the page is complete.
    pub unsupported: Vec<Unsupported>,
    /// The page's text, in the order the content stream showed it.
    ///
    /// Produced by the same pass that draws the glyphs, and from the same code-to-glyph
    /// decisions, which is what makes it worth comparing against another extractor: a
    /// difference is evidence about the *rendering*, not about a separate text pipeline
    /// that might be wrong in its own way.
    ///
    /// This is reading order as the producer wrote it, which is not always visual order.
    /// It carries no layout analysis and does not try to reconstruct columns.
    pub text: String,
    /// How many glyphs *marked the page*.
    ///
    /// Deliberately not the length of [`Self::text`], which is a different question with a
    /// different answer: a font with no `/ToUnicode` and no glyph names the Adobe Glyph
    /// List knows draws perfectly good glyphs that nothing can name, so the readback is
    /// empty for a page that is nothing but text. The reference comparison chooses a page's
    /// tolerance from this, because what decides whether a difference is glyph hinting or a
    /// misplaced shape is whether glyphs were drawn — not whether we could say which ones.
    ///
    /// Counts a glyph that was filled, stroked or run as a Type 3 description, so text in
    /// rendering mode 3 or 7 (invisible, and clip-only) contributes nothing, and neither
    /// does a glyph on a hidden optional-content layer. It is a count rather than a flag
    /// because "a page with three glyphs on it" and "a page of text" are different pages.
    pub glyphs: usize,
    /// Codes this page showed that reached no glyph at all, over every font it used.
    ///
    /// The complement of [`Self::glyphs`] for a `Font::Program`: a code whose font resolved it
    /// to nothing, excluding the two cases that are not marks missed — a code that reads back
    /// as whitespace, which is *meant* to have no outline, and a code §9.10.2 gave a character
    /// that the substitute face lacks, which `Unsupported::Font` names on its own.
    ///
    /// **A count rather than a report, and that is the whole point of it.** ADR 0152 measured
    /// the alternative: naming every uncovered code named 13 documents that mostly draw fine,
    /// and every report costs the oracle a judged page (trap 11), so the tree reports a font
    /// that drew *nothing* and stays quiet about one that drew most of its codes.
    /// `doc/todo/21-font-substitution.md` asks whether that trade still holds, and the input to
    /// that question is a number rather than an opinion — which is this field, summed over the
    /// corpus by `pdf-model/tests/corpus.rs`.
    ///
    /// **It counts a mark missed and not a mark absent**, which are different things and were
    /// one number until the four-hundred-and-thirty-fourth session: a code that reached a glyph
    /// the program *contains* and that program describes as empty is in
    /// [`Self::codes_reaching_a_blank_glyph`] instead. ADR 0270 has the split.
    ///
    /// **And a third exclusion sat in the code and not in this sentence for two hundred
    /// sessions**: a code §9.10.2 could not *name* was excluded as well, so a route that ended
    /// at no glyph at all went uncounted whenever the readback happened to be empty. That is
    /// the wrong question — whether the program answered is decided by the glyph, and needs no
    /// character — and `issue17333.pdf` is the page it cost: one `Tj`, one code, an embedded
    /// subset, §9.6.5.4's algorithm terminating with nothing, a wholly blank sheet, and every
    /// counter that measures the picture reading zero. Corrected in the
    /// six-hundred-and-eighty-fifth session; ADR 0520.
    pub codes_without_a_glyph: usize,
    /// Codes this page showed that reached a glyph the font program describes as empty, and
    /// that §9.10.2 gave a character.
    ///
    /// The other half of the branch [`Self::codes_without_a_glyph`] counts, separated because
    /// only one of the two is a mark the reader loses. §9.6.5.4's and §9.7.4.2's routes ended
    /// at a glyph the program contains, and that glyph has no contours — which is how every
    /// sfnt stores a space, and how a subset stores a character it was asked to carry and had
    /// nothing to draw for. Drawing nothing is what the font says to do.
    ///
    /// It is a separate count rather than nothing at all because the whitespace exemption in
    /// front of it is blind to a font whose `/ToUnicode` reads its own space back as something
    /// else — `pr12564.pdf`'s 26 codes read back as `#`, and the web's largest single
    /// contributor reads its space back as U+0007 — so this is where such a code lands, and
    /// leaving it uncounted would have made the correction unmeasurable.
    ///
    /// **The named-character condition is this half's and not the other's**, which is the shape
    /// the six-hundred-and-eighty-fifth session separated (ADR 0520). Here the program has
    /// answered — it contains the glyph and describes it as empty — so the only thing left to
    /// lose is the *reading*, and §9.10.2's silence about a code is not evidence that one was
    /// lost. [`Self::codes_without_a_glyph`] asks the opposite question and therefore does not
    /// wait for a character.
    pub codes_reaching_a_blank_glyph: usize,
    /// Codes this page showed that §9.10.2 could not name, whether or not a glyph was drawn.
    ///
    /// The reading half of what the two counts above measure for the drawing half, and the one
    /// the tree had no channel for at all: §9.10.2 ends "there is no way to determine what the
    /// character code represents", and a page whose fonts are in that position drew its text
    /// perfectly and handed back nothing, with nothing anywhere saying which of the two had
    /// happened. `french_diacritics.pdf` is the sharpest case — a pdfTeX Type 3 font whose
    /// `/Differences` names the Latin-1 accented letters `/a192`, `/a224` … , which is the
    /// producer's own label and not a name the clause's second method can resolve. This is that
    /// refusal saying what it is. `doc/todo/21` §5 has the reading and ADR 0311 the argument.
    ///
    /// **Not a report**, for ADR 0152's reason: every report costs the oracle a judged page
    /// (trap 11), and this is a shortfall in the readback rather than in the picture. A host
    /// that searches or selects can read it and say so; the oracle never sees it.
    ///
    /// Counts a code the *page* showed. A code shown inside a Type 3 glyph description is how
    /// that glyph is painted rather than text of the page (§9.6.4), and is not counted here.
    ///
    /// **Split by cause since the four-hundred-and-eighty-third session**, because the total on
    /// its own could not say whether a reader lost a code to a question the standard leaves
    /// unanswerable or to a route this program does not walk. [`UnnamedCodes::total`] is what
    /// this field used to be.
    pub codes_without_a_character: UnnamedCodes,
    /// ISO 32000-2 §14.9's accessibility spans over [`Self::text`], in the order they closed.
    ///
    /// One entry per marked-content sequence stating an `/Alt`, an `/E` or a `/Lang`, in
    /// either of the two places §14.9 puts them — the sequence's own property list or the
    /// structure element a `/MCID` names. Empty for every page whose producer tagged nothing,
    /// which is most of them.
    ///
    /// Kept as spans rather than as a second string because they answer a different question
    /// from [`Self::text`]: that is what a person copying the page gets, and this is what a
    /// text-to-speech engine gets. [`Self::speech`] combines the two.
    pub described: Vec<crate::accessibility::Described>,
    /// §14.8.2.2's artifact spans over [`Self::text`], in the order they closed.
    ///
    /// One entry per marked-content sequence tagged `/Artifact`, with whatever Table 363's
    /// property list stated about it. **Nothing is removed from the text**: the clause leaves
    /// what to do with an artifact to the consumer — "[a] text-to-speech engine, for instance,
    /// may decide not to speak running heads or page numbers" — so this says which ranges a
    /// consumer *may* drop and drops none of them itself.
    ///
    /// 30 of the 953 corpus first pages mark at least one.
    pub artifacts: Vec<ArtifactSpan>,
    /// How many word or line separators in [`Self::text`] were inferred from glyph positions.
    ///
    /// §14.8.2.6.2 requires a *tagged* document to state them: "any white-space characters that
    /// would be present to separate words in a pure text representation shall be present in the
    /// tagged PDF representation of the text", and its NOTE 1 draws the consequence — "the PDF
    /// processor can determine word breaks without having to rely on heuristics based on
    /// information such as glyph positioning on the page".
    ///
    /// This reader infers them anyway, because the clause binds documents and most documents are
    /// not tagged. The count is what makes that a measurement rather than an assumption: a
    /// conforming tagged page should need none, and `tests/logical_order.rs` says how many
    /// actually do.
    pub inferred_separators: usize,
    /// §14.7.5.2's marked-content sequences over [`Self::text`], in the order they closed.
    ///
    /// One entry per `BDC` … `EMC` whose property list stated an `/MCID`, which is the
    /// identifier §14.7.5.2 uses to tie page content to a structure element: "[t]he marked-content
    /// sequence … shall be identified by an integer marked-content identifier". Recorded because
    /// §14.8.2.5.1 defines a *second* order over the same content — the logical one, which is a
    /// depth-first walk of the structure tree — and turning one into the other needs to know
    /// which bytes of the readback belong to which sequence.
    ///
    /// Nothing is reordered here. [`crate::structure::Tree::logical_text`] is what puts the
    /// spans in the structure's order, and it is a *different* string from [`Self::text`] on
    /// exactly the pages where the two orders do not coincide.
    pub marked: Vec<MarkedSpan>,
    /// §14.13.5's associated files, each with the range of [`Self::text`] its section covered.
    ///
    /// One entry per file per `/AF`-tagged marked-content sequence. The clause's own example is
    /// a `MathML` version of an equation associated with the form `XObject` that draws it, which
    /// is a statement about *this* content and not about the document — the document-wide ones
    /// are `attachment::associated` on the catalog, and the page's on the page.
    pub associated_files: Vec<(std::ops::Range<usize>, crate::attachment::Attachment)>,
    /// The document catalog's `/Lang`, §14.9.2.3's default for everything in the file.
    pub language: Option<String>,
    /// Where each code's readback sits on the page, in the order it was shown.
    ///
    /// The geometry [`Self::text`] does not have. One entry per character code the page showed,
    /// carrying the range of [`Self::text`] that code produced and the quadrilateral its glyph
    /// occupies — so a point on the page finds a position in the text, and a range of the text
    /// finds the shapes to draw over.
    ///
    /// **Nothing in ISO 32000-2 asks for this.** Selecting text is not a thing the standard
    /// describes; what it describes is where a glyph is drawn (§9.4.4's text rendering matrix)
    /// and how tall the font's glyphs are (Table 120's `/Ascent` and `/Descent`), and this is
    /// those two, per code, kept rather than discarded.
    ///
    /// Includes text in rendering modes 3 and 7 — the invisible ones — because that is exactly
    /// the OCR layer under a scanned page, and it is the text a person most wants to select.
    /// Excludes text on an optional-content layer that is switched off, which is not on the page.
    pub text_layer: Vec<Placed>,
}

/// One character code's readback, and where its glyph sits on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    /// The range of [`Interpretation::text`] this code produced.
    ///
    /// May be empty: a code whose glyph is drawn but that no `/ToUnicode`, glyph name or
    /// `cmap` can name reads back as nothing, and it still occupies space on the page.
    pub span: std::ops::Range<usize>,
    /// The glyph's box, in the display list's own coordinates.
    ///
    /// `[x0, y0, x1, y1, x2, y2, x3, y3]`, going round the quadrilateral: the corners are
    /// (0, descent), (advance, descent), (advance, ascent) and (0, ascent) in glyph space,
    /// mapped through §9.4.4's text rendering matrix and the page's own transform. A
    /// quadrilateral rather than a rectangle because the text matrix may rotate, shear or
    /// mirror, and a rectangle could only describe the cases where it does not.
    ///
    /// The same space the display list is in, so a consumer that knows what scale a page was
    /// rasterised at knows where these are on the screen.
    pub quad: [f32; 8],
}

/// One §14.8.2.2 artifact, and the range of [`Interpretation::text`] it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactSpan {
    /// Where the artifact's own readback sits in [`Interpretation::text`].
    ///
    /// Empty for the common artifact, which is a rule or a background block and reads back
    /// nothing at all — the span is still recorded, because "this page has a decorative
    /// artifact" is a different statement from "this page has none".
    pub range: std::ops::Range<usize>,
    /// What Table 363 said about it.
    pub artifact: crate::structure::Artifact,
}

/// Codes a page showed that ISO 32000-2 §9.10.2 could not name, by which method could have.
///
/// [`Interpretation::codes_without_a_character`] was one number when ADR 0311 added it, and a
/// number is where this question stops being answerable: a code no method named can be the
/// unanswerable question the clause states in its own words — "there is no way to determine what
/// the character code represents" — or a route this program does not walk, and the two have
/// different consequences. Each field is a [`pdf_font::NamingGap`] variant, which carries the
/// reading behind it; the sum is [`Self::total`], and `examples/unnamed_code_census` is what reads
/// the split over a corpus. ADR 0318, which used it to close one of the six.
///
/// **A count and never a report**, on ADR 0152's trade, exactly as the single number was: a
/// shortfall in the readback is not a shortfall in the picture, and a report on each of the
/// documents behind it would cost the oracle that many judged pages — most of which draw
/// perfectly.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnnamedCodes {
    /// [`pdf_font::NamingGap::EmptyMapping`]: a mapping whose answer is no characters.
    pub empty_mapping: usize,
    /// [`pdf_font::NamingGap::IncompleteToUnicode`]: a `/ToUnicode` that omits the code.
    pub incomplete_to_unicode: usize,
    /// [`pdf_font::NamingGap::UnlistedName`]: a glyph name neither published list holds.
    pub unlisted_name: usize,
    /// [`pdf_font::NamingGap::UnnamedCid`]: a registered collection with nothing for the CID.
    pub unnamed_cid: usize,
    /// [`pdf_font::NamingGap::UnaddressableCid`]: an `Identity` ordering and no `/ToUnicode`.
    pub unaddressable_cid: usize,
    /// [`pdf_font::NamingGap::UnnamedGlyph`]: a glyph selected by code that nothing names.
    pub unnamed_glyph: usize,
}

impl UnnamedCodes {
    /// Every code counted here, which is what [`Interpretation::codes_without_a_character`] was
    /// before the split.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.empty_mapping
            .saturating_add(self.incomplete_to_unicode)
            .saturating_add(self.unlisted_name)
            .saturating_add(self.unnamed_cid)
            .saturating_add(self.unaddressable_cid)
            .saturating_add(self.unnamed_glyph)
    }

    /// Adds another tally to this one, cause by cause.
    ///
    /// Destructured rather than summed field by field from `self` so that a variant added to
    /// [`pdf_font::NamingGap`] — and therefore a field added here — fails to compile instead of
    /// being dropped from every total that uses this.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        let Self {
            empty_mapping,
            incomplete_to_unicode,
            unlisted_name,
            unnamed_cid,
            unaddressable_cid,
            unnamed_glyph,
        } = other;
        Self {
            empty_mapping: self.empty_mapping.saturating_add(empty_mapping),
            incomplete_to_unicode: self
                .incomplete_to_unicode
                .saturating_add(incomplete_to_unicode),
            unlisted_name: self.unlisted_name.saturating_add(unlisted_name),
            unnamed_cid: self.unnamed_cid.saturating_add(unnamed_cid),
            unaddressable_cid: self.unaddressable_cid.saturating_add(unaddressable_cid),
            unnamed_glyph: self.unnamed_glyph.saturating_add(unnamed_glyph),
        }
    }

    /// Counts one code against the method that could have named it.
    pub(super) fn count(&mut self, gap: &pdf_font::NamingGap) {
        let slot = match gap {
            pdf_font::NamingGap::EmptyMapping => &mut self.empty_mapping,
            pdf_font::NamingGap::IncompleteToUnicode => &mut self.incomplete_to_unicode,
            pdf_font::NamingGap::UnlistedName(_) => &mut self.unlisted_name,
            pdf_font::NamingGap::UnnamedCid => &mut self.unnamed_cid,
            pdf_font::NamingGap::UnaddressableCid => &mut self.unaddressable_cid,
            pdf_font::NamingGap::UnnamedGlyph => &mut self.unnamed_glyph,
        };
        *slot = slot.saturating_add(1);
    }
}

/// Which content stream a §14.7.5.2 marked-content sequence was read out of.
///
/// ISO 32000-2 §14.7.5.2 makes the identifier unique in one stream and no wider:
///
/// > The marked-content sequence shall contain a property list (see 14.6.2, "Property lists")
/// > containing an MCID entry, which shall be an integer marked-content identifier that
/// > uniquely identifies the marked-content sequence within its content stream
///
/// and Errata Collection 3's Issue #308 (`/State` `Review`/`Completed`) adds §14.7.5.4 a NOTE 2
/// saying the consequence outright: identifiers are scoped by content stream and must start at
/// zero, so the same one may reappear across pages or form `XObject`s. In prose rather than as a
/// blockquote because the sentence is an *addition* and `doc/md/` is a conversion of the base text
/// — the same rule `run.rs` follows for Issue #368, and `spec-errata emit` is where it is read.
///
/// So an identifier alone does not name a sequence, and a page whose `/Contents` and whose form
/// `XObject` both number from zero has two different sequences called `/MCID 0`.
///
/// This is the other half of the key, on the same division Table 357's `/Stm` makes: the page's
/// own content stream, or a stream that entry can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ContentStream {
    /// The page's own `/Contents` (§7.7.3.3 Table 31).
    ///
    /// One stream however many parts it has: §7.8.2 makes the parts of an array "a single
    /// stream", and §14.7.5.4 keys the whole of it by the *page object's* `/StructParents`.
    #[default]
    Page,
    /// A stream Table 357's `/Stm` names — a form `XObject` (§8.10) or an appearance stream
    /// (§12.5.5) — by the object it is.
    Object(ObjectId),
    /// A stream nothing can name: one written as a direct object, or one this program
    /// constructed for an annotation that stated no appearance (§12.7.4.3).
    ///
    /// Table 357 requires `/Stm` to be an indirect reference, so no marked-content reference can
    /// point at such a stream and no structure element can claim a sequence inside it. A sequence
    /// here therefore matches nothing, which is the honest answer rather than a gap: the file has
    /// no way to say the sequence is its.
    Unnameable,
}

impl ContentStream {
    /// Whether Table 357's `/Stm` — absent, or naming an object — names this stream.
    ///
    /// ISO 32000-2 §14.7.5.2, of that entry:
    ///
    /// > This entry should be present only if the marked-content sequence resides in a content
    /// > stream other than the content stream for the page (see 8.10, "Form XObjects" and 12.5.5,
    /// > "Appearance streams"). If this entry is absent, the marked-content sequence shall be
    /// > contained in the content stream of the page identified by Pg
    ///
    /// so an absent `/Stm` names [`Self::Page`] and nothing else.
    #[must_use]
    pub fn named_by(self, stm: Option<ObjectId>) -> bool {
        match (self, stm) {
            (Self::Page, None) => true,
            (Self::Object(is), Some(named)) => is == named,
            _ => false,
        }
    }
}

/// The sequences of `spans` that a structure element's content item names.
///
/// The whole of §14.7.5.2's rule, in one place, because three answers rest on it — the text range
/// a structure element covers, §14.8.3.3's content rectangle, and §14.8.2.5's logical order — and
/// two of them used to key on the identifier alone.
///
/// `mcid` and `stm` are what Table 355's `/K` states: the identifier, and Table 357's `/Stm` where
/// the item is a marked-content reference that names a stream. Both halves are needed, because the
/// identifier "uniquely identifies the marked-content sequence within its content stream" and no
/// further.
///
/// # The one recovery, and what it is for
///
/// An absent `/Stm` is not silence. §14.7.5.2 makes it a statement about the file:
///
/// > If this entry is absent, the marked-content sequence shall be contained in the content stream
/// > of the page identified by Pg
///
/// A file whose sequence is in a form `XObject` and whose `/K` states a bare integer has broken
/// that `shall`. Two of the corpus's 153 tagged documents do exactly that — every sequence inside
/// one form, every content item a bare integer, no `/Stm` and no `/StructParents` anywhere — and
/// read strictly they say nothing at all to a screen reader.
///
/// So where the page's own content stream holds **no** sequence with that identifier, and exactly
/// one other stream does, that one is answered instead. The condition is what makes this a reading
/// rather than a guess: there is one sequence on the page the file could mean, the clause it broke
/// leaves no other candidate, and the moment two streams carry the identifier the answer is empty
/// again rather than both.
///
/// **What it costs is written down**: a file that both breaks the `shall` and repeats an identifier
/// across two streams gets nothing where it used to get something wrong, and a file that breaks it
/// once gets an attribution this crate inferred rather than one the document stated. Neither is
/// visible to a caller, which is the honest limit — there is no channel here for a shortfall, for
/// [`Interpretation::codes_without_a_character`]'s reason (ADR 0152).
#[must_use]
pub fn named_sequences(spans: &[MarkedSpan], mcid: i64, stm: Option<ObjectId>) -> Vec<&MarkedSpan> {
    let exact: Vec<&MarkedSpan> = spans
        .iter()
        .filter(|span| span.mcid == mcid && span.stream.named_by(stm))
        .collect();
    if !exact.is_empty() || stm.is_some() {
        return exact;
    }
    // The clause's `shall` is broken: nothing in the page's own stream carries this identifier.
    // One stream carrying it is a reading; two is an ambiguity, and an ambiguity answers nothing.
    let carried: Vec<&MarkedSpan> = spans.iter().filter(|span| span.mcid == mcid).collect();
    let one_stream = carried
        .first()
        .is_some_and(|first| carried.iter().all(|span| span.stream == first.stream));
    if one_stream { carried } else { Vec::new() }
}

/// One §14.7.5.2 marked-content sequence's extent in a page's readback, and on the page.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkedSpan {
    /// The `/MCID` the sequence's property list stated.
    ///
    /// **Not a key on its own** — see [`Self::stream`], which is the rest of it.
    pub mcid: i64,
    /// The content stream the sequence was read out of, which [`Self::mcid`] is only unique
    /// within (§14.7.5.2).
    pub stream: ContentStream,
    /// Where its text sits in [`Interpretation::text`], in bytes.
    ///
    /// Empty for a sequence that showed no text, which is most of them on a page of graphics —
    /// and an empty range is still recorded, because "this element's content drew nothing
    /// readable" and "this element has no content" are different statements.
    pub range: std::ops::Range<usize>,
    /// Where the sequence's marks are, in the display list's own coordinates: `[x0, y0, x1, y1]`.
    ///
    /// §14.8.3.3 gives every block- and inline-level structure element a content rectangle
    /// "derived from the shape of the enclosed content", and §14.8.5.4.5 states that derivation
    /// for the cases that are marks rather than layout — a table cell's rectangle is "determined
    /// from the bounding box of all graphics objects in the cell's content". This is that union,
    /// taken as the sequence's commands were emitted; [`crate::content::Interpreter::draw`] has
    /// the argument and what it costs.
    ///
    /// **A derived extent and never a stated one.** Table 379's `/BBox` is what a *producer* says
    /// about an element, and it reaches an assistive technology by a route of its own
    /// (`viewer_core::AccessibilityNode::bounds`); this is what the page turned out to draw, which
    /// is the same kind of fact as [`Interpretation::text_layer`]'s quadrilaterals and is carried
    /// beside rather than merged with the other.
    ///
    /// `None` for a sequence whose content marked nothing — an empty `BDC` … `EMC`, or one whose
    /// every command was clipped away — which is an answer rather than a failure: the page has
    /// drawn nothing there, and inventing a rectangle would turn that into a place.
    ///
    /// The same space [`Placed::quad`] is in, so a consumer that maps one maps the other.
    pub drawn: Option<[f32; 4]>,
}

impl Interpretation {
    /// The page as §14.9 says it should be vocalised: [`Self::text`] with the descriptions,
    /// expansions and languages applied.
    ///
    /// Built on demand rather than during interpretation, because nothing that draws a page
    /// needs it and the great majority of pages state nothing for it to do.
    #[must_use]
    pub fn speech(&self) -> Vec<crate::accessibility::Spoken> {
        crate::accessibility::speech(&self.text, &self.described, self.language.as_deref())
    }

    /// Returns `true` if everything on the page was drawn.
    ///
    /// The harness uses this to decide whether a page may be compared against a reference
    /// renderer at all: comparing a page we knowingly rendered incompletely would report a
    /// difference that is not a defect.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.unsupported.is_empty()
    }

    /// The three per-code counts, as one value a consumer outside this crate can hold.
    ///
    /// See [`Shortfall`] for why they travel together.
    #[must_use]
    pub const fn shortfall(&self) -> Shortfall {
        Shortfall {
            unnamed: self.codes_without_a_character,
            without_a_glyph: self.codes_without_a_glyph,
            reaching_a_blank_glyph: self.codes_reaching_a_blank_glyph,
        }
    }
}

/// What a page's codes cost the reader, in the three ways this tree can tell apart.
///
/// [`Interpretation`] carries the three as separate fields because they are counted at three
/// different places; they are one type here because a **consumer** of them cannot use one without
/// the others. Two of them are about the picture and one is about the readback, and the
/// distinction between the first two is the whole of ADR 0270 — a code that reached a glyph the
/// program describes as empty is a space and not a loss, so a host told only
/// [`Self::without_a_glyph`] would be reading a fraction of a fraction. `Answer::Field` and
/// `Answer::Fields` carry one type for the same reason: a host must not be able to learn an
/// exception from one question and miss it in the other.
///
/// **None of the three is an [`Unsupported`] report, and that is a decision rather than an
/// omission.** ADR 0152's arithmetic prices one: a page that reports leaves the oracle's judged
/// set, and these are shortfalls on pages that otherwise draw perfectly — 41 of the 892 corpus
/// page ones that report nothing show a code §9.10.2 could not name. So the number crosses to a
/// host and to `pdf-retrieve` instead, where a reader who searched, selected or extracted can be
/// told that what came back is short and by how much. ADR 0422.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Shortfall {
    /// Codes ISO 32000-2 §9.10.2 could not name, by which of its methods could have.
    ///
    /// [`Interpretation::codes_without_a_character`].
    pub unnamed: UnnamedCodes,
    /// Codes that reached no glyph at all, which is a mark the reader loses.
    ///
    /// [`Interpretation::codes_without_a_glyph`].
    pub without_a_glyph: usize,
    /// Codes that reached a glyph the font program describes as empty, which a space is.
    ///
    /// [`Interpretation::codes_reaching_a_blank_glyph`]. Carried beside the one above rather than
    /// folded into it because only the other is a loss; ADR 0270 measured what folding them cost.
    pub reaching_a_blank_glyph: usize,
}

impl Shortfall {
    /// Whether every code this page showed was named and drawn.
    ///
    /// [`Self::reaching_a_blank_glyph`] does **not** make this false: a glyph the program
    /// describes as empty is the program saying the code makes no mark, which is what a space is.
    #[must_use]
    pub const fn is_whole(&self) -> bool {
        self.unnamed.total() == 0 && self.without_a_glyph == 0
    }

    /// Adds another page's counts to these, field by field.
    ///
    /// Written out rather than derived for the reason `examples/unnamed_code_census`'s own sum is:
    /// a field added later must fail to compile here rather than be dropped in silence.
    #[must_use]
    pub const fn merge(self, other: Self) -> Self {
        let Self {
            unnamed,
            without_a_glyph,
            reaching_a_blank_glyph,
        } = other;
        Self {
            unnamed: self.unnamed.merge(unnamed),
            without_a_glyph: self.without_a_glyph.saturating_add(without_a_glyph),
            reaching_a_blank_glyph: self
                .reaching_a_blank_glyph
                .saturating_add(reaching_a_blank_glyph),
        }
    }
}
