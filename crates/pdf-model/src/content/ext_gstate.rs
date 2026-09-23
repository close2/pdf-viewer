//! `gs` and Table 57: the graphics state parameter dictionary, entry by entry.
//!
//! §10.5's transfer function lives here too, because Table 57's `/TR`, `/TR2` and `/HT` are the
//! three ways a file states one — the clause's own two bullets, the second of which reaches into a
//! halftone dictionary and is why this module reads §10.6.5 for one entry and nothing else.

use std::sync::Arc;

use pdf_render::BlendMode;
use pdf_syntax::{Dictionary, Document, Object};

use super::colour::{BlackPoint, Intent};
use super::report::Unsupported;
use super::run::{name_at, narrow};
use super::{GraphicsState, Interpreter, apply_dash, line_cap, line_join, miter_limit};
use crate::black_generation::BlackGeneration;

pub(crate) use crate::transfer::{Stated, Transfer};

/// What a halftone dictionary said about **one** component's transfer function.
///
/// Three answers for the same reason [`Stated`] has three, one component down: §10.6.5's tables
/// make `TransferFunction` optional and let its value be the name `Identity`, and those are
/// different instructions. A halftone that says nothing leaves the graphics state's `/TR` in force
/// for that component; one that names `Identity` **overrides it with the identity function**, which
/// turns it off. Folding the two together would let a `/TR` survive an `/HT` written to stop it.
#[derive(Debug, Clone, Default)]
enum Component {
    /// The halftone dictionary has no `TransferFunction` for this component.
    #[default]
    Unsaid,
    /// It names `Identity`: "[t]he name Identity may be used to specify the identity function."
    Identity,
    /// It carries a function.
    Function(Arc<crate::function::Function>),
}

/// ISO 32000-2 §10.5's transfer function: its two sources, and what the device is handed.
///
/// The clause states two, and states which wins:
///
/// > - The current transfer function parameter in the graphics state shall consist of either a
/// >   single transfer function or an array of four separate transfer functions …
/// > - The current halftone parameter in the graphics state may specify transfer functions as
/// >   optional entries in halftone dictionaries (see 10.6.5, "Halftone dictionaries"). … A
/// >   transfer function specified in a halftone dictionary shall override the corresponding one
/// >   specified by the current transfer function parameter in the graphics state.
///
/// **Why a screen owes the second bullet, when §10.6 is `inapplicable` here.** A halftone
/// dictionary holds two unrelated things: a *screen* — a frequency, an angle, a spot function, a
/// threshold array — and a `TransferFunction`. §10.1's list of rendering steps makes only the first
/// conditional on the device: "[i]f the raster output device supports PDF-defined halftoning, apply
/// halftoning according to 10.6", against an unconditional "[f]or any object for which transfer
/// functions are in effect, apply those transfer functions". §10.6.1 then says outright what a
/// device needing no screen still owes: "[h]alftoning is not required for such devices; **after
/// gamma correction by the transfer functions**, the colour components shall be transmitted
/// directly to the device." So the screen is inapplicable and the entry beside it is not — the
/// dictionary is the *carrier*, and this module reads that one entry out of it and nothing else.
///
/// **Both are graphics state parameters** (Table 52's `halftone` and `transfer`), so both are saved
/// and restored by `q`/`Q`, and either can be set without the other. They are therefore kept apart
/// and composed, rather than folded at the `gs`: a later `/TR /Identity` must clear the first bullet
/// without touching the second, and a later `/HT /Default` the reverse.
#[derive(Debug, Clone, Default)]
pub(super) struct TransferState {
    /// Table 57's `/TR` or `/TR2`.
    stated: Option<Arc<Transfer>>,
    /// The current halftone's `TransferFunction`, per component, red then green then blue.
    halftone: [Component; 3],
    /// The two composed, which is what every mark on the page is coloured through.
    ///
    /// Derived, never assigned: [`TransferState::compose`] is the only writer, and it runs
    /// whenever either source moves. Kept rather than computed at each mark because a page has one
    /// `gs` per state and many marks under it.
    effective: Option<Arc<Transfer>>,
}

