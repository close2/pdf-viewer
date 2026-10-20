//! §11.4.7 and §11.7.2: the press a page or a group of four components composites in, and the
//! `ICCBased` CMYK space `DeviceCMYK` is redefined to be inside it (ADR 2080).

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Name, Object};

use crate::colour::{ColourSpace, Press, Presses};
use crate::icc::Rendering;
use crate::page::Page;

use crate::content::Interpreter;
use crate::content::colour::output_intent_space;

use super::TransparencyGroup;

/// What §11.4.7's page group asks a page to composite in.
#[derive(Debug, Clone)]
pub(in crate::content) enum PagePress {
    /// The device's three components: no page group, or one whose space is a three-component
    /// RGB space, which is what this raster already holds.
    Device,
    /// Four components, and this is whose.
    In(Arc<Press>),
    /// Four components this page cannot be drawn in, and this says why.
    Beyond(BeyondPress),
}

/// Why four components were not sampled into a press.
///
/// **Every reason here is the document's**, which it was not until ADR 0417: one of them used
/// to be [`crate::colour::MAX_PRESSES`] being spent by other files, so the same page carried a
/// different verdict depending on what the process had opened first, and ADR 0416 had to give
/// a caller a way to tell the two apart. The budget is the interpretation's now, so the
/// distinction has nothing left to separate and the sentence is a fact about the file again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::content) struct BeyondPress {
    /// The sentence the report carries.
    pub(in crate::content) why: &'static str,
}

impl BeyondPress {
    /// A refusal the document decides: the same answer on every run and on every machine.
    pub(super) const fn stated(why: &'static str) -> Self {
        Self { why }
    }
}

/// The press §11.4.7's page group composites in, and where its four components come from.
///
/// > That initial colour space shall serve as the default blending colour space for each page,
/// > unless the page explicitly specifies an alternative default by means of its page
/// > dictionary containing a Group key that contains a CS key whose value represents a
/// > different colour space from the initial blending colour space.
///
/// Three routes reach a press, and Annex P — informative, and the standard's own algorithm for
/// this question — puts them in this order. A device blending space "first appl\[ies] the
/// default colour space mechanism" (§8.6.5.6's `/DefaultCMYK`, whose value "shall be used as
/// the colour space for the operation currently being performed"); a page group with no parent
/// otherwise inherits "from the output device, or from the output intent" (§14.11.5), which is
/// also §8.6.5.7 NOTE 3's "an output intent dictionary, if present, can suggest such a
/// calibration"; and a `/CS` that is itself a four-component `ICCBased` space names the press
/// outright, which is §11.7.2's paragraph about `DeviceCMYK` being redefined inside the group.
///
/// Where none of the three names one, the four components are the assumed inks of ADR 0263 and
/// this is [`crate::colour::assumed_press`], which no budget applies to because no file asked
/// for it. Where one names a space this tree cannot sample — four components that are not an ICC
/// profile, or a press past this *page's* [`crate::colour::MAX_PRESSES`] —
/// [`PagePress::Beyond`] carries the reason into the report.
pub(in crate::content) fn page_press(
    document: &Document,
    page: &Page,
    presses: &Presses,
) -> PagePress {
    let attributes = document.get_key(&page.dict, "Group");
    let Some(attributes) = attributes.as_dict() else {
        return PagePress::Device;
    };
    // §8.10.3 Table 94's `/S`, as [`page_blending_space`] reads it.
    if document
        .get_key(attributes, "S")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Transparency")
    {
        return PagePress::Device;
    }
    let entry = document.get_key(attributes, "CS");
    // §14.11.5's array is resolved here, where the page is, rather than inside `named_press`:
    // the clause puts one in a page dictionary as well as in the catalog and makes the page's
    // the one that "shall be used", and the tail of this walk no longer holds a page.
    let intent = output_intent_space(document, Some(&page.dict));
    // §11.7.5.3's second bullet fixes the conversion out of a group's space at "the time the
    // `Do` operator is applied to the group", and §11.4.7's page group has no `Do`: nothing
    // has run, so the parameters are the initial graphics state's — Table 51's
    // `RelativeColorimetric` with §8.6.5.9's `/UseBlackPtComp` at its `Default`, which this
    // processor compensates. That is `Rendering::compensating()` exactly.
    press_for_entry(
        document,
        &entry,
        &page.resources,
        presses,
        intent.as_ref(),
        Rendering::compensating(),
    )
}

