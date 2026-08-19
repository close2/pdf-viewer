//! The colour operators: §8.6's spaces, `sc`/`scn`, and device-space resolution.
//!
//! What belongs here is how an operator's operands become a [`Color`] in the graphics
//! state — the spaces themselves are `crate::colour`'s. §8.6.5.9's black point setting and
//! §14.11.5's output intent are read here because they change what those operands mean.

use pdf_render::Color;
use pdf_syntax::{Dictionary, Document, Name, Object, ObjectId};

use crate::colour::{ColourSpace, Compositing};

use super::report::Unsupported;
use super::run::{name_at, number_at};
use super::{GraphicsState, Interpreter};

/// Whether black point compensation applies, per ISO 32000-2 §8.6.5.9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlackPoint {
    /// `/UseBlackPtComp ON`, or the processor's own default.
    On,
    /// `/UseBlackPtComp OFF`, or any rendering intent of `AbsColorimetric` — for which
    /// the specification says the entry "shall be treated as OFF" whatever it holds.
    Off,
    /// `/UseBlackPtComp Default`, which the specification leaves to the processor.
    Default,
}

impl BlackPoint {
    /// Whether to compensate. `Default` does, which is this processor's determination.
    fn applies(self) -> bool {
        self != Self::Off
    }
}

impl Interpreter<'_> {
    /// Converts a colour for whatever is being composited into.
    ///
    /// [`convert`] carries the arithmetic. This exists so that the interpreter's own
    /// `compositing` is read in one place: an operator's colour, an image's samples and a
    /// shading's ramp all have to reach the raster in the same quantity, and the other two
    /// take the same value through `crate::image` and `crate::shading`.
    pub(super) fn colour(
        &mut self,
        space: &ColourSpace,
        values: &[f32],
        black_point: BlackPoint,
    ) -> Color {
        convert(space, values, black_point, &self.compositing)
    }

    /// Sets a colour space, which decides how the operands of `sc`/`scn` are read.
    ///
    /// The space itself is kept rather than only its component count, so that `Separation`
    /// and `DeviceN` colours go through their tint transform and `Indexed` ones through
    /// their table. Reading them by component count alone treats a single ink tint as a
    /// grey level, which is a plausible and wrong colour.
    pub(super) fn set_colour_space(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
        fill: bool,
    ) {
        let Some(name) = name_at(operands, 0) else {
            return;
        };

        // The one space worth remembering, and the only one that can be: see
        // [`Interpreter::icc_spaces`].
        // **The table is asked before the shape is tested**, and the order is the whole cost
        // argument: a hit is one map lookup, and [`is_icc_based`] — which resolves the array
        // and so copies it — runs once per *distinct* object rather than once per operator.
        let stated = self
            .resource_entry(resources, "ColorSpace", &name)
            .and_then(|entry| entry.as_reference());
        if let Some(id) = stated
            && let Some(space) = self.icc_spaces.get(&id)
        {
            let space = space.clone();
            self.take_colour_space(space, state, fill);
            return;
        }

        let space = ColourSpace::parse(self.document, &Object::Name(name.clone()), resources);
        if let (Some(id), Some(parsed)) = (stated, space.as_ref())
            && is_icc_based(self.document, id)
        {
            self.icc_spaces.insert(id, parsed.clone());
        }
        let space = space.unwrap_or_else(|| {
            self.note(Unsupported::Shading {
                name: format!("colour space /{}", String::from_utf8_lossy(name.as_bytes())),
            });
            ColourSpace::Gray
        });

        self.take_colour_space(space, state, fill);
    }

    /// Puts a space into the graphics state, with §8.6.8's initial colour.
    ///
    /// Split out of [`Interpreter::set_colour_space`] so that a space answered from
    /// [`Interpreter::icc_spaces`] and one parsed on the spot take exactly the same path: a
    /// memo that skipped this would set the space and leave the *previous* space's colour, and
    /// the clause is explicit that it must not.
    fn take_colour_space(&mut self, space: ColourSpace, state: &mut GraphicsState, fill: bool) {
        // §8.6.8: `cs` and `CS` "shall also set the current colour to its initial value,
        // which depends on the colour space". Omitting this leaves the previous space's
        // colour in place, which shows up as content painted in the wrong colour — and the
        // initial value is *not* simply black: `ColourSpace::initial_colour` carries the
        // clause's five cases, of which a `Separation`'s full ink and an `Indexed` space's
        // entry 0 are the two that are usually some other colour entirely.
        //
        // A `Pattern` space is the sixth case and has no components: its initial colour "shall
        // be a pattern object that causes nothing to be painted", which is a fully transparent
        // paint here, and the pattern the previous `scn` set has to go with it.
        let initial = space.initial_colour();
        let colour = if initial.is_empty() {
            Color::TRANSPARENT
        } else {
            self.colour(&space, &initial, state.black_point)
        };
        if fill {
            state.fill_space = space;
            state.fill = colour;
            state.fill_pattern = None;
        } else {
            state.stroke_space = space;
            state.stroke_colour = colour;
            state.stroke_pattern = None;
        }
    }

    /// Sets a colour from `sc`/`scn` operands, interpreting them by component count.
    pub(super) fn set_colour(
        &mut self,
        operands: &[Object],
        resources: &Dictionary,
        state: &mut GraphicsState,
        fill: bool,
    ) {
        // A trailing name means a pattern rather than a colour.
        if let Some(name) = operands.iter().find_map(Object::as_name) {
            // Numeric operands alongside the name are the colour an *uncoloured* tiling
            // pattern is poured through, in the pattern's underlying space.
            let tint: Vec<f32> = (0..operands.len())
                .filter_map(|index| number_at(operands, index))
                .collect();
            let pattern = self.pattern(name, resources, &tint, state, fill);
            if fill {
                state.fill_pattern = pattern;
            } else {
                state.stroke_pattern = pattern;
            }
            return;
        }

        // Setting an ordinary colour clears any pattern the space had selected.
        if fill {
            state.fill_pattern = None;
        } else {
            state.stroke_pattern = None;
        }

        let space = if fill {
            &state.fill_space
        } else {
            &state.stroke_space
        };
        let values: Vec<f32> = (0..operands.len())
            .filter_map(|index| number_at(operands, index))
            .collect();

        // Where the operand count disagrees with the declared space, the operands win:
        // producers get `/CS` wrong more often than they get the operand count wrong, and
        // a device space with a matching component count is the likeliest intent.
        let colour = match (values.len(), space.components()) {
            (0, _) => return,
            (given, expected) if given == expected => {
                let space = space.clone();
                self.colour(&space, &values, state.black_point)
            }
            (1, _) => self.colour(&ColourSpace::Gray, &values, state.black_point),
            (3, _) => self.colour(&ColourSpace::Rgb, &values, state.black_point),
            (4, _) => self.colour(&ColourSpace::Cmyk, &values, state.black_point),
            (given, expected) => {
                self.note(Unsupported::Shading {
                    name: format!("{given} colour components (expected {expected})"),
                });
                return;
            }
        };

        if fill {
            state.fill = colour;
        } else {
            state.stroke_colour = colour;
        }
    }

    /// Resolves a device colour space to what the document says it means.
    ///
    /// Three sources, in the order the specification puts them. A `/Default` entry in the
    /// resources §8.6.5.6 says *shall* be used. Failing that, the output intent describes
    /// the device the document's colours were prepared for, which §8.6.5.7 NOTE 3 names as
    /// the only thing in a PDF that can. Failing both, the device space itself — where
    /// §8.6.4.4 states no conversion, §10.4.2.5 states one and §10.4.2.1 ranks it below
    /// §10.3's ICC route, so what happens then is this processor's own choice between two
    /// answers the standard has already ordered, and is documented as such in `colour.rs`.
    pub(super) fn device_space(&self, name: &str, resources: &Dictionary) -> ColourSpace {
        let named = Object::Name(Name::new(name.as_bytes().to_vec()));
        if let Some(space) = ColourSpace::parse(self.document, &named, resources) {
            // `parse` returns the device space itself when no `/Default` entry replaces
            // it, so an output intent gets its turn only when nothing did.
            let replaced = !matches!(
                (&space, name),
                (ColourSpace::Gray, "DeviceGray")
                    | (ColourSpace::Rgb, "DeviceRGB")
                    | (ColourSpace::Cmyk, "DeviceCMYK")
            );
            if replaced {
                return space;
            }
        }

        if let Some(intent) = &self.output_intent
            && intent.components() == expected_components(name)
        {
            return intent.clone();
        }

        match name {
            "DeviceGray" => ColourSpace::Gray,
            "DeviceCMYK" => ColourSpace::Cmyk,
            _ => ColourSpace::Rgb,
        }
    }
}