impl TransferState {
    /// What the file has stated, for the one caller entitled to ask.
    ///
    /// **That caller is [`super::Interpreter::transfer_for_mark`] and no other**, because what a
    /// mark is painted with is §11.7.5.2's choice between this and the page's default rather than
    /// this on its own. There used to be a second accessor here — `in_force`, handing out a
    /// `&Transfer` for a caller that applied it immediately — and every one of its callers was a
    /// place making that choice by omission.
    pub(super) fn shared(&self) -> Option<&Arc<Transfer>> {
        self.effective.as_ref()
    }

    /// Composes the two sources, the halftone's overriding "the corresponding one" per component.
    ///
    /// A state in which no component is mapped is `None` rather than three identities, which keeps
    /// every "is a transfer function in force" question — the shading cache's, §11.7.5.2's report's
    /// — asking what it means to ask.
    fn compose(&mut self) {
        let channels: [Option<Arc<crate::function::Function>>; 3] =
            std::array::from_fn(|index| match &self.halftone[index] {
                Component::Unsaid => self
                    .stated
                    .as_ref()
                    .and_then(|stated| stated.channel(index)),
                Component::Identity => None,
                Component::Function(function) => Some(Arc::clone(function)),
            });
        self.effective = channels
            .iter()
            .any(Option::is_some)
            .then(|| Arc::new(Transfer::from_channels(channels)));
    }
}

/// Table 57's `/HT`, read for §10.5's second bullet and for nothing else.
///
/// `None` where the entry says nothing this reader can act on, which leaves the halftone in force.
///
/// The entry is "dictionary, stream, or name", and the three shapes are three different answers:
///
/// - **the name `/Default`**, which Table 57 defines as "the halftone that was in effect at the
///   start of the page". Table 52 says what that is — "a PDF reader shall initialise this to a
///   suitable device dependent value" — so it is *this device's* halftone, and a device halftone is
///   not a halftone dictionary in the file and specifies no `TransferFunction`. `/HT /Default`
///   therefore takes an earlier `/HT`'s override off and reveals whatever `/TR` states, which is
///   [`Component::Unsaid`] in all three components rather than `None` here.
/// - **a Type 5 dictionary**, whose keys are colourant names: each component takes its own entry's
///   `TransferFunction`, and a component with no entry of its own takes `/Default`'s, which Table
///   132 requires. This device's native colour space is `DeviceRGB`, and §10.6.5.6 names its
///   primaries — "Red , Green , and Blue for `DeviceRGB`".
/// - **any other dictionary or stream**, whose single `TransferFunction` applies to every
///   component. §10.6.5.6 used to say so in a sentence contrasting the two uses of a component
///   dictionary, and **erratum #311 strikes that whole paragraph**, so the reason is the one that
///   survives it: Table 52's parameter is "[a] halftone screen for gray and colour rendering", one
///   per graphics state, and §10.6.5.6 opens by saying Type 5 is what exists for the other case —
///   "[s]ome devices, particularly colour printers, require separate halftones for each individual
///   colourant". A halftone that is not Type 5 governs the rendering rather than a colourant, so
///   "the same component" in Tables 128 to 131 is every one of them.
///
/// `HalftoneName` needs no case of its own and the reason is the clause's: "[i]f there is no
/// `HalftoneName` entry, **or if the requested halftone name does not exist on the device**, the
/// halftone's parameters may be defined by the other entries in the dictionary". This device has no
/// halftones of its own to name, so the second branch is always the one taken.
fn halftone_transfer(document: &Document, state: &Dictionary) -> Option<[Component; 3]> {
    let entry = document.get_key(state, "HT");
    if entry.is_null() {
        return None;
    }
    if let Some(name) = entry.as_name() {
        // Table 57 gives this entry exactly one name. Any other is a halftone nobody defined, and
        // leaving the parameter alone is what an unreadable state means everywhere else here.
        return (name.as_bytes() == b"Default").then(Component::default_all);
    }
    let halftone = match &entry {
        Object::Dictionary(dict) => dict.clone(),
        Object::Stream(stream) => stream.dict.clone(),
        _ => return None,
    };
    if document.get_key(&halftone, "HalftoneType").as_integer() == Some(5) {
        // "Its keys shall be name objects representing the names of individual colourants or
        // colour components", and "Default" is "[a] halftone that shall be used for any colourant
        // or colour component that does not have an entry of its own".
        let default = document.get_key(&halftone, "Default");
        return Some(std::array::from_fn(|index| {
            let colourant = ["Red", "Green", "Blue"][index];
            let component = match document.get_key(&halftone, colourant) {
                Object::Null => document.resolve(&default),
                named => named,
            };
            component_transfer(document, &component)
        }));
    }
    let one = component_transfer(document, &entry);
    Some([one.clone(), one.clone(), one])
}

