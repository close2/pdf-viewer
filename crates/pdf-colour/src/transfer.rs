//! ISO 32000-2 §10.5's transfer function, held where the colour stack can reach it.
//!
//! Table 57's `/TR`, `/TR2` and `/HT` — the three ways a file states one — are read in
//! `pdf_model::content`'s graphics-state parameter dictionary, and composed there by its
//! `TransferState`. This module holds only the functions those entries build and the
//! [`Stated`] answer one `/ExtGState` gives, so that [`crate::shading`] and [`crate::mesh`] depend
//! on the colour stack rather than on the content interpreter (ADR 1131).

use std::sync::Arc;

use pdf_render::Color;
use pdf_syntax::{Dictionary, Document, Object};

/// ISO 32000-2 §10.5's transfer function: one map per component, ready to apply.
///
/// `TransferState` is what an `/ExtGState` sets — Table 57's `/TR` and `/TR2` for the clause's
/// first bullet, `/HT` for its second — and this is the composition of the two that a colour goes
/// through.
///
/// > In the sequence of steps for processing colours, the PDF processor shall apply the transfer
/// > function after performing any needed conversions between colour spaces.
///
/// **Why a screen has one at all**: the standard never uses the phrase "marking device" —
/// §8.3.2.2's term is a "raster output device *such as a display or a printer*" — and §10.1's list
/// of rendering steps makes halftoning conditional on the device and the transfer function not.
/// §10.6.1 says it for the case of a screen outright: "\[h]alftoning is not required for such
/// devices; **after gamma correction by the transfer functions**, the colour components shall be
/// transmitted directly to the device."
///
/// One function or four, and **which device takes which**. The clause: "\[i]f only a single
/// function is specified, it shall apply to all components. An RGB device shall use the first
/// three, a monochrome device shall use the gray transfer function only, and a CMYK device shall
/// use all four." This tree draws on two devices, and both are kept:
///
/// - **the screen**, an RGB device, which takes red, green and blue ([`Transfer::apply`]);
/// - **§10.8.3's simulated press**, whose device "supports subtractive process colourants and
///   possibly spot colours", a CMYK device, which takes all four — cyan, magenta, yellow and black,
///   "their complements" — and a spot colourant's own ([`Transfer::on_the_press`]).
///
/// Both ends are additive by the clause's own rule — "the greater the numeric value, the lighter
/// the colour" — so on the screen nothing is subtracted from 1.0, and on the press the caller
/// hands over each subtractive component already "converted to additive form by subtracting it
/// from 1.0" (`pdf_render::TransferMap` does, where the planes are). ADR 3769.
///
/// **Public because a shading's colours are built outside this module.** Table 57 is read here and
/// nowhere else, so this is where the type belongs; but §10.5's subject is the component value, and
/// a ramp's stops, a mesh's corners and a function-based shading's grid are produced in
/// [`crate::shading`] and [`crate::mesh`]. Those take one of these rather than a closure, so the
/// clause is stated once and applied in every place a colour is made. The constructors are the
/// crate's own — [`Transfer::read`] for an `/ExtGState` and [`Transfer::from_components`] for a
/// halftone's per-component override — and both name a §10.5 source, so a caller passes a transfer
/// on rather than inventing one.
///
/// **A channel is optional because the clause has two sources and they meet per component.** The
/// second bullet lets a halftone dictionary carry a `TransferFunction` for one component and say
/// nothing about the others, and `TransferState` composes the two; a channel that neither source
/// names passes its component through untouched.
#[derive(Debug, Clone)]
pub struct Transfer {
    /// Red, green and blue, as the screen takes them.
    additive: [Channel; 3],
    /// Cyan, magenta, yellow and black, as §10.8.3's simulated press takes them: Table 57's four
    /// functions in its order, "red, green, blue, and gray or their complements".
    process: [Channel; 4],
    /// Every other component of the press: a spot colourant. §10.5: a halftone dictionary "is the
    /// only way to set transfer functions for nonprimary colour components", and a single function
    /// "shall apply to all components" — so Table 57's array of four leaves this `None`.
    nonprimary: Channel,
}

/// One component's function, `Arc` so that one stated function fills several without a clone.
type Channel = Option<Arc<crate::function::Function>>;

/// A process colourant of §10.8.3's simulated press, in Table 57's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Process {
    /// The first function's component on a CMYK device.
    Cyan,
    /// The second.
    Magenta,
    /// The third.
    Yellow,
    /// The fourth, "gray" on an additive device.
    Black,
}

impl Process {
    /// The four, in Table 57's order.
    pub const ALL: [Self; 4] = [Self::Cyan, Self::Magenta, Self::Yellow, Self::Black];

    /// The index of this colourant's function in Table 57's array.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Cyan => 0,
            Self::Magenta => 1,
            Self::Yellow => 2,
            Self::Black => 3,
        }
    }
}