/// Assigns a colour to the fill or stroke slot, along with the space that set it.
///
/// `g`, `rg` and `k` set a device space and a colour in one operator, so they replace
/// whatever `cs` had selected — including a pattern.
pub(super) fn assign_colour(
    state: &mut GraphicsState,
    fill: bool,
    colour: Color,
    space: ColourSpace,
) {
    if fill {
        state.fill = colour;
        state.fill_space = space;
        state.fill_pattern = None;
    } else {
        state.stroke_colour = colour;
        state.stroke_space = space;
        state.stroke_pattern = None;
    }
}

/// How many components a device space's colours have.
fn expected_components(name: &str) -> usize {
    match name {
        "DeviceGray" => 1,
        "DeviceCMYK" => 4,
        _ => 3,
    }
}

/// Converts a colour, honouring the graphics state's black point setting.
///
/// Inside a `/Luminosity` mask group whose blending space is subtractive, what is painted is
/// the ink §10.4.2.3 weighs rather than the colour — as a grey, so that
/// `pdf_render::SoftMask::value` reads it back through §10.4.2.2's own formula and gets the
/// number this put there. Two properties make that exact rather than approximate, and both are
/// the clause's:
///
/// - `ColourSpace::ink` is **linear** in a subtractive space's components, and source-over
///   compositing is affine in each component, so mixing greys is mixing the components the
///   clause would have mixed. §10.4.2.3's `min` is the one non-linear step and it is applied
///   after the mixing, by the mask's own transfer table, where the clause puts it.
/// - The three weights sum to 1.0, so the grey of a grey is that grey.
///
/// One line, because the decision is `crate::colour`'s: an image's samples and a shading's
/// ramp take the same route and there is one function for all three (ADR 0220).
pub(super) fn convert(
    space: &ColourSpace,
    values: &[f32],
    black_point: BlackPoint,
    into: &Compositing,
) -> Color {
    into.paint(space, values, black_point.applies())
}

