//! `gs` and Table 57: the graphics state parameter dictionary, entry by entry.
//!
//! §10.5's transfer function lives here too, because Table 57's `/TR`, `/TR2` and `/HT` are the
//! three ways a file states one — the clause's own two bullets, the second of which reaches into a
//! halftone dictionary and is why this module reads §10.6.5 for one entry and nothing else.

use std::sync::Arc;

use pdf_render::{BlendMode, Color};
use pdf_syntax::{Dictionary, Document, Object};

use super::colour::{BlackPoint, Intent};
use super::report::Unsupported;
use super::run::{name_at, narrow};
use super::{GraphicsState, Interpreter, apply_dash, line_cap, line_join, miter_limit};

/// ISO 32000-2 §10.5's transfer function: one map per component, ready to apply.
///
/// [`TransferState`] is what an `/ExtGState` sets — Table 57's `/TR` and `/TR2` for the clause's
/// first bullet, `/HT` for its second — and this is the composition of the two that a colour goes
/// through.
///
/// > In the sequence of steps for processing colours, the PDF processor shall apply the transfer
/// > function after performing any needed conversions between colour spaces.
///
/// **Why a screen has one at all**, since this tree called it inapplicable for three hundred and
/// fifty-seven sessions: the standard never uses the phrase "marking device" — §8.3.2.2's term is
/// a "raster output device *such as a display or a printer*" — and §10.1's list of rendering steps
/// makes halftoning conditional on the device and the transfer function not. §10.6.1 says it for
/// the case of a screen outright: "[h]alftoning is not required for such devices; **after gamma
/// correction by the transfer functions**, the colour components shall be transmitted directly to
/// the device."
///
/// One function or four. The clause: "[i]f only a single function is specified, it shall apply to
/// all components. An RGB device shall use the first three" — and this device is RGB, so the
/// fourth is read and never asked.
///
/// Both ends are additive by the clause's own rule — "the greater the numeric value, the lighter
/// the colour" — which is what makes applying it to an RGB colour the whole of it: nothing here
/// has to subtract anything from 1.0, because nothing here is subtractive by the time it arrives.
///
/// **Public because a shading's colours are built outside this module.** Table 57 is read here and
/// nowhere else, so this is where the type belongs; but §10.5's subject is the component value, and
/// a ramp's stops, a mesh's corners and a function-based shading's grid are produced in
/// [`crate::shading`] and [`crate::mesh`]. Those take one of these rather than a closure, so the
/// clause is stated once and applied in every place a colour is made. Nothing outside this crate
/// can construct one — [`Transfer::read`] is private and an `/ExtGState` is the only source the
/// clause gives — so the public surface is what a caller needs to pass one on and no more.
///
/// **A channel is optional because the clause has two sources and they meet per component.** The
/// second bullet lets a halftone dictionary carry a `TransferFunction` for one component and say
/// nothing about the others, and [`TransferState`] composes the two; a channel that neither source
/// names passes its component through untouched.
#[derive(Debug, Clone)]
pub struct Transfer {
    /// Red, green and blue. One stated function fills all three (`Arc` so it is not cloned).
    channels: [Option<Arc<crate::function::Function>>; 3],
}

