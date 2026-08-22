//! When a shading pattern's colours are resolved, and how many documents can tell the difference.
//!
//! ISO 32000-2 §11.6.7 says which graphics state a shading pattern's definition is evaluated
//! under, and it is neither the `scn` that selects it nor the mark that paints it:
//!
//! > Any parameters that are not so specified shall be inherited from the graphics state that was
//! > in effect at the beginning of the content stream in which the shading pattern is set to be
//! > the current colour in the graphics state or in which the sh operator is used.
//!
//! > In the case of a shading pattern, the parameter values may be augmented by the contents of
//! > the ExtGState entry in the pattern dictionary (see 8.7.4, "Shading patterns"). Only those
//! > parameters that affect the sh operator, such as the current transformation matrix, black
//! > point compensation and rendering intent, shall be used.
//!
//! Two of the three parameters that sentence names decide a colour, and two more clauses put two
//! further quantities at the *mark* instead, so this counts the documents on which either moment
//! can change a pixel. Four figures, each an over-approximation stated in the direction that
//! cannot miss a witness:
//!
//! - **Table 75's `/ExtGState`**, counted exactly: a Type 2 pattern that states one, with the keys
//!   it states. `doc/conformance/ledger.toml`'s §8.7.4.1 row has claimed since the
//!   six-hundred-and-sixteenth session that no corpus document writes one; the crawl had never
//!   been asked.
//! - **§8.6.5.9's black point.** Compensation only ever moves a *CIE-based* conversion, so a
//!   pattern whose shading paints in a device space cannot see the parameter change however the
//!   state moves. The condition is therefore a Type 2 pattern whose shading's `/ColorSpace` is
//!   CIE-based **and** a document that states the parameter at all — Table 57's
//!   `/UseBlackPtComp`, or §8.6.5.8's `AbsoluteColorimetric`, which §8.6.5.9 makes an override of
//!   it.
//! - **§11.4.7's compositing target.** A shading pattern's colours are resolved into whatever the
//!   run is compositing in. §11.6.7 makes the pattern's definition a *non-isolated* group, and
//!   §11.7.2 gives a non-isolated group "their colour space from the nearest ancestor isolated
//!   parent group" — which is the group the mark is in, not the one the `scn` is in. The two
//!   differ only where a page composites somewhere other than the device's components, so the
//!   condition is a Type 2 pattern plus a group attributes dictionary whose `/CS` has four
//!   components.
//! - **§10.5's transfer function**, added in the six-hundred-and-sixtieth session with the rebuild
//!   at the mark. §11.7.5.2 puts the function at "the last (topmost) elementary graphics object
//!   enclosing that point", so a page that moves it between the `scn` and the mark paints
//!   different colours before and after that change. Whether a page *does* move it is a fact
//!   about a content stream and this census reads objects, so the condition is the
//!   over-approximation those objects can answer: a Type 2 pattern plus a Table 57 `/TR` or `/TR2`
//!   stating a real function — neither `/Identity` nor `/Default`, which state none.
//!
//! The last line names every document any condition matched, one per line, so that
//! `examples/raster_digest` can be run over exactly those and trap 11's rule — look at what a
//! count matched — costs nothing.
//!
//! ```sh
//! cargo run --release -p pdf-model --example pattern_state_census -- doc/pdf.js/test/pdfs/*.pdf
//! find corpus-cache/safedocs -name '*.pdf' -print0 | xargs -0 -n 2000 <the binary>
//! ```
//!
//! Documents are walked in parallel, one to a thread, because a `Document` is not `Sync`.

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]
#![expect(
    clippy::struct_excessive_bools,
    reason = "one flag per condition the census asks, each named after the clause it comes from; \
              a state machine over five independent questions would say less"
)]

use std::collections::{BTreeMap, BTreeSet};

use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};

use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// How far a colour space array is followed into another one.
const MAX_DEPTH: usize = 8;

/// What one document said.
#[derive(Default)]
struct Says {
    /// The document opened.
    opened: bool,
    /// How many `/PatternType 2` dictionaries it holds.
    type2: usize,
    /// How many of those state Table 75's `/ExtGState`, with the keys each states.
    ext_gstate: Vec<String>,
    /// How many of those paint in a CIE-based space, where black point compensation can move a
    /// colour at all.
    cie: usize,
    /// The document states Table 57's `/UseBlackPtComp` with a value other than `Default`.
    black_point: bool,
    /// The document names §8.6.5.8's `AbsoluteColorimetric`, which §8.6.5.9 treats as `OFF`.
    absolute: bool,
    /// The document states a group `/CS` of four components (§11.4.7, §11.6.6 Table 145).
    press: bool,
    /// The document states Table 57's `/TR` or `/TR2` as a real function (§10.5).
    transfer: bool,
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let said: Vec<(String, Says)> = paths
        .par_iter()
        .map(|path| {
            let name = path.rsplit('/').next().unwrap_or(path).to_owned();
            (name, read(path))
        })
        .collect();

