//! ISO 32000-2 §10.4.2.4's black-generation and undercolour-removal functions.
//!
//! Table 57's `/BG`, `/BG2`, `/UCR` and `/UCR2` are read in `pdf_model::content`'s graphics-state
//! parameter dictionary and in a shading pattern's own (§11.6.7). This module holds the pair those
//! entries build and the conversion §10.4.2.4 defines over it, so that [`crate::colour`] can
//! separate a `DeviceRGB` colour with the file's own functions without depending on the content
//! interpreter — the same boundary [`crate::transfer`] keeps for §10.5 (ADR 1131).
//!
//! # Which conversion these are parameters of, and when it is this tree's
//!
//! §10.4.2.1 ranks two branches and this tree is on §10.3's: a colour goes into a page's four
//! components through the press's own `B2A` table, or through the right inverse of the ink cube
//! (ADRs 0009, 0042, 0263, 0796). That branch has a black-generation step of its own — the search
//! in [`crate::colour`] picks a black amount and solves the other three for the colour — and
//! §10.4.2.4 requires one: "Each device shall be configured with default values that are
//! appropriate for that device."
//!
//! §11.7.5.3 is what a file's own functions replace that default in:
//!
//! > When painting an elementary object with a DeviceRGB colour directly into a transparency
//! > group whose colour space is DeviceCMYK , the functions used shall be the current
//! > black-generation and undercolour-removal functions in effect in the graphics state at the
//! > time of the painting operation.
//!
//! So a stated function is not a request to change branches for the page; it is the one parameter
//! of one conversion, scoped to the colours §11.7.5.3's own sentence names — a `DeviceRGB` colour
//! painted into a group whose colour space is `DeviceCMYK`. ADR 1207 is the argument, and it
//! supersedes ADR 1069, whose premise was that this tree's conversion had no step for such a
//! function to replace.

use std::sync::Arc;

use pdf_syntax::{Dictionary, Document, Object};

use crate::function::Function;

/// ISO 32000-2 §10.4.2.4's two functions, as one graphics-state parameter pair.
///
/// The pair rather than two parameters because they are evaluated together and at one moment:
/// §10.4.2.4 calls both with the same operand — "the input of both the black-generation and
/// undercolour-removal functions shall be k , the minimum of the intermediate c , m , and y
/// values" — and the formula below reads both results in one expression. They are still *set*
/// independently, which is why each half is its own `Option`.
///
/// **A half that is `None` is the clause's nominal function**, `f(k) = k`, which §10.4.2.4
/// describes for each of the two in the same words — a black generation "may simply return its k
/// operand unchanged" and so may an undercolour removal. It is also the pair that makes
/// §10.4.2.5's conversion back an exact inverse, which is why `crate::colour`'s `rgb_to_cmyk`
/// already records it as this device's defaults *for this formula*. A state with neither half
/// stated is not one of these at all: the graphics state holds `None` there, and the conversion
/// stays §10.3's.
#[derive(Debug, Clone)]
pub struct BlackGeneration {
    /// `/BG` or `/BG2`, or the nominal `BG(k) = k`.
    black: Option<Arc<Function>>,
    /// `/UCR` or `/UCR2`, or the nominal `UCR(k) = k`.
    undercolour: Option<Arc<Function>>,
}

/// What one of Table 57's two pairs says about the parameter it sets.
///
/// Three answers, for [`crate::transfer::Stated`]'s reason: a dictionary that says nothing leaves
/// the function in force, and one that says `/Default` puts the *device's* back. Folding the two
/// together would carry a function on past the `q … gs … Q` written to stop it.
enum Said {
    /// Neither key, or a value this reader cannot make sense of.
    Unsaid,
    /// `/BG2` or `/UCR2` naming `Default`: the function in effect at the start of the page.
    Initial,
    /// A function dictionary or stream.
    Set(Arc<Function>),
}