impl Transfer {
    /// Table 57's two entries, read from an `/ExtGState`, with `/TR2` in preference to `/TR`.
    ///
    /// Table 57 makes that precedence explicit — "\[i]f both TR and TR2 are present in the same
    /// graphics state parameter dictionary, TR2 shall take precedence" — and both take a
    /// function, an array of four, or a name.
    ///
    /// Three answers, not two, which is what [`Stated`] exists to say: the state says nothing, the
    /// state turns an inherited transfer **off** (`/Identity`, or `/TR2`'s `/Default`), or the
    /// state sets one. Folding the middle into the first would leave an inherited transfer running
    /// through a `q … /Identity gs … Q` that exists to stop it.
    pub fn read(document: &Document, state: &Dictionary) -> Stated {
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
        let parse = |object: &Object| {
            crate::function::Function::parse(document, object)
                .ok()
                .map(Arc::new)
        };
        match entry.as_array() {
            // "[A]n array of four separate transfer functions, one each for red, green, blue, and
            // gray or their complements cyan, magenta, yellow, and black" — the screen takes the
            // first three and the simulated press all four. An array of three states no fourth,
            // and its black passes through.
            Some(items) if items.len() >= 3 => {
                let mut out: [Channel; 4] = [None, None, None, None];
                for (slot, item) in out.iter_mut().zip(items.iter()) {
                    let Some(function) = parse(&document.resolve(item)) else {
                        return Stated::Unsaid;
                    };
                    *slot = Some(function);
                }
                let [red, green, blue, gray] = out;
                Stated::Set(Self {
                    additive: [red.clone(), green.clone(), blue.clone()],
                    process: [red, green, blue, gray],
                    nonprimary: None,
                })
            }
            // An array of any other length is not what the clause states, and a state this reader
            // cannot make sense of leaves the one in force alone.
            Some(_) => Stated::Unsaid,
            // "If only a single function is specified, it shall apply to all components."
            None => {
                let Some(one) = parse(&entry) else {
                    return Stated::Unsaid;
                };
                Stated::Set(Self {
                    additive: [Some(one.clone()), Some(one.clone()), Some(one.clone())],
                    process: [
                        Some(one.clone()),
                        Some(one.clone()),
                        Some(one.clone()),
                        Some(one.clone()),
                    ],
                    nonprimary: Some(one),
                })
            }
        }
    }

    /// The colour the screen would receive, with the alpha untouched: an RGB device, which takes
    /// the first three functions.
    ///
    /// Alpha is not a colour component: §10.5 speaks of "the value of a colour component in the
    /// device's native colour space", and §11's shape and opacity are a different quantity in a
    /// different clause.
    #[must_use]
    pub fn apply(&self, colour: Color) -> Color {
        Color {
            r: map(self.additive[0].as_ref(), colour.r),
            g: map(self.additive[1].as_ref(), colour.g),
            b: map(self.additive[2].as_ref(), colour.b),
            a: colour.a,
        }
    }

    /// The value §10.8.3's simulated press would receive for one process colourant, given and
    /// returned in additive form — `1 − c` — as §10.5 requires of a subtractive component.
    #[must_use]
    pub fn on_the_press(&self, colourant: Process, additive: f32) -> f32 {
        map(self.process[colourant.index()].as_ref(), additive)
    }

    /// The value the simulated press would receive for a spot colourant, in additive form.
    #[must_use]
    pub fn on_a_spot_colourant(&self, additive: f32) -> f32 {
        map(self.nonprimary.as_ref(), additive)
    }

    /// The transfer a `DeviceGray` colour takes on §10.8.3's simulated press, a CMYK device:
    /// this one's black function alone. ISO 32000-2 §10.5:
    ///
    /// > When the current colour space is DeviceGray and the output device's native colour space
    /// > is DeviceCMYK , a PDF processor shall use only the gray transfer function. The normal
    /// > conversion from DeviceGray to DeviceCMYK produces 0.0 for the cyan, magenta, and yellow
    /// > components. These components shall not be passed through their respective transfer
    /// > functions but are rendered directly, producing output containing no coloured inks.
    ///
    /// The screen's three are kept, because the special case is the CMYK device's and the screen
    /// is an RGB one; a `DeviceGray` colour has no spot component, so the nonprimary function is
    /// kept as it is and never asked. ADR 3770.
    #[must_use]
    pub fn grey_alone(&self) -> Self {
        let [_, _, _, gray] = self.process.clone();
        Self {
            additive: self.additive.clone(),
            process: [None, None, None, gray],
            nonprimary: self.nonprimary.clone(),
        }
    }

    /// Builds a transfer from per-component functions, for `TransferState`'s composition of
    /// §10.5's two bullets in `pdf_model::content` — the one place besides [`Transfer::read`] that
    /// makes one, combining Table 57's `/TR` with a halftone's per-component override. `additive`
    /// is the screen's red, green and blue, `process` the press's cyan, magenta, yellow and black,
    /// and `nonprimary` its spot colourants'.
    #[must_use]
    pub fn from_components(
        additive: [Option<Arc<crate::function::Function>>; 3],
        process: [Option<Arc<crate::function::Function>>; 4],
        nonprimary: Option<Arc<crate::function::Function>>,
    ) -> Self {
        Self {
            additive,
            process,
            nonprimary,
        }
    }