/// The `ICCBased` CMYK space §11.7.2 redefines `DeviceCMYK` to be inside a page or an isolated
/// group whose `/CS` parsed as `parsed`, or `None` where that entry states none and the
/// redefinition in force outside the group is the one inside it.
///
/// > If an isolated transparency group or page has an ICCBased 'CMYK' colour space ,
/// > DeviceCMYK shall be redefined within the transparency group to be the same as the
/// > blending colour space
///
/// Two entries state such a space. A four-component `ICCBased` one does outright. A
/// `/DeviceCMYK` one does where Table 145's remapping makes it one — "Device colour spaces
/// shall be subject to remapping according to the DefaultGray , DefaultRGB , and DefaultCMYK
/// entries in the ColorSpace subdictionary of the current resource dictionary" — through a
/// `/DefaultCMYK` in `resources`, read as [`named_press`] reads *current*; Annex P puts that
/// remapping before any search of the ancestors.
///
/// Every other entry states none, and the ancestors decide, by two sentences of the same
/// clause. A `/DeviceCMYK` group unremapped is the first: "If the colour space of the
/// transparency group is a device colour space, and some ancestor of the group has a CIE-based
/// colour space with the same number of colourants, then the colour space of this group shall be
/// the CIE-based space of the nearest such ancestor." A group of another count — an `ICCBased`
/// RGB one, say — is the fifth: a graphics object's device space, where "the current group or an
/// ancestor of the current group is defined with a CIE-based colour space with the same number
/// of colourants", is "for compositing purposes only" that of "the nearest such ancestor", so a
/// `k` inside an RGB group inside a CMYK one is still the CMYK group's. ADR 2012.
#[expect(
    clippy::doc_markdown,
    reason = "the comment quotes §11.7.2 and Table 145 verbatim, and a quotation is not marked up"
)]
///
/// `parse` reads the entry as `ColourSpace::parse` does against no resources, and is asked only
/// where [`may_state_four_components`] holds — so a group or page of an RGB or grey profile, the
/// commonest blending spaces there are, has no profile read for this question.
pub(super) fn stated_cmyk_group(
    document: &Document,
    entry: &Object,
    resources: &Dictionary,
    parse: impl FnOnce(&Object) -> Option<ColourSpace>,
) -> Option<ColourSpace> {
    if !may_state_four_components(document, entry) {
        return None;
    }
    match parse(entry)? {
        space @ ColourSpace::Icc { .. } if space.components() == 4 => Some(space),
        ColourSpace::Cmyk => match ColourSpace::parse(
            document,
            &Object::Name(Name::new(b"DeviceCMYK".to_vec())),
            resources,
        ) {
            Some(space @ ColourSpace::Icc { .. }) if space.components() == 4 => Some(space),
            _ => None,
        },
        _ => None,
    }
}

/// Whether a page or group `/CS` entry can be a space of four components, read off the entry
/// without its profile: a device CMYK family by name or in an array, `CalCMYK` — which §8.6.5.1
/// has a reader render as `DeviceCMYK` — or an `ICCBased` array whose stream's `/N`, which
/// Table 65 requires, is 4 or is not stated.
fn may_state_four_components(document: &Document, entry: &Object) -> bool {
    let entry = document.resolve(entry);
    if let Some(name) = entry.as_name() {
        return matches!(name.as_bytes(), b"DeviceCMYK" | b"CMYK");
    }
    let Some(items) = entry.as_array() else {
        return false;
    };
    let family = items.first().map(|family| document.resolve(family));
    match family
        .as_ref()
        .and_then(Object::as_name)
        .map(Name::as_bytes)
    {
        Some(b"DeviceCMYK" | b"CMYK" | b"CalCMYK") => true,
        Some(b"ICCBased") => items
            .get(1)
            .map(|stream| document.resolve(stream))
            .as_ref()
            .and_then(Object::as_stream)
            .is_some_and(|stream| {
                document
                    .get_key(&stream.dict, "N")
                    .as_integer()
                    .is_none_or(|count| count == 4)
            }),
        _ => false,
    }
}

/// The space §11.7.2 redefines `DeviceCMYK` to be on `page`, from §11.4.7's page group, which
/// "is effectively isolated regardless of the I value" (Table 145) and has no ancestor: the
/// starting value of `Interpreter::cmyk_group`. [`stated_cmyk_group`] says which entries
/// state one.
pub(in crate::content) fn page_cmyk_group(document: &Document, page: &Page) -> Option<ColourSpace> {
    let attributes = document.get_key(&page.dict, "Group");
    let attributes = attributes.as_dict()?;
    // §8.10.3 Table 94's `/S`, as [`page_blending_space`] reads it.
    if document
        .get_key(attributes, "S")
        .as_name()
        .is_none_or(|name| name.as_bytes() != b"Transparency")
    {
        return None;
    }
    let entry = document.get_key(attributes, "CS");
    stated_cmyk_group(document, &entry, &page.resources, |entry| {
        ColourSpace::parse(document, entry, &Dictionary::new())
    })
}