impl BlackGeneration {
    /// Table 57's four entries, applied to the pair `current` already holds.
    ///
    /// Returns the pair in force after this dictionary, or `None` where neither half is a stated
    /// function — which is the state a page begins in and the state `/Default` restores.
    ///
    /// Each pair has its own precedence and §8.4.5's Table 57 states it twice in the same words:
    ///
    /// > If both BG and BG2 are present in the same graphics state parameter dictionary, BG2
    /// > shall take precedence.
    ///
    /// and, of the other pair, "[i]f both UCR and UCR2 are present in the same graphics state
    /// parameter dictionary, UCR2 shall take precedence". So `/BG2` is read first and a `/BG`
    /// beside it is not read at all — including where the `/BG2` is a value this reader rejects,
    /// because precedence decides which entry is *in force* rather than which one parses.
    #[must_use]
    pub fn read(
        document: &Document,
        state: &Dictionary,
        current: Option<&Self>,
    ) -> Option<Arc<Self>> {
        let held =
            |half: fn(&Self) -> &Option<Arc<Function>>| current.and_then(|pair| half(pair).clone());
        let take = |said: Said, held: Option<Arc<Function>>| match said {
            Said::Unsaid => held,
            Said::Initial => None,
            Said::Set(function) => Some(function),
        };
        let black = take(
            Self::said(document, state, "BG2", "BG"),
            held(|pair| &pair.black),
        );
        let undercolour = take(
            Self::said(document, state, "UCR2", "UCR"),
            held(|pair| &pair.undercolour),
        );
        (black.is_some() || undercolour.is_some()).then(|| Arc::new(Self { black, undercolour }))
    }

    /// One of Table 57's pairs, `second` — the `2` variant — in preference to `first`.
    ///
    /// §10.4.2.4 says what counts as a stated function and it is not a name: the two "shall be
    /// defined as PDF function dictionaries (see 7.10, "Functions")". The name `Default` is
    /// admissible only on the `2` variants, where Table 57 gives it a meaning — "the name Default
    /// , denoting the black-generation function that was in effect at the start of the page" —
    /// and at the start of a page no dictionary has run, so what it denotes is this device's own.
    /// A name on the plain entry is a value Table 57 does not give that entry, and a name nobody
    /// defined leaves the state alone rather than clearing it.
    fn said(document: &Document, state: &Dictionary, second: &str, first: &str) -> Said {
        let Some((key, entry)) = [second, first]
            .into_iter()
            .map(|key| (key, document.get_key(state, key)))
            .find(|(_, value)| !value.is_null())
        else {
            return Said::Unsaid;
        };
        match &entry {
            Object::Dictionary(_) | Object::Stream(_) => Function::parse(document, &entry)
                .ok()
                .map_or(Said::Unsaid, |function| Said::Set(Arc::new(function))),
            Object::Name(name) if key == second && name.as_bytes() == b"Default" => Said::Initial,
            _ => Said::Unsaid,
        }
    }

    /// Whether the caches keyed on a conversion are looking at this pair.
    ///
    /// The address, which is an identity because every holder of one of these holds the `Arc`:
    /// a cache key carries the pointer *and* the reference that keeps it alive, so no second pair
    /// can be allocated where this one was while a key naming it exists. Two clones of one pair
    /// are one pair, which is what a key has to say.
    #[must_use]
    pub fn identity(self: &Arc<Self>) -> usize {
        Arc::as_ptr(self) as usize
    }