/// One halftone dictionary's `TransferFunction`, for the component or components it governs.
///
/// ISO 32000-2 §10.6.5.2, Table 128, and Tables 129 to 131 say the same in the same words:
///
/// > ( Optional ) A transfer function, which overrides the current transfer function in the
/// > graphics state for the same component.
///
/// > The name Identity may be used to specify the identity function.
fn component_transfer(document: &Document, halftone: &Object) -> Component {
    let dict = match halftone {
        Object::Dictionary(dict) => dict,
        Object::Stream(stream) => &stream.dict,
        _ => return Component::Unsaid,
    };
    let entry = document.get_key(dict, "TransferFunction");
    if let Some(name) = entry.as_name() {
        // `Identity` is the only name Table 128 gives this entry; another names a function this
        // file did not supply, which is the `/TR` case one clause up and takes the same answer.
        return if name.as_bytes() == b"Identity" {
            Component::Identity
        } else {
            Component::Unsaid
        };
    }
    crate::function::Function::parse(document, &entry)
        .ok()
        .map_or(Component::Unsaid, |function| {
            Component::Function(Arc::new(function))
        })
}

impl Component {
    /// Three components none of which the halftone speaks for.
    fn default_all() -> [Self; 3] {
        [Self::Unsaid, Self::Unsaid, Self::Unsaid]
    }
}