/// The press a group or page `/CS` entry names, resolved against `resources`.
///
/// The tail of [`page_press`], split out because a *group's* `/CS` asks the same question one
/// scope down (§11.7.2 redefines `DeviceCMYK` "within the transparency group" for an
/// `ICCBased` CMYK space, and Table 145 subjects a device space here to §8.6.5.6's
/// remapping) — with the
/// resource dictionary in force at the group rather than the page's.
fn press_for_entry(
    document: &Document,
    entry: &Object,
    resources: &Dictionary,
    presses: &Presses,
    intent: Option<&ColourSpace>,
    rendering: Rendering,
) -> PagePress {
    match ColourSpace::parse(document, entry, &Dictionary::new()) {
        Some(ColourSpace::Cmyk) => named_press(document, resources, presses, intent, rendering),
        Some(ColourSpace::Icc { profile }) if profile.channels() == 4 => {
            press_or_beyond(&profile, presses, rendering)
        }
        // A four-component space that is not a profile — a `DeviceN` of four inks, say — names
        // components this tree has no conversion out of, so it keeps its report.
        Some(space) if space.components() == 4 => PagePress::Beyond(BeyondPress::stated(
            "its four components are named by a space this tree cannot sample (§11.7.2), \
             so there is no conversion out of them",
        )),
        _ => PagePress::Device,
    }
}

/// The press a `/DeviceCMYK` group or page's four components belong to.
///
/// `/DefaultCMYK` first, because §8.6.5.6 says "shall" about the operation being performed
/// while §8.6.5.7 NOTE 3 says an output intent "can suggest" a calibration — the nearer and
/// stronger statement wins, which is the ranking ADR 0009 recorded for a colour on its way to a
/// pixel and this is the same ranking one clause up.
///
/// **`resources` is the group `XObject`'s own, and the standard says which only under an
/// erratum.** Table 145 subjects a group's device colour space to remapping through the
/// `/DefaultGray`, `/DefaultRGB` and `/DefaultCMYK` entries of *the current* resource
/// dictionary's `/ColorSpace` subdictionary (§11.6.6, §8.6.5.6), and at a `Do` the dictionary
/// current in the ordinary sense is the **parent's** — the group's content stream has not
/// begun. Errata Collection 3's Issue #134 settles it by naming the transparency group
/// `XObject`'s own, which is what `xobject.rs` has always passed here as `form_resources`. It
/// is a `Caret` with no `StrikeOut`, so `spec-errata check` cannot see it by construction and
/// `emit` is what found it; `doc/errata-read.md` has the reading and ADR 0551 the argument.
fn named_press(
    document: &Document,
    resources: &Dictionary,
    presses: &Presses,
    intent: Option<&ColourSpace>,
    rendering: Rendering,
) -> PagePress {
    // `None` cannot happen for a literal device name — `ColourSpace::by_name` falls back on the
    // device space when a `/DefaultCMYK` will not parse — and it is grouped with the plain
    // answer because a space that did not parse names no press.
    match ColourSpace::parse(
        document,
        &Object::Name(Name::new(b"DeviceCMYK".to_vec())),
        resources,
    ) {
        Some(ColourSpace::Cmyk) | None => {}
        Some(ColourSpace::Icc { profile }) if profile.channels() == 4 => {
            return press_or_beyond(&profile, presses, rendering);
        }
        Some(space) if space.components() == 4 => {
            return PagePress::Beyond(BeyondPress::stated(
                "its /DefaultCMYK names a space this tree cannot sample (§8.6.5.6), so there \
                 is no conversion out of its four components",
            ));
        }
        Some(_) => {}
    }
    match intent {
        Some(ColourSpace::Icc { profile }) if profile.channels() == 4 => {
            press_or_beyond(profile, presses, rendering)
        }
        _ => PagePress::In(crate::colour::assumed_press()),
    }
}

/// A profile sampled into a press, or the reason this page is not drawn in it.
///
/// The refusal is [`crate::colour::MAX_PRESSES`], and since ADR 0417 it is this page's own: a
/// page naming a ninth distinct press is refused the ninth on every run and on every machine,
/// where before the ninth press of the *process* was refused and which page that fell on
/// depended on the order the scheduler ran the others in.
fn press_or_beyond(
    profile: &crate::icc::Profile,
    presses: &Presses,
    rendering: Rendering,
) -> PagePress {
    presses.press_for_profile(profile, rendering).map_or(
        PagePress::Beyond(BeyondPress::stated(
            "it is the ninth distinct press this page names and eight is the budget \
             (§11.7.2), so its four components are not converted out",
        )),
        PagePress::In,
    )
}