    /// The screen's function for red (0), green (1) or blue (2), so the composition above can read
    /// the stated transfer a halftone left in force for that component.
    #[must_use]
    pub fn additive_channel(&self, index: usize) -> Option<Arc<crate::function::Function>> {
        self.additive.get(index).cloned().flatten()
    }

    /// The press's function for one process colourant, for the same composition.
    #[must_use]
    pub fn process_channel(&self, colourant: Process) -> Option<Arc<crate::function::Function>> {
        self.process[colourant.index()].clone()
    }

    /// The press's function for a spot colourant, for the same composition.
    #[must_use]
    pub fn nonprimary_channel(&self) -> Option<Arc<crate::function::Function>> {
        self.nonprimary.clone()
    }

    /// Whether any component of either device is mapped.
    #[must_use]
    pub fn maps_anything(&self) -> bool {
        self.additive.iter().any(Option::is_some)
            || self.process.iter().any(Option::is_some)
            || self.nonprimary.is_some()
    }
}

/// One component through its function, clamped to the clause's range at both ends; a channel no
/// source names passes its component through. ISO 32000-2 §10.5:
///
/// > The transfer function shall be called with a numeric operand in the range 0.0 to 1.0 and
/// > shall return a number in the same range.
fn map(channel: Option<&Arc<crate::function::Function>>, value: f32) -> f32 {
    let Some(function) = channel else {
        return value;
    };
    function
        .eval(&[value.clamp(0.0, 1.0)])
        .first()
        .copied()
        .map_or(value, |out| out.clamp(0.0, 1.0))
}

/// What an `/ExtGState` said about §10.5's transfer function.
///
/// Three answers rather than two, because "says nothing" and "says `/Identity`" are different
/// instructions: the first leaves whatever is in force, and the second is how a file turns an
/// inherited transfer off. `issue6931_reduced.pdf` uses both — one state sets three functions and
/// the next sets `/Identity` — so a reader that could not tell them apart would carry the transfer
/// on past the object it was written for.
#[derive(Debug)]
pub enum Stated {
    /// The dictionary has neither entry, or has one this reader cannot make sense of.
    Unsaid,
    /// `/Identity`, or `/TR2`'s `/Default`: no transfer from here on.
    None,
    /// A function, or four of them.
    Set(Transfer),
}

#[cfg(test)]
mod tests {
    use super::{Process, Stated, Transfer};

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
    ///
    /// Walks the page tree by hand — catalogue, page tree, first kid, resources — rather than
    /// through `pdf_model::Pages`, which lives a layer above this colour crate (ADR 1131 §4).
    /// The fixture has one page, so the first kid is it.
    fn state(document: &pdf_syntax::Document) -> pdf_syntax::Dictionary {
        let catalog = document.catalog().expect("the fixture has a catalogue");
        let tree = document.get_key(&catalog, "Pages");
        let tree = tree.as_dict().expect("the catalogue names a page tree");
        let kids = document.get_key(tree, "Kids");
        let kids = kids.as_array().expect("the page tree has kids");
        let first = kids.first().expect("one page");
        let page = document.resolve(first);
        let page = page.as_dict().expect("the first kid is a page");
        let resources = document.get_key(page, "Resources");
        let resources = resources.as_dict().expect("the page has resources");
        let graphics = document.get_key(resources, "ExtGState");
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

        // "[A] CMYK device shall use all four": §10.8.3's simulated press takes the fourth on its
        // black, and an array of four states nothing for a spot colourant: a halftone is "the only
        // way to set transfer functions for nonprimary colour components".
        assert!((transfer.on_the_press(Process::Black, 0.25) - 0.75).abs() < 1e-4);
        assert!((transfer.on_the_press(Process::Cyan, 0.25) - 0.75).abs() < 1e-4);
        assert!((transfer.on_a_spot_colourant(0.25) - 0.25).abs() < 1e-6);
        // "When the current colour space is DeviceGray and the output device's native colour space
        // is DeviceCMYK , a PDF processor shall use only the gray transfer function."
        let grey = transfer.grey_alone();
        assert!((grey.on_the_press(Process::Cyan, 0.25) - 0.25).abs() < 1e-6);
        assert!((grey.on_the_press(Process::Black, 0.25) - 0.75).abs() < 1e-4);
        assert!(
            (grey
                .apply(pdf_render::Color {
                    r: 0.25,
                    g: 0.25,
                    b: 0.25,
                    a: 1.0
                })
                .r
                - 0.75)
                .abs()
                < 1e-4
        );

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
        assert!((transfer.on_the_press(Process::Black, 0.25) - 0.75).abs() < 1e-4);
        assert!((transfer.on_a_spot_colourant(0.25) - 0.75).abs() < 1e-4);

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