/// Applies an `/ExtGState` resource.
impl Interpreter<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "Table 57 read once, entry by entry, in the table's own order — which is where a \
                  reader looking for \"does this tree read /SA\" should find the answer"
    )]
    pub(super) fn apply_ext_gstate(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
        in_text: bool,
    ) {
        let Some(name) = name_at(operands, 0) else {
            return;
        };
        // Table 56 makes `gs`'s operand "the name of a graphics state parameter dictionary in
        // the ExtGState subdictionary of the current resource dictionary", and where the
        // subdictionary has no such key every parameter it would have set stays at whatever the
        // last one left — an alpha, a blend mode, a soft mask, a dash pattern. That is a
        // *wrong* graphics state rather than a missing mark, which is the harder of the two to
        // see on a page and the better reason to say so.
        let Some(dict) = self.resource(resources, "ExtGState", &name) else {
            self.note_missing_resource("ExtGState", &name, "is not in /ExtGState");
            return;
        };
        // §8.4.5 makes the value "a graphics state parameter dictionary whose contents specify
        // the values of one or more graphics state parameters"; anything else specifies none.
        let Some(dict) = dict.as_dict() else {
            self.note_missing_resource("ExtGState", &name, "is not a dictionary");
            return;
        };

        // Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2`: the pair in force, and the flag that
        // says a function was stated at all. The two are separate because they answer separate
        // questions — the pair is a graphics state parameter and is saved and restored with the
        // rest of this state, and the flag feeds one report for the whole interpretation
        // (`Interpreter::note_black_generation_departure`). ADR 1207.
        if self.states_black_generation(dict) {
            self.black_generation_stated = true;
        }
        state.black_generation =
            BlackGeneration::read(self.document, dict, state.black_generation.as_deref());
        if let Some(alpha) = self.document.get_key(dict, "ca").as_number() {
            state.fill_alpha = clamp_unit(alpha);
        }
        if let Some(alpha) = self.document.get_key(dict, "CA").as_number() {
            state.stroke_alpha = clamp_unit(alpha);
        }
        // Table 57 `/D`: the line dash pattern, "expressed as an array of the form
        // [ dashArray dashPhase ]". The same pattern the `d` operator sets, written as a
        // real array rather than as flattened operands.
        if let Some(entry) = self.document.get_key(dict, "D").as_array()
            && let Some(items) = entry.first().map(|item| self.document.resolve(item))
            && let Some(items) = items.as_array()
        {
            let array = items
                .iter()
                .map(|item| self.document.resolve(item))
                .filter_map(|item| item.as_number())
                .map(narrow)
                .collect();
            let phase = entry
                .get(1)
                .map(|item| self.document.resolve(item))
                .and_then(|item| item.as_number())
                .map_or(0.0, narrow);
            apply_dash(array, phase, &mut state.stroke);
        }
        // Table 57's `/LC`, `/LJ` and `/ML`: the same three parameters `J`, `j` and `M` set,
        // through the other of §8.4.1 NOTE 1's two routes. `issue16287.pdf`, `issue7878.pdf`
        // and `extgstate.pdf` set all three this way, and none of them reached the stroke.
        if let Some(code) = self.document.get_key(dict, "LC").as_integer() {
            state.stroke.cap = line_cap(code);
        }
        if let Some(code) = self.document.get_key(dict, "LJ").as_integer() {
            state.stroke.join = line_join(code);
        }
        if let Some(limit) = self.document.get_key(dict, "ML").as_number() {
            state.stroke.miter_limit = miter_limit(narrow(limit));
        }
        self.apply_ext_gstate_font(dict, state);
        // Table 57's `/LW`, which §8.4.1's NOTE 1 makes the second route to the same
        // parameter the `w` operator sets — so it is clipped into range by §8.4.1's own
        // sentence, exactly as `run.rs`'s `w` is, and for the reason written there.
        if let Some(width) = self.document.get_key(dict, "LW").as_number() {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a line width outside f32's range is not a line width"
            )]
            {
                state.stroke.width = (width as f32).max(0.0);
            }
        }
        // Table 57's `/SA`: §10.7.5's automatic stroke adjustment. What it changes here is
        // the one rule of that clause a display can state exactly — a line under half a
        // device pixel wide "shall be rendered as a single-pixel line" — which
        // `Stroke::device_width` applies once for every rasteriser, since only a backend knows
        // the resolution. The clause's other half, adjusting a stroke's *coordinates* to the
        // pixel grid for uniform thickness, is declined: ADR 0028 made that argument and
        // ADR 0848 measured it, on a ladder of one width at eight sub-pixel placements, where
        // this device holds the clause's own thickness bound to 0.0039 of a device pixel and
        // the one reference that grid-fits holds it to 0.4000. The ledger's §10.7.5 row has
        // the ladder, the population and why the status stays `partial` all the same.
        if let Object::Boolean(adjust) = self.document.get_key(dict, "SA") {
            state.stroke.adjust = adjust;
        }
        // Table 57's `/TR` and `/TR2`: §10.5's transfer function. `Transfer::read` answers `None`
        // for a state that names neither and `Some(None)` for one that names `/Identity` or
        // `/Default` — a state that turns an inherited transfer *off* rather than one that says
        // nothing, which are different things and only one of them clears the field.
        //
        // **Skipped inside an uncoloured figure**, which §8.6.8 requires and which this did not
        // do between the three-hundred-and-fifty-eighth session and the
        // three-hundred-and-seventy-fifth. The clause names both entries in the list it applies
        // to a `d1` glyph description and to an uncoloured tiling pattern's stream: "[a]ll of the
        // following entries, if present in the graphics state parameter dictionary of a gs
        // operator shall be ignored" — `TR`, `TR2`, `BG`, `BG2`, `UCR`, `UCR2`, `HT` and
        // `UseBlackPtComp`. A transfer function is a colour mapping and such a figure's colour is
        // "specified separately each time [it is] used", so honouring one here would let the cell
        // decide a colour the caller supplies.
        //
        // Table 57's `/HT` is read here too, and for one entry of it: §10.5's second bullet makes
        // a halftone dictionary the other place a transfer function is stated, and says that one
        // "shall override the corresponding one specified by the current transfer function
        // parameter in the graphics state". The screen it is written beside is §10.6's and this
        // device performs none of it; see [`TransferState`] for why the two part company.
        if !self.uncoloured {
            let mut moved = false;
            match Transfer::read(self.document, dict) {
                Stated::Unsaid => {}
                Stated::None => {
                    state.transfer.stated = None;
                    moved = true;
                }
                Stated::Set(transfer) => {
                    state.transfer.stated = Some(Arc::new(transfer));
                    moved = true;
                }
            }
            if let Some(halftone) = halftone_transfer(self.document, dict) {
                state.transfer.halftone = halftone;
                moved = true;
            }
            if moved {
                state.transfer.compose();
            }
        }
        // Table 57's `/FL`: §10.7.2's flatness tolerance, read here and discarded. The clause
        // grants that outright rather than leaving it to a judgement about devices —
        //
        // > PDF processors may choose to ignore any flatness tolerance specified within a PDF
        // > file.
        //
        // — and this one does, because the tolerance "controls the maximum permitted distance in
        // device pixels between the mathematically correct path and an approximation constructed
        // from straight line segments", and no curve in this tree is approximated by a number the
        // document states: each backend flattens at its own device resolution, which is the only
        // place the pixel the clause measures in exists.
        //
        // **Discarded here for the same reason `run.rs` discards `i`, which is the other of
        // §8.4.1 NOTE 1's two routes to this one parameter.** §10.7.2 makes them equals —
        // flatness "may be specified as the operand of the i operator ... or as the value of the
        // FL entry in a graphics state parameter dictionary" — so a permission taken on one route
        // is taken on both, and the read is what says this entry was looked at and let go rather
        // than never looked for.
        let _ = self.document.get_key(dict, "FL");
        // Table 57's `/SM`: §10.7.3's smoothness tolerance, "the maximum error tolerance for
        // rendering shadings", expressed "as a fraction of the range of each colour
        // component". It decides how finely a shading's colour function is sampled, and only
        // upwards — see `Ramp::resolution_for`, where the clause's own "each output device
        // may have internal limits" is what keeps this device's 1/256 for the coarser
        // requests. 23 corpus documents state one; most say 0.02 and five say 0.002.
        if let Some(tolerance) = self.document.get_key(dict, "SM").as_number() {
            state.smoothness = Some(narrow(tolerance));
        }
        // Table 57's `/OP`, `/op` and `/OPM`: §8.6.7's two overprint parameters and the
        // overprint mode. `/OP` sets both "unless there is also an op entry in the same
        // graphics state parameter dictionary, in which case the OP entry shall set only the
        // overprint parameter for stroking", and `/op`, where absent, takes `/OP`'s value —
        // so the two entries of one dictionary are read together rather than one after the
        // other. §8.6.7's own condition keeps them from changing a pixel on this device's
        // three additive colourants; what they decide is §11.7.4.3's special blend mode
        // inside a transparency group compositing in four components, which is a group space
        // rather than a device one. See [`Interpreter::overprint_blend`] and ADR 1157.
        let boolean = |entry: Object| match entry {
            Object::Boolean(flag) => Some(flag),
            _ => None,
        };
        let stroking = boolean(self.document.get_key(dict, "OP"));
        let filling = boolean(self.document.get_key(dict, "op"));
        if let Some(overprint) = stroking {
            state.overprint_stroking = overprint;
        }
        if let Some(overprint) = filling.or(stroking) {
            state.overprint_filling = overprint;
        }
        if let Some(mode) = self.document.get_key(dict, "OPM").as_integer() {
            state.overprint_mode = mode;
        }
        // ISO 32000-2 §8.6.5.9 and its table entry: `/UseBlackPtComp` takes ON, OFF or
        // Default, and a rendering intent of AbsColorimetric forces it off regardless.
        //
        // Both are skipped inside an uncoloured figure. §8.6.8 lists the `/ExtGState` entries
        // such a stream may not set, and this tree reads four of them: `/UseBlackPtComp` by
        // name, `/RI` because the `ri` operator that sets the same parameter is on the operator
        // half of the same list, `/TR` and `/TR2` above — since the three-hundred-and-fifty-eighth
        // session — and `/HT` beside them, since the six-hundred-and-seventy-seventh. `/BG`,
        // `/BG2`, `/UCR` and `/UCR2` are §10.4's, which this device does not perform, and are
        // read for a flag and never evaluated. **This comment said the two
        // here were "the only ones on that list this tree reads at all", and listed `/TR` and
        // `/TR2` among the unread, until the three-hundred-and-seventy-fifth session** — thirty
        // lines below the `Transfer::read` that had read both for seventeen sessions, and the
        // sentence was the reason nobody noticed §8.6.8 was being broken. The rest of this
        // dictionary is not colour and still applies — the
        // line width §9.6.4 asks a glyph description to set explicitly among it.
        if !self.uncoloured {
            if let Object::Name(value) = self.document.get_key(dict, "UseBlackPtComp") {
                state.use_black_pt_comp = match value.as_bytes() {
                    b"ON" => BlackPoint::On,
                    b"OFF" => BlackPoint::Off,
                    _ => BlackPoint::Default,
                };
            }
            // §8.6.5.8's second route to the intent: "the RI entry in a graphics state
            // parameter dictionary". Every name is kept rather than only the absolute one,
            // because §8.6.5.9's override is asked of the intent in force when an object is
            // painted — so a `/RI` naming any other intent has to *replace* an absolute one
            // rather than leave a decision the previous one made standing.
            if let Object::Name(intent) = self.document.get_key(dict, "RI") {
                state.intent = Intent::read(intent.as_bytes());
            }
        }
        match self.document.get_key(dict, "BM") {
            Object::Name(name) => state.blend = blend_mode(name.as_bytes()),
            Object::Array(items) => {
                // §11.6.3, of the deprecated array form: a processor "shall use the first
                // blend mode in the array that it recognizes (or Normal if it recognizes none
                // of them)". The first *name* is not the first recognised one — `[/FooBar
                // /Multiply]` names a mode this reader knows in second place — so the
                // recognition test has to be inside the search rather than after it.
                state.blend = items
                    .iter()
                    .map(|item| self.document.resolve(item))
                    .filter_map(|item| item.as_name().map(|name| name.as_bytes().to_vec()))
                    .find_map(|name| known_blend_mode(&name))
                    .unwrap_or(BlendMode::Normal);
            }
            _ => {}
        }

        // §9.3.8, `/TK`: the ninth text state parameter, and the only one with no operator.
        // "Any TK value in a graphics state parameter dictionary installed using the gs
        // operator shall be ignored between the BT and ET operators delimiting a text
        // object" — so a `gs` inside a text object sets everything else here and not this.
        if !in_text && let Object::Boolean(knockout) = self.document.get_key(dict, "TK") {
            state.text.knockout = knockout;
        }

        // §11.6.4.3's `/AIS`, Table 57's "alpha source flag": whether the soft mask and
        // §11.6.4.4's two constants state shape or opacity.
        //
        // > This is a boolean flag, set with the AIS ("alpha is shape") entry in a graphics
        // > state parameter dictionary (8.4.5, "Graphics state parameter dictionaries"): true
        // > if the soft mask contains shape values, false for opacity.
        //
        // Alpha is the product `α = f × q` (§11.3.7.1), so which of the two a value is called
        // changes nothing anywhere the product is all that is used — and until ADR 0234 that
        // was everywhere this tree draws, because §11.4.6's groups were reported. It is not
        // now: a knockout element's shape is built by *removing* the mask and the constant
        // under `/AIS false`, and under true the drawn alpha already is the shape, because
        // §11.6.4.2 gives an elementary object an intrinsic opacity of 1.0 and this entry
        // takes the other two opacity inputs away. Both readings are drawn; see
        // `transparency::stated_shape`.
        //
        // So the flag is read, and both records of it are kept, with two scopes.
        // `state.alpha_is_shape` is the parameter itself — set either way here, bounded by
        // `q`/`Q` like everything else in the struct — and `self.readings` is the question a
        // *group* asks when it closes, "which reading did my elements paint under", which is a
        // history rather than a value and is therefore accumulated within one scope's run and
        // kept to one reading by sealing what was painted under the other (ADR 0327, ADR 1319).
        // `issue18032.pdf` states the entry inside a form whose group draws nothing, which is
        // why the record is a scope's and not the page's.
        if let Object::Boolean(flag) = self.document.get_key(dict, "AIS") {
            state.alpha_is_shape = flag;
            self.note_alpha_source(flag);
        }

        // §11.6.4.3's soft mask: an independent source of shape or opacity, defined by a
        // transparency group and applied to every object painted while it is in force.
        match crate::soft_mask::entry_with_output_intent(
            self.document,
            dict,
            self.presses,
            self.output_intent.as_ref(),
        ) {
            crate::soft_mask::SoftMaskEntry::None => state.soft_mask = None,
            crate::soft_mask::SoftMaskEntry::Mask(request) => {
                state.soft_mask = self.build_soft_mask(&request, state);
            }
            crate::soft_mask::SoftMaskEntry::Unusable(detail) => {
                state.soft_mask = None;
                self.note(Unsupported::SoftMask {
                    detail: format!("/{name}: {detail}"),
                });
            }
        }
    }

    /// Whether `dict` states a black-generation or undercolour-removal function of its own.
    ///
    /// ISO 32000-2 Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2`, read for the one question this
    /// tree asks of them: does the file replace the *device's* functions, or name them? §10.4.2.4
    /// settles what counts as stating one — "[t]he black-generation and undercolour-removal
    /// functions shall be defined as PDF function dictionaries (see 7.10, \"Functions\")" — so a
    /// dictionary or a stream is a function and nothing else is.
    ///
    /// **A name is not**, which is the whole of this reader's narrowing: Table 57 gives `/BG2` and
    /// `/UCR2` a second admissible value, "the name Default, denoting the black-generation function
    /// that was in effect at the start of the page", and that function is this device's own. A
    /// state naming it departs from nothing. Any other name names a function the file did not
    /// supply.
    ///
    /// Table 57's own precedence decides which of each pair is in force: "[i]f both BG and BG2 are
    /// present in the same graphics state parameter dictionary, BG2 shall take precedence" — so a
    /// `/BG2 /Default` beside a `/BG` function puts the device's function back.
    pub(super) fn states_black_generation(&self, dict: &Dictionary) -> bool {
        [["BG2", "BG"], ["UCR2", "UCR"]].into_iter().any(|pair| {
            pair.into_iter()
                .map(|key| self.document.get_key(dict, key))
                .find(|entry| !entry.is_null())
                .is_some_and(|entry| matches!(entry, Object::Dictionary(_) | Object::Stream(_)))
        })
    }
}