impl Transfer {
    /// Table 57's two entries, read from an `/ExtGState`, with `/TR2` in preference to `/TR`.
    ///
    /// Table 57 makes that precedence explicit — "[i]f both TR and TR2 are present in the same
    /// graphics state parameter dictionary, TR2 shall take precedence" — and both take a
    /// function, an array of four, or a name.
    ///
    /// Three answers, not two, which is what [`Stated`] exists to say: the state says nothing, the
    /// state turns an inherited transfer **off** (`/Identity`, or `/TR2`'s `/Default`), or the
    /// state sets one. Folding the middle into the first would leave an inherited transfer running
    /// through a `q … /Identity gs … Q` that exists to stop it.
    fn read(document: &Document, state: &Dictionary) -> Stated {
        let Some(entry) = ["TR2", "TR"]
            .into_iter()
            .map(|key| document.get_key(state, key))
            .find(|value| !value.is_null())
        else {
            return Stated::Unsaid;
        };
        if let Some(name) = entry.as_name() {
            // The two names that mean "no transfer". Any other name is a function this file did
            // not supply, and leaving the state alone is what a name nobody defined can mean.
            return match name.as_bytes() {
                b"Identity" | b"Default" => Stated::None,
                _ => Stated::Unsaid,
            };
        }
        let read = |object: &Object| {
            crate::function::Function::parse(document, object)
                .ok()
                .map(Arc::new)
        };
        let channels = match entry.as_array() {
            // "[A]n array of four separate transfer functions, one each for red, green, blue, and
            // gray or their complements" — an RGB device uses the first three.
            Some(items) if items.len() >= 3 => {
                let mut out = Vec::with_capacity(3);
                for item in items.iter().take(3) {
                    let Some(function) = read(&document.resolve(item)) else {
                        return Stated::Unsaid;
                    };
                    out.push(function);
                }
                match (out.first(), out.get(1), out.get(2)) {
                    (Some(first), Some(second), Some(third)) => [
                        Some(first.clone()),
                        Some(second.clone()),
                        Some(third.clone()),
                    ],
                    _ => return Stated::Unsaid,
                }
            }
            // An array of any other length is not what the clause states, and a state this reader
            // cannot make sense of leaves the one in force alone.
            Some(_) => return Stated::Unsaid,
            None => {
                let Some(one) = read(&entry) else {
                    return Stated::Unsaid;
                };
                [Some(one.clone()), Some(one.clone()), Some(one)]
            }
        };
        Stated::Set(Self { channels })
    }

    /// The colour a device would receive, with the alpha untouched.
    ///
    /// Alpha is not a colour component: §10.5 speaks of "the value of a colour component in the
    /// device's native colour space", and §11's shape and opacity are a different quantity in a
    /// different clause.
    #[must_use]
    pub fn apply(&self, colour: Color) -> Color {
        let map = |channel: &Option<Arc<crate::function::Function>>, value: f32| {
            let Some(function) = channel.as_ref() else {
                return value;
            };
            function
                .eval(&[value.clamp(0.0, 1.0)])
                .first()
                .copied()
                .map_or(value, |out| out.clamp(0.0, 1.0))
        };
        Color {
            r: map(&self.channels[0], colour.r),
            g: map(&self.channels[1], colour.g),
            b: map(&self.channels[2], colour.b),
            a: colour.a,
        }
    }
}

