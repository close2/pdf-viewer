//! ISO 32000-2 §10.5's transfer function, held where the colour stack can reach it.
//!
//! Table 57's `/TR`, `/TR2` and `/HT` — the three ways a file states one — are read in
//! `pdf_model::content`'s graphics-state parameter dictionary, and composed there by its
//! `TransferState`. This module holds only the function triple those entries build and the
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
/// clause is stated once and applied in every place a colour is made. The two constructors are
/// the crate's own — [`Transfer::read`] for an `/ExtGState` and [`Transfer::from_channels`] for a
/// halftone's per-component override — and both name a §10.5 source, so a caller passes a transfer
/// on rather than inventing one.
///
/// **A channel is optional because the clause has two sources and they meet per component.** The
/// second bullet lets a halftone dictionary carry a `TransferFunction` for one component and say
/// nothing about the others, and `TransferState` composes the two; a channel that neither source
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

    /// Builds a transfer from three per-component functions, for `TransferState`'s composition of
    /// §10.5's two bullets in `pdf_model::content` — the one place besides [`Transfer::read`] that
    /// makes one, combining Table 57's `/TR` with a halftone's per-component override.
    #[must_use]
    pub fn from_channels(channels: [Option<Arc<crate::function::Function>>; 3]) -> Self {
        Self { channels }
    }

    /// One component's function, red then green then blue, so the composition above can read the
    /// stated transfer a halftone left in force for that component.
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<Arc<crate::function::Function>> {
        self.channels[index].clone()
    }
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