    let mut documents = 0_usize;
    let mut patterns = 0_usize;
    let mut with_pattern = 0_usize;
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    let mut with_ext_gstate: Vec<String> = Vec::new();
    let mut cie_patterns = 0_usize;
    let mut black_point: Vec<String> = Vec::new();
    let mut press: Vec<String> = Vec::new();
    let mut transfer: Vec<String> = Vec::new();
    let mut witnesses: BTreeSet<String> = BTreeSet::new();

    for (name, says) in said {
        if !says.opened {
            continue;
        }
        documents = documents.saturating_add(1);
        patterns = patterns.saturating_add(says.type2);
        cie_patterns = cie_patterns.saturating_add(says.cie);
        if says.type2 == 0 {
            continue;
        }
        with_pattern = with_pattern.saturating_add(1);
        if !says.ext_gstate.is_empty() {
            with_ext_gstate.push(name.clone());
            for keys in &says.ext_gstate {
                let counter = states.entry(keys.clone()).or_default();
                *counter = counter.saturating_add(1);
            }
        }
        if says.cie > 0 && (says.black_point || says.absolute) {
            black_point.push(name.clone());
            witnesses.insert(name.clone());
        }
        if says.press {
            press.push(name.clone());
            witnesses.insert(name.clone());
        }
        if says.transfer {
            transfer.push(name.clone());
            witnesses.insert(name.clone());
        }
    }

    println!("{documents} document(s) opened");
    println!("  {with_pattern} hold a /PatternType 2 object ({patterns} of them)");
    println!("  {cie_patterns} of those patterns shade in a CIE-based space");
    println!(
        "  **{} state Table 75's /ExtGState**: {}",
        with_ext_gstate.len(),
        show(&with_ext_gstate)
    );
    println!("    keys: {states:?}");
    println!(
        "  **{} could see §8.6.5.9's black point move** (CIE-based shading + the parameter \
         stated): {}",
        black_point.len(),
        show(&black_point)
    );
    println!(
        "  **{} could see §11.4.7's compositing target move** (a group /CS of four components): \
         {}",
        press.len(),
        show(&press)
    );
    println!(
        "  **{} could see §10.5's transfer function move** (a real /TR or /TR2 stated anywhere): \
         {}",
        transfer.len(),
        show(&transfer)
    );
    println!("witnesses ({}), one per line:", witnesses.len());
    for name in &witnesses {
        println!("{name}");
    }
}

/// Up to forty names, so that a round can look at what a count matched (trap 11).
fn show(names: &[String]) -> String {
    if names.len() > 40 {
        return format!("{} document(s), too many to name", names.len());
    }
    names.join(" ")
}

/// One document's objects, read for everything above in a single pass.
fn read(path: &str) -> Says {
    let Ok(bytes) = std::fs::read(path) else {
        return Says::default();
    };
    let Ok(document) = Document::open(bytes) else {
        return Says::default();
    };
    let mut says = Says {
        opened: true,
        ..Says::default()
    };
    let numbers: Vec<u32> = document.xref().object_numbers().collect();
    for number in numbers {
        let object = document.get(ObjectId::new(number, 0));
        let dict = match &object {
            Object::Dictionary(dict) => dict.clone(),
            Object::Stream(stream) => stream.dict.clone(),
            _ => continue,
        };
        scan(&document, &dict, &mut says);
    }
    says
}