/// Reads the colour space a document's output intent describes.
///
/// Only a profile whose own space is one a PDF can name is useful here; an output intent
/// for a device with some other colourant model says nothing about `DeviceCMYK`.
pub(super) fn output_intent_space(document: &Document) -> Option<ColourSpace> {
    let catalog = document.catalog().ok()?;
    let intents = document.get_key(&catalog, "OutputIntents");
    // The specification is explicit that PDF carries no selector for choosing among
    // several, so the first usable one is taken.
    for intent in intents.as_array()? {
        let intent = document.resolve(intent);
        let Some(dict) = intent.as_dict() else {
            continue;
        };
        let profile = document.get_key(dict, "DestOutputProfile");
        let Some(stream) = profile.as_stream() else {
            continue;
        };
        if let Some(data) = document.decoded_stream_data(stream)
            && let Some(parsed) = crate::icc::Profile::parse(&data)
        {
            return Some(ColourSpace::Icc {
                profile: Box::new(parsed),
            });
        }
    }
    None
}

/// Whether this object is `[/ICCBased <stream>]` — §8.6.5.5's one-element array form.
///
/// The test [`Interpreter::icc_spaces`] needs, and it is deliberately narrow: it says that the
/// space's whole content is the stream, so the resource dictionary in force cannot change what
/// it means. Resolving the array is cheap — an array of two objects — and the profile behind it
/// is what costs.
fn is_icc_based(document: &Document, id: ObjectId) -> bool {
    document
        .get(id)
        .as_array()
        .and_then(<[Object]>::first)
        .and_then(Object::as_name)
        .is_some_and(|name| name.as_bytes() == b"ICCBased")
}