/// Clamps a value to `0.0..=1.0` as an `f32`.
fn clamp_unit(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to 0.0..=1.0 before narrowing, so the conversion is exact"
    )]
    {
        value.clamp(0.0, 1.0) as f32
    }
}

/// Maps a PDF blend mode name, taking `Normal` for anything this reader does not know.
///
/// `Normal` and `Compatible` are the two names that mean it deliberately; §11.6.3 asks for the
/// same answer for an unrecognised one, which is why the two cases can share an arm here and
/// have to be told apart by [`known_blend_mode`] when an array is choosing between names.
pub(super) fn blend_mode(name: &[u8]) -> BlendMode {
    known_blend_mode(name).unwrap_or(BlendMode::Normal)
}

/// Maps a PDF blend mode name, or `None` where the name is not one Table 134 or 135 lists.
fn known_blend_mode(name: &[u8]) -> Option<BlendMode> {
    Some(match name {
        b"Normal" | b"Compatible" => BlendMode::Normal,
        b"Multiply" => BlendMode::Multiply,
        b"Screen" => BlendMode::Screen,
        b"Overlay" => BlendMode::Overlay,
        b"Darken" => BlendMode::Darken,
        b"Lighten" => BlendMode::Lighten,
        b"ColorDodge" => BlendMode::ColorDodge,
        b"ColorBurn" => BlendMode::ColorBurn,
        b"HardLight" => BlendMode::HardLight,
        b"SoftLight" => BlendMode::SoftLight,
        b"Difference" => BlendMode::Difference,
        b"Exclusion" => BlendMode::Exclusion,
        b"Hue" => BlendMode::Hue,
        b"Saturation" => BlendMode::Saturation,
        b"Color" => BlendMode::Color,
        b"Luminosity" => BlendMode::Luminosity,
        _ => return None,
    })
}