/// One object's dictionary, against each of the census's conditions.
fn scan(document: &Document, dict: &Dictionary, says: &mut Says) {
    if document.get_key(dict, "PatternType").as_integer() == Some(2) {
        says.type2 = says.type2.saturating_add(1);
        if let Some(state) = document.get_key(dict, "ExtGState").as_dict() {
            let keys: Vec<String> = state
                .iter()
                .map(|(key, _)| String::from_utf8_lossy(key.as_bytes()).into_owned())
                .collect();
            says.ext_gstate.push(format!("/{}", keys.join(" /")));
        }
        let shading = document.get_key(dict, "Shading");
        let shading = match &shading {
            Object::Stream(stream) => Some(stream.dict.clone()),
            other => other.as_dict().cloned(),
        };
        if let Some(shading) = shading {
            let space = document.get_key(&shading, "ColorSpace");
            if is_cie(document, &space, 0) {
                says.cie = says.cie.saturating_add(1);
            }
        }
    }

    // Table 57's entry. `Default` is the initial value and states nothing, so it cannot move the
    // parameter between the beginning of a content stream and a `scn`.
    if let Some(name) = document.get_key(dict, "UseBlackPtComp").as_name() {
        says.black_point |= name.as_bytes() != b"Default";
    }
    // §8.6.5.9: "If the current render intent of an object is AbsColorimetric then the value of
    // UseBlackPtComp shall be treated as OFF", so an absolute intent moves the parameter too.
    // Table 69's spelling is what a file states, through `/RI` here and through the `ri` operator
    // in a content stream — the second of which this census does not reach, and which is why the
    // figure is an over-approximation only in one direction.
    for key in ["RI", "Intent"] {
        if let Some(name) = document.get_key(dict, key).as_name() {
            says.absolute |= name.as_bytes() == b"AbsoluteColorimetric";
        }
    }

    // Table 57's `/TR` and `/TR2`, §10.5's transfer function. `/TR2` takes precedence where both
    // are stated, which makes no difference to a question about whether *either* states one. The
    // two names that mean "no function" are excluded, and so is any other name: §10.5 defines a
    // transfer function as "PDF function objects", so a name nobody defined leaves the parameter
    // where it was — the same reading `ext_gstate::Transfer::read` implements.
    for key in ["TR", "TR2"] {
        says.transfer |= !matches!(document.get_key(dict, key), Object::Null | Object::Name(_));
    }

    // §11.6.6 Table 145's `/CS`, on a group attributes dictionary or a page's `/Group`.
    if document
        .get_key(dict, "S")
        .as_name()
        .is_some_and(|name| name.as_bytes() == b"Transparency")
    {
        says.press |= components(document, &document.get_key(dict, "CS"), 0) == Some(4);
    }
}

/// Whether a shading's colour space is one black point compensation can move.
///
/// §8.6.5.9 puts compensation inside "CIE-based colour conversion", so the device families cannot
/// see it. An `Indexed` space is followed to its base and a `Separation` or `DeviceN` to its
/// alternate, because §8.7.4.4 makes the conversion happen there.
fn is_cie(document: &Document, space: &Object, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    match space {
        Object::Reference(_) => is_cie(document, &document.resolve(space), depth.saturating_add(1)),
        Object::Name(name) => matches!(name.as_bytes(), b"CalGray" | b"CalRGB" | b"Lab"),
        Object::Array(items) => {
            let Some(Object::Name(head)) = items.first() else {
                return false;
            };
            match head.as_bytes() {
                b"CalGray" | b"CalRGB" | b"Lab" | b"ICCBased" => true,
                b"Indexed" => items
                    .get(1)
                    .is_some_and(|base| is_cie(document, base, depth.saturating_add(1))),
                b"Separation" => items
                    .get(2)
                    .is_some_and(|alternate| is_cie(document, alternate, depth.saturating_add(1))),
                b"DeviceN" => items
                    .get(2)
                    .is_some_and(|alternate| is_cie(document, alternate, depth.saturating_add(1))),
                _ => false,
            }
        }
        _ => false,
    }
}

/// How many components a group's `/CS` names, where that can be told from the object alone.
fn components(document: &Document, space: &Object, depth: usize) -> Option<usize> {
    if depth > MAX_DEPTH {
        return None;
    }
    match space {
        Object::Reference(_) => {
            components(document, &document.resolve(space), depth.saturating_add(1))
        }
        Object::Name(name) => match name.as_bytes() {
            b"DeviceGray" | b"CalGray" | b"G" => Some(1),
            b"DeviceRGB" | b"CalRGB" | b"RGB" => Some(3),
            b"DeviceCMYK" | b"CMYK" => Some(4),
            _ => None,
        },
        Object::Array(items) => {
            let Some(Object::Name(head)) = items.first() else {
                return None;
            };
            match head.as_bytes() {
                b"ICCBased" => {
                    let stream = document.resolve(items.get(1)?);
                    let stream = stream.as_stream()?;
                    document
                        .get_key(&stream.dict, "N")
                        .as_integer()
                        .and_then(|n| usize::try_from(n).ok())
                }
                _ => components(document, items.first()?, depth.saturating_add(1)),
            }
        }
        _ => None,
    }
}