/// What an `/ExtGState` said about §10.5's transfer function.
///
/// Three answers rather than two, because "says nothing" and "says `/Identity`" are different
/// instructions: the first leaves whatever is in force, and the second is how a file turns an
/// inherited transfer off. `issue6931_reduced.pdf` uses both — one state sets three functions and
/// the next sets `/Identity` — so a reader that could not tell them apart would carry the transfer
/// on past the object it was written for.
enum Stated {
    /// The dictionary has neither entry, or has one this reader cannot make sense of.
    Unsaid,
    /// `/Identity`, or `/TR2`'s `/Default`: no transfer from here on.
    None,
    /// A function, or four of them.
    Set(Transfer),
}

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
                    .and_then(|stated| stated.channels[index].clone()),
                Component::Identity => None,
                Component::Function(function) => Some(Arc::clone(function)),
            });
        self.effective = channels
            .iter()
            .any(Option::is_some)
            .then(|| Arc::new(Transfer { channels }));
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

        // Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2`, which this tree does not evaluate.
        // They matter to exactly one thing it does: §11.7.5.3 makes them the functions
        // §10.4.2.4 uses "[w]hen painting an elementary object with a DeviceRGB colour
        // directly into a transparency group whose colour space is DeviceCMYK", which is
        // every non-subtractive colour on a page §11.4.7 composites in `DeviceCMYK`. A page
        // that states one is therefore drawn with the wrong black generation and is not drawn
        // in its blending space at all — the flag is monotone for the page, because the state
        // they were set in is not the only place they apply.
        if ["BG", "BG2", "UCR", "UCR2"]
            .iter()
            .any(|key| !matches!(self.document.get_key(dict, key), Object::Null))
        {
            self.black_generation_stated = true;
        }
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
        // `Stroke::device_width` applies once for both backends, since only a backend knows
        // the resolution. The clause's other half, adjusting a stroke's *coordinates* to the
        // pixel grid for uniform thickness, is what anti-aliasing already achieves by a
        // different route; ADR 0028 has that argument and the ledger's §10.7.5 row records it.
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
        // Table 57's `/SM`: §10.7.3's smoothness tolerance, "the maximum error tolerance for
        // rendering shadings", expressed "as a fraction of the range of each colour
        // component". It decides how finely a shading's colour function is sampled, and only
        // upwards — see `Ramp::resolution_for`, where the clause's own "each output device
        // may have internal limits" is what keeps this device's 1/256 for the coarser
        // requests. 23 corpus documents state one; most say 0.02 and five say 0.002.
        if let Some(tolerance) = self.document.get_key(dict, "SM").as_number() {
            state.smoothness = Some(narrow(tolerance));
        }
        // Table 57's `/OP`, `/op` and `/OPM`: overprint and overprint mode, deliberately not
        // read, which §8.6.7 is explicit about rather than silent on. Overprinting decides
        // what happens to the device colourants a painting operation does *not* name, and
        // this device has three additive process colourants and no separations: "Not all
        // devices support overprinting. … If overprinting is not supported, the value of the
        // overprint parameter shall be ignored" (§8.6.7 NOTE 1), and of the overprint mode,
        // "It also shall not apply if the native colour space of the output device does not
        // include CMYK device colourants; in that case, source colours shall be converted to
        // the device's native colour space, and all components participate in the conversion,
        // whatever their values." §11.7.4's transparent-model reading reaches the same place
        // by a second route — see ADR 0028 and the ledger's §11.7.4 rows.
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
        // `q`/`Q` like everything else in the struct — and `self.alpha_sources` is the
        // question a *group* asks when it closes, "which readings did my content paint
        // under", which is a history rather than a value and is therefore accumulated within
        // one group's run. It used to be a single monotone flag across the whole page, and
        // nine corpus documents state the entry: `issue18032.pdf` states it inside a form
        // whose group draws nothing, and the page-wide flag refused a knockout group two
        // forms later for it (ADR 0327).
        if let Object::Boolean(flag) = self.document.get_key(dict, "AIS") {
            state.alpha_is_shape = flag;
            let stated = super::AlphaSourcesSeen::of(flag);
            let painted = self.list.command_count();
            // A reading nothing was painted under is replaced rather than mixed in — see
            // `Interpreter::alpha_sources_mark`, and it is what lets a form that opens with
            // the `gs` stating `/AIS` be drawn instead of reported.
            self.alpha_sources = if painted == self.alpha_sources_mark {
                stated
            } else {
                self.alpha_sources.with(stated)
            };
            self.alpha_sources_mark = painted;
        }

        // §11.6.4.3's soft mask: an independent source of shape or opacity, defined by a
        // transparency group and applied to every object painted while it is in force.
        match crate::soft_mask::entry(self.document, dict) {
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

#[cfg(test)]
mod tests {
    use super::{Stated, Transfer};

    /// One page whose `/ExtGState` sets §10.5's transfer function, as PDF bytes.
    ///
    /// `entry` is written verbatim into the graphics state, so one builder serves an array of
    /// three, a single function, `/Identity` and a name nobody defined.
    fn with_transfer(entry: &str) -> pdf_syntax::Document {
        use std::fmt::Write as _;
        // A type-2 exponential function, which is §7.10.3's two-line form: f(x) = 1 - x here,
        // because `/C0 [1]`, `/C1 [0]` and `/N 1` is the straight line between them.
        let body = format!(
            "1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
             /Resources << /ExtGState << /G << /Type /ExtGState /TR {entry} >> >> >> \
             /Contents 4 0 R >> endobj\n\
             4 0 obj << /Length 0 >> stream\n\nendstream endobj\n\
             5 0 obj << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> endobj\n"
        );
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for object in body.split_inclusive("endobj\n") {
            offsets.push(out.len());
            out.push_str(object);
        }
        let at = out.len();
        let size = offsets.len().saturating_add(1);
        let _ = writeln!(out, "xref\n0 {size}");
        out.push_str("0000000000 65535 f \n");
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
        );
        pdf_syntax::Document::open(out.into_bytes()).expect("the fixture parses")
    }

    /// The `/ExtGState` of the fixture above.
    fn state(document: &pdf_syntax::Document) -> pdf_syntax::Dictionary {
        let pages = crate::Pages::new(document);
        let page = pages.get(0).expect("one page");
        let graphics = document.get_key(&page.resources, "ExtGState");
        let dict = graphics.as_dict().expect("the fixture states one");
        document
            .get_key(dict, "G")
            .as_dict()
            .cloned()
            .expect("the fixture names it /G")
    }

    /// ISO 32000-2 §10.5, read and applied: every colour component through its own function.
    ///
    /// > The transfer function shall be called with a numeric operand in the range 0.0 to 1.0 and
    /// > shall return a number in the same range.
    ///
    /// Three shapes, because the clause states three: an array — "one each for red, green, blue,
    /// and gray" of which "[a]n RGB device shall use the first three" — a single function, which
    /// "shall apply to all components", and `/Identity`. The fourth case is a name nobody defined,
    /// which the clause does not describe and which leaves the state alone rather than clearing it.
    #[test]
    fn a_transfer_function_maps_every_colour_component() {
        let document = with_transfer("[5 0 R 5 0 R 5 0 R 5 0 R]");
        let Stated::Set(transfer) = Transfer::read(&document, &state(&document)) else {
            panic!("an array of four is a transfer function");
        };
        let out = transfer.apply(pdf_render::Color {
            r: 0.25,
            g: 0.5,
            b: 1.0,
            a: 0.75,
        });
        assert!((out.r - 0.75).abs() < 1e-4, "{out:?}");
        assert!((out.g - 0.5).abs() < 1e-4, "{out:?}");
        assert!(out.b.abs() < 1e-4, "{out:?}");
        // Alpha is not a colour component: §10.5 speaks of "the value of a colour component in the
        // device's native colour space", and §11's opacity is a different clause's quantity.
        assert!((out.a - 0.75).abs() < 1e-6, "{out:?}");

        // "If only a single function is specified, it shall apply to all components."
        let one = with_transfer("5 0 R");
        let Stated::Set(transfer) = Transfer::read(&one, &state(&one)) else {
            panic!("a single function is a transfer function");
        };
        let out = transfer.apply(pdf_render::Color {
            r: 0.25,
            g: 0.25,
            b: 0.25,
            a: 1.0,
        });
        assert!(
            (out.r - 0.75).abs() < 1e-4 && (out.b - 0.75).abs() < 1e-4,
            "{out:?}"
        );

        // `/Identity` turns an inherited transfer *off*, which is not the same as saying nothing —
        // and `issue6931_reduced.pdf` states both, one graphics state after the other.
        let identity = with_transfer("/Identity");
        assert!(matches!(
            Transfer::read(&identity, &state(&identity)),
            Stated::None
        ));
        let nonsense = with_transfer("/NoSuchThing");
        assert!(matches!(
            Transfer::read(&nonsense, &state(&nonsense)),
            Stated::Unsaid
        ));
    }
}
