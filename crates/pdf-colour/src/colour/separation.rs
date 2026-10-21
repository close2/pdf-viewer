//! §8.6.6.4 and §8.6.6.5: `Separation` and `DeviceN` spaces, their process and spot components, and §10.8.3's simulation.

use pdf_syntax::{Dictionary, Document, Name, Object};

use super::cie::{D50, srgb_to_xyz_d50};
use super::device_cmyk::channel;
use super::entries::{colourant_names, named};
use super::{ColourSpace, Reading, Separations, Tints};
use crate::function::Function;
use crate::icc::Rendering;

impl ColourSpace {
    /// §8.6.6.5's process dictionary, where it answers for **every** component of the space:
    /// the process `/ColorSpace` and which tint supplies each of its components.
    ///
    /// `None` — meaning the space reverts through its tint transform like any other `DeviceN`
    /// — wherever the clause leaves anything to decide. Each condition is one sentence of it:
    ///
    /// - **`/Subtype` is `NChannel`.** Table 70 defaults it to `DeviceN`, and "\[a] value of
    ///   DeviceN for the Subtype entry, or no value, shall mean that only the previous
    ///   features shall be supported" — the previous features being the tint transform.
    /// - **A `/Process` dictionary with both of Table 71's required entries**, its
    ///   `/ColorSpace` a space this tree reads and its `/Components` naming exactly that
    ///   space's components. Table 71 requires the correspondence — "\[a]n array of component
    ///   names that correspond, in order, to the components of the process colour space
    ///   specified in ColorSpace" — so a count that disagrees is a dictionary this reader
    ///   cannot index by, whatever else it says.
    /// - **Every name in the space's `names` array is one of `/Components`**, which is the
    ///   clause's own test for a spot colourant: "\[a]ny component not specified in the process
    ///   dictionary shall be considered to be a spot colourant." A spot colourant would need
    ///   its own `/Colorants` `Separation` *and* a rule for combining the result with the
    ///   process components, and the clause states no such rule — see [`Tints`].
    /// - **Every component of `/ColorSpace` is supplied by a name, unless the process space is
    ///   a CMYK one.** §8.6.6.5 gives the permission to that one family — "\[f]or a CMYK
    ///   colour space, a subset of the components may be present, and they may appear in any
    ///   order in the names array" — and §8.6.4.4 supplies the value a subtractive component
    ///   of zero has: "0.0 shall denote the complete absence of a process colourant". For any
    ///   other process space the standard states neither, so an omission is refused rather
    ///   than filled in. The test is the *space* and not the entry, because `/DeviceCMYK` and
    ///   a four-channel `ICCBased` space can be one another — §8.6.5.6's `/DefaultCMYK` and
    ///   §14.11.5's output intent substitute a profile for the name, and a process dictionary
    ///   may name that profile outright.
    ///
    /// **The reserved four are not read here**, and that is a case declined rather than one
    /// missed. §8.6.6.5 says "\[t]he reserved names Cyan , Magenta , Yellow , and Black shall
    /// always be considered to be process colours … they need not have entries in the process
    /// dictionary", so a CMYK `NChannel` space may name a component the `/Components` array
    /// omits; such a space keeps its tint transform, which is conforming, rather than gaining
    /// a second way of deciding which component a name is. `examples/nchannel_census` counts
    /// how many exist.
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.6.6.5 and Table 71 verbatim, and a quotation is not \
                  marked up"
    )]
    fn nchannel_process(
        document: &Document,
        attributes: Option<&Object>,
        names: &[Vec<u8>],
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<(Self, Vec<Option<usize>>)> {
        let attributes = document.resolve(attributes?);
        let attributes = attributes.as_dict()?;
        if document
            .get_key(attributes, "Subtype")
            .as_name()?
            .as_bytes()
            != b"NChannel"
        {
            return None;
        }
        let process = document.get_key(attributes, "Process");
        let process = process.as_dict()?;

        let stated = document.get_key(process, "ColorSpace");
        let space = Self::parse_at(
            document,
            &stated,
            resources,
            reading,
            depth.saturating_add(1),
        )?;
        let components = document.get_key(process, "Components");
        let components: Vec<Vec<u8>> = components
            .as_array()?
            .iter()
            .map(|entry| document.resolve(entry))
            .map(|entry| Some(entry.as_name()?.as_bytes().to_vec()))
            .collect::<Option<_>>()?;
        if components.len() != space.components() {
            return None;
        }

        let mut from = vec![None; components.len()];
        for (tint, name) in names.iter().enumerate() {
            let component = components.iter().position(|component| component == name)?;
            from[component] = Some(tint);
        }
        if from.iter().any(Option::is_none) && !Self::is_cmyk_process(&space) {
            return None;
        }
        Some((space, from))
    }

    /// Reads a `Separation` or a `DeviceN` space: ISO 32000-2 §8.6.6.4 and §8.6.6.5.
    ///
    /// Four readings, in the order the two clauses rank them — §8.6.6.4's two special
    /// colourant names, §10.8.3's separation simulation where a reader has asked for it,
    /// Table 71's process dictionary where it answers for every component, and the
    /// `alternateSpace` and `tintTransform` the clauses require of a display.
    pub(super) fn parse_separation(
        document: &Document,
        family: &[u8],
        items: &[Object],
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<Self> {
        let is_separation = family == b"Separation";
        let names = colourant_names(document, items.get(1), is_separation);
        let inputs = if is_separation { 1 } else { names.len() };

        // §8.6.6.4's two special names are decided *before* the alternate space and
        // the tint transform, because the clause requires both to be ignored: "A PDF
        // processor shall support Separation colour spaces with the colourant names
        // All and None on all devices, even if the devices are not capable of
        // supporting any others. When processing Separation spaces with either of
        // these colourant names PDF processors shall ignore the alternateSpace and
        // tintTransform parameters … although valid values shall still be provided."
        // Reading them first is what makes that true even where they are unreadable.
        if is_separation {
            match names.first().map(Vec::as_slice) {
                Some(b"All") => return Some(Self::AllColourants),
                Some(b"None") => return Some(Self::NoColourant { inputs: 1 }),
                _ => {}
            }
        } else if inputs > 0 && names.iter().all(|name| name.as_slice() == b"None") {
            // §8.6.6.5: "A DeviceN colour space whose component colourant names are
            // all None shall always discard its output, just the same as a Separation
            // colour space for None; it shall never revert to the alternate colour
            // space."
            return Some(Self::NoColourant { inputs });
        }

        if inputs == 0 {
            // §8.6.6.5's `names` array decides how many operands `scn` takes, so an
            // empty or missing one leaves the space undefined rather than degenerate.
            return None;
        }
        // §10.8.3's simulation, where the host has asked for it. Asked first
        // because it is the reading of §8.6.6.5's per-component sentence that reaches
        // a spot colourant, and the two below are what is left when it does not
        // apply: a space of process components alone, and the tint transform.
        if !is_separation
            && reading.separations() == Separations::Simulated
            && let Some((process, spots)) = Self::nchannel_separations(
                document,
                items.get(4),
                &names,
                resources,
                reading,
                depth,
            )
        {
            return Some(Self::Simulated {
                names: named(&names),
                process: process.map(Box::new),
                spots,
            });
        }
        // §8.6.6.5's per-component evaluation, asked before the `alternateSpace` and
        // `tintTransform` parameters are even read. That order is the clause's:
        // "PDF processors need not use the alternateSpace and tintTransform
        // parameters, and may instead use custom blending algorithms, along with
        // other information provided in the attributes dictionary if present. (If
        // the value of the Subtype entry in the attributes dictionary is NChannel ,
        // such information shall be present.)" So an `NChannel` space whose process
        // dictionary answers for every one of its components is complete without
        // them — including where the tint transform is one this tree cannot read,
        // which used to take the whole space with it.
        if !is_separation
            && let Some((alternate, from)) =
                Self::nchannel_process(document, items.get(4), &names, resources, reading, depth)
        {
            return Some(Self::Separation {
                inputs,
                names: named(&names),
                alternate: Box::new(alternate),
                tints: Tints::Process(from),
            });
        }
        let alternate = Self::parse_at(
            document,
            items.get(2)?,
            resources,
            reading,
            depth.saturating_add(1),
        )?;
        let transform = Function::parse(document, items.get(3)?).ok()?;
        Some(Self::Separation {
            inputs,
            names: named(&names),
            alternate: Box::new(alternate),
            tints: Tints::Transform(Box::new(transform)),
        })
    }

    /// Whether a space is "a CMYK colour space" in ISO 32000-2 §8.6.6.5's sense, which is the
    /// sense §10.8.1's "subtractive colourants" needs too.
    ///
    /// It is the space rather than the entry that answers: §8.6.5.6's `/DefaultCMYK` and
    /// §14.11.5's intent can make `/DeviceCMYK` a four-channel profile, and a process
    /// dictionary may name that same profile outright. §8.6.5.5's Table 67 admits four ICC
    /// data colour spaces and `CMYK` is the only one with four components, so the channel
    /// count tells them apart here (trap 6).
    fn is_cmyk_process(space: &Self) -> bool {
        matches!(space, Self::Cmyk)
            || matches!(space, Self::Icc { profile } if profile.channels() == 4)
    }

    /// Table 71's process colour space and the component names that index it, checked against
    /// the correspondence ISO 32000-2 §8.6.6.5's own table requires.
    ///
    /// > An array of component names that correspond, in order, to the components of the
    /// > process colour space specified in ColorSpace
    ///
    /// A count that disagrees is a dictionary no reader can index by, whatever else it says,
    /// so it answers `None` and the space keeps its tint transform.
    fn process_components(
        document: &Document,
        process: &Dictionary,
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<(Self, Vec<Vec<u8>>)> {
        let stated = document.get_key(process, "ColorSpace");
        let space = Self::parse_at(
            document,
            &stated,
            resources,
            reading,
            depth.saturating_add(1),
        )?;
        let components = document.get_key(process, "Components");
        let components: Vec<Vec<u8>> = components
            .as_array()?
            .iter()
            .map(|entry| document.resolve(entry))
            .map(|entry| Some(entry.as_name()?.as_bytes().to_vec()))
            .collect::<Option<_>>()?;
        if components.len() != space.components() {
            return None;
        }
        Some((space, components))
    }

    /// The separations §10.8.3 combines, which are ISO 32000-2 §8.6.6.5's per-component
    /// evaluation of an `NChannel` space.
    ///
    /// > For NChannel colour spaces, the components shall be evaluated individually; that is,
    /// > only the ones not present on the output device shall use the alternate colour space
    /// > of that component.
    ///
    /// On a display no component is present — §8.6.6.4 states that outright, "\[f]or an
    /// additive device such as a computer display, a Separation colour space never applies a
    /// process colourant directly; it always reverts to the alternate colour space" — so every
    /// component takes its own alternate, and what this answers is which space that is for
    /// each. `None` wherever the clause leaves one of them unanswered, and the space then
    /// keeps its tint transform *whole* rather than half of it, which is the clause's own
    /// guideline: a processor "should apply either the specified tint transformation function
    /// or invoke the same alternative blending algorithm for all DeviceN instances in the
    /// document".
    ///
    /// The conditions, each a sentence of the standard:
    ///
    /// - **`/Subtype` is `NChannel`.** Table 70 defaults it to `DeviceN`, and "\[a] value of
    ///   DeviceN for the Subtype entry, or no value, shall mean that only the previous
    ///   features shall be supported" — the previous features being the tint transform. The
    ///   sentence quoted above is stated for `NChannel` spaces and no other.
    /// - **A process dictionary, where there is one, is consistent and subtractive.** §10.8.1
    ///   makes separations a subtractive device's output — "\[i]f those are subtractive
    ///   colourants … then the output for the device may take a form called separations" — and
    ///   §10.8.3 step a)'s simulated device "supports subtractive process colourants". A
    ///   process space whose zero is black is not a separation, and §8.6.6.5 says such spaces
    ///   exist here: "\[f]or NChannel colour spaces, values for additive process colours (such
    ///   as RGB) shall be specified in their natural form, where 1.0 shall represent maximum
    ///   intensity of colour."
    /// - **Every component that is not a process one has a `/Colorants` entry.** "\[A]ny
    ///   component not specified in the process dictionary shall be considered to be a spot
    ///   colourant", and Table 70 makes the dictionary "\[r]equired if Subtype is NChannel and
    ///   the colour space includes spot colourants", each value "an array defining a
    ///   Separation colour space for that colourant". `/None` is the exception: it names no
    ///   colourant and "shall never be painted on the page".
    /// - **At least one component is a spot colourant.** Where they are all process ones the
    ///   combination does not arise, the space is complete as [`Tints::Process`], and running
    ///   §10.8.3 over it would replace a colour space's own account of its colourants with a
    ///   multiply of four filters in series. ADR 1103 is that reading and this preference does
    ///   not disturb it.
    #[expect(
        clippy::doc_markdown,
        reason = "the comment quotes §8.6.6.5, §8.6.6.4, §10.8.1, §10.8.3 and Table 70 \
                  verbatim, and a quotation is not marked up"
    )]
    fn nchannel_separations(
        document: &Document,
        attributes: Option<&Object>,
        names: &[Vec<u8>],
        resources: &Dictionary,
        reading: Reading<'_>,
        depth: usize,
    ) -> Option<(Option<Self>, Vec<Option<Self>>)> {
        let attributes = document.resolve(attributes?);
        let attributes = attributes.as_dict()?;
        if document
            .get_key(attributes, "Subtype")
            .as_name()?
            .as_bytes()
            != b"NChannel"
        {
            return None;
        }

        // Absent is not a failure here, unlike in `nchannel_process`: Table 70 makes
        // `/Process` "[r]equired if Subtype is NChannel and the colour space includes
        // components of a process colour space", so a space of spot colourants alone has none.
        let stated = document.get_key(attributes, "Process");
        let process = match stated.as_dict() {
            Some(dict) => Some(Self::process_components(
                document, dict, resources, reading, depth,
            )?),
            None => None,
        };
        if let Some((space, _)) = &process
            && !Self::is_cmyk_process(space)
        {
            return None;
        }

        let stated = document.get_key(attributes, "Colorants");
        let colorants = stated.as_dict();

        let mut from = process
            .as_ref()
            .map(|(_, components)| vec![None; components.len()]);
        let mut spots = Vec::with_capacity(names.len());
        let mut any_spot = false;
        for (tint, name) in names.iter().enumerate() {
            if name.as_slice() == b"None" {
                spots.push(None);
                continue;
            }
            if let Some((_, components)) = &process
                && let Some(component) = components.iter().position(|entry| entry == name)
            {
                if let Some(from) = from.as_mut()
                    && let Some(slot) = from.get_mut(component)
                {
                    *slot = Some(tint);
                }
                spots.push(None);
                continue;
            }
            // "The key shall match the colourant name given in that colour space" (Table 70): an
            // entry that is another space, or another colourant's, states nothing about this
            // one alone, and the space reverts as it does where the entry is missing (ADR 1338).
            let entry = document.resolve(colorants?.get_by_name(&Name::new(name.clone()))?);
            let items = entry.as_array()?;
            let is_name = |at: usize, wanted: &[u8]| {
                items
                    .get(at)
                    .map(|item| document.resolve(item))
                    .is_some_and(|item| item.as_name().is_some_and(|n| n.as_bytes() == wanted))
            };
            if !(is_name(0, b"Separation") && is_name(1, name)) {
                return None;
            }
            let space = Self::parse_at(
                document,
                &entry,
                resources,
                reading,
                depth.saturating_add(1),
            )?;
            any_spot = true;
            spots.push(Some(space));
        }
        if !any_spot {
            return None;
        }

        let process = process.map(|(space, _)| Self::Separation {
            inputs: names.len(),
            names: named(names),
            alternate: Box::new(space),
            tints: Tints::Process(from.unwrap_or_default()),
        });
        Some((process, spots))
    }

    /// ISO 32000-2 §10.8.3's steps b) and c) over the separations of a [`Self::Simulated`]
    /// space, answering the flat XYZ its step d) converts to the device.
    ///
    /// > - b) Convert each separation into "flat XYZ" (no gamma) and using a background matte
    /// >   of all white.
    /// > - c) Blend the resulting separations into a single result using a multiply blend
    /// >   (see "Table 133 -Variables used in the basic compositing formula").
    ///
    /// **The arithmetic, because the clause states the steps and not the algebra.** Step b)
    /// gives separation *i* at its tints an XYZ `Xᵢ` — flat meaning linear and ungammaed,
    /// which is what every XYZ in this crate already is, and the white matte meaning the
    /// separation is white where its colourant is absent, which is where a tint of zero puts a
    /// well-formed `Separation` (§8.6.6.4: "a tint value of 0.0 denotes the lightest colour
    /// that can be achieved with the given colourant"). Step c)'s multiply is Table 136's
    /// `B(cb, cs) = cb × cs`, and that entry's own NOTE 3 fixes the unit the product is taken
    /// in: "multiplying any colour with black produces black while multiplying with white
    /// leaves the original colour unchanged". So the matte's white `W` is the 1.0 of the
    /// formula and the product is of the ratios to it:
    ///
    /// ```text
    /// result = W × Π (Xᵢ / W)        componentwise over X, Y and Z
    /// ```
    ///
    /// Multiplying the `Xᵢ` themselves instead would darken the result by a factor of `W` for
    /// every separation past the first and leave blank paper at `W²` rather than at `W`, which
    /// is NOTE 3's sentence failing on its own terms. `W` is D50 because that is the white
    /// every XYZ in this crate is relative to.
    ///
    /// A separation whose space states no XYZ of its own — a tint transform into a device
    /// space — is taken in through this processor's CIE definition of that space (§10.3.2,
    /// ADR 0009), which is the one route trap 6 allows.
    pub(super) fn simulated_xyz(
        process: Option<&Self>,
        spots: &[Option<Self>],
        values: &[f32],
        depth: usize,
        rendering: Rendering,
    ) -> [f32; 3] {
        let deeper = depth.saturating_add(1);
        let mut product = [1.0_f32; 3];
        let multiply = |product: &mut [f32; 3], xyz: [f32; 3]| {
            for (axis, value) in product.iter_mut().enumerate() {
                *value *= xyz[axis] / D50[axis];
            }
        };
        if let Some(process) = process {
            multiply(
                &mut product,
                Self::flat_xyz(process, values, deeper, rendering),
            );
        }
        for (index, spot) in spots.iter().enumerate() {
            if let Some(spot) = spot {
                let tint = [channel(values.get(index).copied().unwrap_or(0.0))];
                multiply(&mut product, Self::flat_xyz(spot, &tint, deeper, rendering));
            }
        }
        [
            product[0] * D50[0],
            product[1] * D50[1],
            product[2] * D50[2],
        ]
    }

    /// One separation's flat XYZ: the space's own where it states one, and this processor's
    /// CIE definition of a device space where it does not.
    fn flat_xyz(space: &Self, values: &[f32], depth: usize, rendering: Rendering) -> [f32; 3] {
        space
            .cie_xyz_at(values, depth, rendering)
            .unwrap_or_else(|| srgb_to_xyz_d50(space.to_rgb_at(values, depth, rendering)))
    }

    /// ISO 32000-2 §10.8.3 step b) for one separation painted in this space at `values`: its flat
    /// XYZ divided by the matte's white, the factor it enters step c)'s multiply as.
    ///
    /// The separation's XYZ is `Self::flat_xyz`'s — the space's own where it states one and
    /// this processor's CIE definition of a device space where it does not (§10.3.2, ADR 0009) —
    /// taken under `rendering`, and the white is D50's, the one every XYZ in this crate is
    /// relative to. `simulated_xyz` states why the product is taken in that unit.
    #[must_use]
    pub fn flat_ratio(&self, values: &[f32], rendering: Rendering) -> [f32; 3] {
        let xyz = Self::flat_xyz(self, values, 0, rendering);
        std::array::from_fn(|axis| xyz[axis] / D50[axis])
    }
}