impl Interpreter<'_> {
    /// The press an isolated group's own `/CS` asks its elements to composite in, where
    /// this run can honour it (§11.6.6, §11.7.2).
    ///
    /// `None` is "composite in what the parent composites in", and it is the answer unless
    /// **all** of the following hold, each for a stated reason:
    ///
    /// - **The group is isolated.** §11.6.6: "\[f]or non-isolated groups, or if no group
    ///   colour space is specified, the group colour space shall be inherited from the
    ///   parent group or page."
    /// - **The parent's space is one this group's result can be converted into**, which is
    ///   [`Interpreter::group_compositing`]'s condition and not this function's: it asks
    ///   this only where the parent composites on the device or where the group changes
    ///   the space in force and [`super::into_parent::parent_channels`] has a conversion into the parent's.
    /// - **The `/CS` names four components this tree can sample** — `/DeviceCMYK` through
    ///   §8.6.5.6's and §14.11.5's ranking, or a four-component `ICCBased` profile
    ///   (§11.7.2). Table 145 subjects a device space here to remapping through the
    ///   `DefaultCMYK` entry of the *current* resource dictionary's `ColorSpace`
    ///   subdictionary, which is why the group's resources are consulted rather than the
    ///   page's — and Errata Collection 3's Issue #134 is what makes that reading of
    ///   *current* the clause's rather than this tree's (see [`named_press`]).
    /// - **No figure is supplying this content's colour from outside it** (§8.6.8's
    ///   `uncoloured`): the marks inside carry a colour resolved for the *parent's*
    ///   compositing, and reinterpreting them in ink would convert a colour that was never
    ///   stated here.
    ///
    /// # A knockout group used to be a fourth condition
    ///
    /// §11.4.6's rewrites edit the element list after the runs, and this function refused a
    /// knockout group outright because editing one half of a pair would leave the other
    /// describing a different construction. What that never asked is whether the *same* edit
    /// can be made to both halves, and it can: the two lists are one content stream
    /// interpreted twice with only the colours resolved differently, so
    /// [`super::knockout::knockout_construction`] applies its chosen rewrite to the pair's black half as well
    /// and commits neither unless the results still [`super::knockout::paired`]. The backdrop §11.4.6
    /// composites each element against needs no conversion of its own here, because a group
    /// reaching this function is isolated and the clause says what such a group's is: "\[a]n
    /// isolated knockout group composites the element with a transparent backdrop." ADR 1103.
    ///
    /// # Why a shading pattern is no condition here, and the uncoloured cell is
    ///
    /// A group a *shading pattern* is carried into over the `Do` is not a condition: §11.6.7 makes
    /// the pattern's definition a non-isolated group, §11.7.2 gives such a group the space of "the
    /// nearest ancestor isolated parent group", and `Interpreter::shading_paint` builds a shading
    /// pattern's colours at the mark (ADR 0483), in whatever this run is compositing into, so a
    /// pattern painted inside a press is resolved in that press's half like every other colour
    /// there.
    ///
    /// **[`Interpreter::uncoloured`] stays, and the difference is which way the colour travels.**
    /// §8.6.8's uncoloured cell takes its colour from the `scn` *outside* — "the pattern's
    /// content stream shall not specify any colour" — so there is nothing inside the group to
    /// rebuild from; the tint is a resolved [`pdf_render::Color`] and reinterpreting it as ink
    /// would state one the file never did. A shading pattern carries its whole definition, which
    /// is exactly what made the rebuild possible.
    pub(super) fn group_press(
        &mut self,
        group: &TransparencyGroup,
        resources: &Dictionary,
        rendering: Rendering,
    ) -> Option<Arc<Press>> {
        if !group.isolated || self.uncoloured {
            return None;
        }
        // The page's own §14.11.5 intent, resolved once when this interpretation began — a
        // group inside a page is drawn under the intent in force for that page — and borrowed
        // rather than cloned, because cloning it would copy an ICC profile per group.
        match press_for_entry(
            self.document,
            &group.colour_space,
            resources,
            self.presses,
            self.output_intent.as_ref(),
            rendering,
        ) {
            PagePress::In(press) => Some(press),
            // A group whose press this page has no budget left for is drawn in the parent's
            // space and keeps §11.6.6's report, which is the answer that was right before this
            // construction existed and is still right.
            PagePress::Device | PagePress::Beyond(_) => None,
        }
    }
}