    /// ISO 32000-2 §10.4.2.4's complete conversion from `DeviceRGB` to `DeviceCMYK`.
    ///
    /// The clause's own two steps, with this pair's functions. It prints the formula in
    /// mathematical italics no transcription survives, so it is set out here rather than quoted:
    /// `c = 1 − red`, `m = 1 − green`, `y = 1 − blue`, `k = min(c, m, y)`, and then
    /// `cyan = min(1, max(0, c − UCR(k)))` with magenta and yellow alongside, and
    /// `black = min(1, max(0, BG(k)))`.
    ///
    /// The operand is the clause's: "[t]he input of both the black-generation and
    /// undercolour-removal functions shall be k , the minimum of the intermediate c , m , and y
    /// values that have been computed by subtracting the original red , green , and blue
    /// components from 1.0."
    ///
    /// The outer clamps are the clause's too, and they are on the *results* rather than on what a
    /// function returned — Table 57 gives `/UCR` the range `[-1.0 1.0]` precisely so that it may
    /// "even [return] a negative amount, thereby adding to the total amount of colourant":
    ///
    /// > The final component values that result after applying black generation and undercolour
    /// > removal should be in the range 0.0 to 1.0. If a value falls outside this range, the
    /// > nearest valid value shall be substituted automatically without error indication.
    ///
    /// A function that returns nothing — an empty range, or an input outside its domain that the
    /// evaluator cannot answer for — falls back to the nominal `f(k) = k` rather than to zero,
    /// because zero is a value the clause gives a meaning ("0.0 for no black at all") and a
    /// function this reader could not evaluate has not asked for it.
    #[must_use]
    pub fn separate(&self, red: f32, green: f32, blue: f32) -> [f32; 4] {
        let cyan = 1.0 - red.clamp(0.0, 1.0);
        let magenta = 1.0 - green.clamp(0.0, 1.0);
        let yellow = 1.0 - blue.clamp(0.0, 1.0);
        let k = cyan.min(magenta).min(yellow);
        let at = |function: &Option<Arc<Function>>| {
            function.as_ref().map_or(k, |function| {
                function.eval(&[k]).first().copied().unwrap_or(k)
            })
        };
        let removed = at(&self.undercolour);
        let generated = at(&self.black);
        [
            (cyan - removed).clamp(0.0, 1.0),
            (magenta - removed).clamp(0.0, 1.0),
            (yellow - removed).clamp(0.0, 1.0),
            generated.clamp(0.0, 1.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::BlackGeneration;

    /// One page whose `/ExtGState` states Table 57's black generation, as PDF bytes.
    ///
    /// `entries` is written verbatim into the graphics state, so one builder serves a `/BG`, a
    /// `/UCR`, both, the `2` variants and `/Default`. Object 5 is `f(x) = 1 − x` and object 6 is
    /// `f(x) = x ÷ 2`, both §7.10.3's two-line exponential form.
    fn with(entries: &str) -> pdf_syntax::Document {
        use std::fmt::Write as _;
        let body = format!(
            "1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n\
             2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n\
             3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] \
             /Resources << /ExtGState << /G << /Type /ExtGState {entries} >> >> >> \
             /Contents 4 0 R >> endobj\n\
             4 0 obj << /Length 0 >> stream\n\nendstream endobj\n\
             5 0 obj << /FunctionType 2 /Domain [0 1] /C0 [1] /C1 [0] /N 1 >> endobj\n\
             6 0 obj << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [0.5] /N 1 >> endobj\n"
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

    /// The `/ExtGState` of the fixture above, walked by hand for `crate::transfer`'s reason:
    /// `pdf_model::Pages` lives a layer above this crate (ADR 1131 §4).
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

    /// ISO 32000-2 §10.4.2.4's conversion, with the functions this file states.
    ///
    /// Every expected value below is the clause's formula worked by hand on the stated
    /// functions, not a number read off this code. The colour is `0.2 0.7 0.4 rg`, which is the
    /// clause's own EXAMPLE: "[a] colour that is 0.2 red , 0.7 green , and 0.4 blue can also be
    /// expressed as 1.0 - 0.2 = 0.8 cyan , 1.0 - 0.7 = 0.3 magenta , and 1.0 - 0.4 = 0.6 yellow",
    /// so `c = 0.8`, `m = 0.3`, `y = 0.6` and `k = min(c, m, y) = 0.3`.
    #[test]
    fn the_stated_functions_are_the_ones_the_clause_calls() {
        // `/BG 5 0 R` is `BG(k) = 1 − k`, so `black = 1 − 0.3 = 0.7`; `/UCR` is unstated, so the
        // nominal `UCR(k) = k = 0.3` removes 0.3 from each of the other three.
        let document = with("/BG 5 0 R");
        let pair = BlackGeneration::read(&document, &state(&document), None)
            .expect("a /BG function is a stated black generation");
        let [cyan, magenta, yellow, black] = pair.separate(0.2, 0.7, 0.4);
        assert!((cyan - 0.5).abs() < 1e-5, "{cyan}");
        assert!((magenta - 0.0).abs() < 1e-5, "{magenta}");
        assert!((yellow - 0.3).abs() < 1e-5, "{yellow}");
        assert!((black - 0.7).abs() < 1e-5, "{black}");

        // `/UCR 6 0 R` is `UCR(k) = k ÷ 2 = 0.15`, and the nominal `BG(k) = k = 0.3`.
        let document = with("/UCR 6 0 R");
        let pair = BlackGeneration::read(&document, &state(&document), None)
            .expect("a /UCR function is a stated undercolour removal");
        let [cyan, magenta, yellow, black] = pair.separate(0.2, 0.7, 0.4);
        assert!((cyan - 0.65).abs() < 1e-5, "{cyan}");
        assert!((magenta - 0.15).abs() < 1e-5, "{magenta}");
        assert!((yellow - 0.45).abs() < 1e-5, "{yellow}");
        assert!((black - 0.3).abs() < 1e-5, "{black}");

        // Both stated: `BG(0.3) = 0.7` and `UCR(0.3) = 0.15`.
        let document = with("/BG 5 0 R /UCR 6 0 R");
        let pair = BlackGeneration::read(&document, &state(&document), None)
            .expect("both entries are stated");
        let [cyan, magenta, yellow, black] = pair.separate(0.2, 0.7, 0.4);
        assert!((cyan - 0.65).abs() < 1e-5, "{cyan}");
        assert!((magenta - 0.15).abs() < 1e-5, "{magenta}");
        assert!((yellow - 0.45).abs() < 1e-5, "{yellow}");
        assert!((black - 0.7).abs() < 1e-5, "{black}");

        // The nominal pair on its own is §10.4.2.4's first step and §10.4.2.5's exact inverse:
        // `cyan = c − k`, `black = k`, and `1 − min(1, cyan + black)` is `red` again.
        let nominal = BlackGeneration {
            black: None,
            undercolour: None,
        };
        let [cyan, magenta, yellow, black] = nominal.separate(0.2, 0.7, 0.4);
        assert!((cyan - 0.5).abs() < 1e-5, "{cyan}");
        assert!((magenta - 0.0).abs() < 1e-5, "{magenta}");
        assert!((yellow - 0.3).abs() < 1e-5, "{yellow}");
        assert!((black - 0.3).abs() < 1e-5, "{black}");
        assert!((1.0 - (cyan + black).min(1.0) - 0.2).abs() < 1e-5);
    }

    /// Table 57's precedence, its `/Default` and what is not a stated function at all.
    #[test]
    fn table_57_decides_which_of_each_pair_is_in_force() {
        // "[i]f both BG and BG2 are present in the same graphics state parameter dictionary, BG2
        // shall take precedence" — object 6 rather than object 5, so `BG(0.3) = 0.15`.
        let document = with("/BG 5 0 R /BG2 6 0 R");
        let pair = BlackGeneration::read(&document, &state(&document), None).expect("a pair");
        let [_, _, _, black] = pair.separate(0.2, 0.7, 0.4);
        assert!((black - 0.15).abs() < 1e-5, "{black}");

        // `/BG2 /Default` denotes "the black-generation function that was in effect at the start
        // of the page", which is this device's own — so it puts the pair back to nothing rather
        // than to a function, and a `/BG` beside it is outranked.
        let document = with("/BG 5 0 R /BG2 /Default");
        assert!(BlackGeneration::read(&document, &state(&document), None).is_none());

        // And it clears a function an earlier state set, which is the whole reason `/Default` is
        // not folded into "says nothing".
        let held = {
            let document = with("/BG 5 0 R");
            BlackGeneration::read(&document, &state(&document), None).expect("a pair")
        };
        let document = with("/BG2 /Default");
        assert!(BlackGeneration::read(&document, &state(&document), Some(&held)).is_none());

        // Saying nothing leaves the pair in force.
        let document = with("/ca 0.5");
        let carried = BlackGeneration::read(&document, &state(&document), Some(&held))
            .expect("an unrelated entry leaves the pair alone");
        let [_, _, _, black] = carried.separate(0.2, 0.7, 0.4);
        assert!((black - 0.7).abs() < 1e-5, "{black}");

        // §10.4.2.4 makes a stated function "defined as PDF function dictionaries (see 7.10,
        // "Functions")", so a name on the plain entry is not one — Table 57 gives `/BG` no name
        // value at all — and neither is a name nobody defined on the `2` variant.
        let document = with("/BG /Default");
        assert!(BlackGeneration::read(&document, &state(&document), None).is_none());
        let document = with("/BG2 /NoSuchThing");
        assert!(BlackGeneration::read(&document, &state(&document), None).is_none());
    }
}
