//! Text under a redaction region: the font a `Tf` selected, the codes of a string, and the
//! rewrite of a showing operator that deletes the codes meeting the region, ISO 32000-2 §9.4.3,
//! §9.7.6.2 and §12.5.6.23.
//!
//! The code is the unit of removal. This module splits a string into codes by the font's
//! own rule, tests each code's placed quadrilateral against the region, deletes those that meet
//! it and restores their advance as a `TJ` adjustment so the surviving text does not slide. The
//! walk is held to the interpreter by a count of placed codes ([`super::Walk::quad_at`]).

use pdf_render::Transform;
use pdf_render::geom::Point;
use pdf_syntax::object::{Dictionary, Name, Object};
use std::sync::Arc;

use super::{
    ArrayElement, Codes, CompositeCodes, Operand, Type3Marks, Walk, bbox_of_points, linear, narrow,
    numbers, overlaps, plain_numbers, split_codes, write_number, write_pdf_string,
};

impl Walk<'_> {
    /// `Tf`: reads the font and settles how many bytes one of its codes is (§9.6, §9.7.5).
    pub(super) fn select_font(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let name = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Name(bytes) => Some(bytes.clone()),
            _ => None,
        });
        let size = plain_numbers(operands).last().copied();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "§9.3.1 text font size is a coordinate, far inside f32"
        )]
        {
            self.font_size = size.unwrap_or(0.0) as f32;
        }
        self.codes = match name {
            Some(name) => Some(self.font_codes(&name)?),
            None => None,
        };
        Ok(())
    }

    /// How the named font's codes are read, or a refusal for a font the walk will not cut.
    fn font_codes(&self, name: &[u8]) -> Result<Codes, String> {
        let fonts = self.document.get_key(&self.resources, "Font");
        let font = fonts
            .as_dict()
            .and_then(|fonts| fonts.get_by_name(&Name::new(name)))
            .map(|entry| self.document.resolve(entry));
        let Some(font) = font.as_ref().and_then(Object::as_dict) else {
            return Err(format!(
                "the content shows /{} which /Resources /Font does not define; the page is \
                 refused rather than guess a code width",
                String::from_utf8_lossy(name)
            ));
        };
        let subtype = self
            .document
            .get_key(font, "Subtype")
            .as_name()
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default();
        match subtype.as_slice() {
            b"Type1" | b"TrueType" | b"MMType1" => Ok(Codes::Simple),
            b"Type3" => self.type3_codes(font, name),
            b"Type0" => self.composite_codes(font, name),
            other => Err(format!(
                "the content shows a /{} font, whose code width this walk does not settle; the \
                 page is refused",
                String::from_utf8_lossy(other)
            )),
        }
    }

    /// A Type 3 font's codes (§9.6.4), with what locates each glyph's marks.
    fn type3_codes(&self, font: &Dictionary, name: &[u8]) -> Result<Codes, String> {
        let shown = String::from_utf8_lossy(name);
        let read =
            pdf_model::type3::Type3Font::read(self.document, font, &shown).map_err(|error| {
                format!(
                    "§9.6.4: the Type 3 font /{shown} cannot be read ({error}); the page is refused"
                )
            })?;
        let font_box = numbers(self.document, font, "FontBBox")
            .and_then(|values| match values.as_slice() {
                [llx, lly, urx, ury] => Some([
                    f64::from(*llx),
                    f64::from(*lly),
                    f64::from(*urx),
                    f64::from(*ury),
                ]),
                _ => None,
            })
            .filter(|values| values.iter().any(|value| *value != 0.0))
            .map(|[llx, lly, urx, ury]| [llx.min(urx), lly.min(ury), llx.max(urx), lly.max(ury)]);
        Ok(Codes::Type3(Arc::new(Type3Marks {
            font: read,
            font_box,
        })))
    }

    /// A Type 0 font's codes, as its `CMap`'s codespace ranges delimit them (§9.7.6.2).
    ///
    /// The `CMap` is resolved by the same function the font loader uses, so a predefined name, an
    /// embedded stream and its `/UseCMap` chain are all read one way. A vertical `CMap` carries the
    /// descendant's §9.7.4.3 displacements with it, because in writing mode 1 the gap a removal
    /// leaves is §9.4.4's `ty` ([`CompositeCodes::vertical`]).
    fn composite_codes(&self, font: &Dictionary, name: &[u8]) -> Result<Codes, String> {
        let shown = String::from_utf8_lossy(name);
        let cmap = pdf_font::composite_cmap(self.document, font, &shown).map_err(|error| {
            format!(
                "§9.7.6.2: the composite font /{shown}'s CMap cannot be resolved ({error}), so \
                 how many bytes each of its codes takes is unknown; the page is refused"
            )
        })?;
        let vertical = if cmap.wmode() == 0 {
            None
        } else {
            Some(
                pdf_font::VerticalDisplacements::of_composite(self.document, font).ok_or_else(
                    || {
                        format!(
                            "§9.7.4.3: the composite font /{shown} writes vertically and names no \
                             descendant CIDFont to state its vertical displacements; the page is \
                             refused"
                        )
                    },
                )?,
            )
        };
        Ok(Codes::Composite(Arc::new(CompositeCodes {
            cmap,
            vertical,
        })))
    }

    /// `Tj`/`'`: one string. `'` (§9.4.3) is `T*` then a show, which does not move the text
    /// matrix's linear part, so its rewriting is the same as `Tj`'s but keeps the line advance.
    pub(super) fn show_one(
        &mut self,
        operands: &[(Operand, usize)],
        keyword: &[u8],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((bytes, start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Str(bytes) => Some((bytes.clone(), *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        let elements = [ArrayElement::Str(bytes)];
        if let Some(inner) = self.rewrite_show(&elements)? {
            let prefix = if keyword == b"'" { "T* " } else { "" };
            let replacement = format!("{prefix}{inner} TJ").into_bytes();
            let end = keyword_start.saturating_add(keyword.len());
            self.edits.push((start, end, replacement));
        }
        Ok(())
    }

    /// `"` (§9.4.3): `aw ac string "`. A `"` with anything removed is refused, because
    /// restating its spacing exactly beside a `TJ` gap is not something this walk does.
    pub(super) fn show_quote(&mut self, operands: &[(Operand, usize)]) -> Result<(), String> {
        let Some(bytes) = operands.iter().find_map(|(operand, _)| match operand {
            Operand::Str(bytes) => Some(bytes.clone()),
            _ => None,
        }) else {
            return Ok(());
        };
        if self.string_meets_region(&bytes)? {
            return Err(
                "§9.4.3: the \" operator sets word and character spacing as it shows; a \
                        redaction inside it is refused rather than cut with the spacing wrong"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// `TJ` (§9.4.3): an array of strings and adjustments.
    pub(super) fn show_array(
        &mut self,
        operands: &[(Operand, usize)],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((elements, start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Array(elements) => Some((elements, *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        if let Some(inner) = self.rewrite_show(elements)? {
            let replacement = format!("{inner} TJ").into_bytes();
            let end = keyword_start.saturating_add(2);
            self.edits.push((start, end, replacement));
        }
        Ok(())
    }

    /// Rewrites a show operator's elements, deleting the codes that meet the region and
    /// restoring their advance as adjustments. `None` means nothing was removed, so the source
    /// bytes are left untouched (the "outside byte-identical" guarantee).
    fn rewrite_show(&mut self, elements: &[ArrayElement]) -> Result<Option<String>, String> {
        let codes = self.require_codes()?;
        let scale = self.advance_scale();
        let refusal = if self.font_size == 0.0 || self.horizontal == 0.0 {
            Some(
                "§9.4.4: text drawn at a zero size or horizontal scale meets the region; the \
                 page is refused rather than adjust an advance that cannot be expressed"
                    .to_owned(),
            )
        } else if self.char_spacing != 0.0 || self.word_spacing != 0.0 {
            Some(
                "§9.4.4: character or word spacing is in force where a glyph is removed; the \
                 page is refused rather than restate the spacing beside a TJ gap"
                    .to_owned(),
            )
        } else if !(scale.is_finite() && scale > 0.0) {
            Some(
                "§9.4.4: the text-rendering matrix is degenerate where a glyph is removed; the \
                 page is refused"
                    .to_owned(),
            )
        } else {
            None
        };

        if let Some(reason) = refusal {
            // Still count the codes so the interpreter's total is met, and refuse only where the
            // region is actually touched — a page whose problematic text is nowhere near a
            // redaction is redacted fine.
            return if self.count_only(elements, &codes)? {
                Err(reason)
            } else {
                Ok(None)
            };
        }

        let mut out = String::from("[");
        let mut any = false;
        for element in elements {
            match element {
                ArrayElement::Number(value) => write_number(&mut out, *value),
                ArrayElement::Str(bytes) => {
                    any |= self.rewrite_string(bytes, &codes, &mut out)?;
                }
            }
        }
        out.push(']');
        Ok(any.then_some(out))
    }

    /// Splits one string into runs of kept and removed codes, emitting kept runs as literals and
    /// removed runs as a single `TJ` adjustment; returns whether anything was removed.
    fn rewrite_string(
        &mut self,
        bytes: &[u8],
        codes: &Codes,
        out: &mut String,
    ) -> Result<bool, String> {
        let mut kept: Vec<u8> = Vec::new();
        let mut removed_first: Option<[f32; 8]> = None;
        let mut removed_last: [f32; 8] = [0.0; 8];
        // The removed run's §9.7.4.3 vertical displacement, in thousandths, in writing mode 1.
        let mut removed_w1y = 0.0f32;
        let vertical = match codes {
            Codes::Composite(composite) => composite.vertical.as_ref().map(|_| composite),
            Codes::Simple | Codes::Type3(_) => None,
        };
        let mut any = false;
        for (start, len) in split_codes(bytes, codes) {
            let quad = self.quad_at(self.code_index)?;
            let code = bytes
                .get(start..start.saturating_add(len))
                .unwrap_or_default();
            let marks = self.code_box(&quad, codes, code)?;
            let meets = self.regions.iter().any(|region| overlaps(*region, marks));
            self.code_index = self.code_index.saturating_add(1);
            if meets {
                if removed_first.is_none() {
                    if !kept.is_empty() {
                        write_pdf_string(out, &kept);
                        kept.clear();
                    }
                    removed_first = Some(quad);
                }
                removed_last = quad;
                if let Some(composite) = vertical
                    && let Some(displacements) = composite.vertical.as_ref()
                {
                    removed_w1y += displacements.w1y(composite.cid(code));
                }
                self.removed = self.removed.saturating_add(1);
                any = true;
            } else {
                if let Some(first) = removed_first.take() {
                    self.write_run_gap(out, (first, removed_last), vertical.map(|_| removed_w1y));
                    removed_w1y = 0.0;
                }
                kept.extend_from_slice(&bytes[start..start.saturating_add(len)]);
            }
        }
        if let Some(first) = removed_first.take() {
            self.write_run_gap(out, (first, removed_last), vertical.map(|_| removed_w1y));
        }
        if !kept.is_empty() {
            write_pdf_string(out, &kept);
        }
        Ok(any)
    }

    /// The `TJ` adjustment restoring a removed run's advance, in the writing mode the font writes
    /// in: `vertical_w1y` is the run's summed §9.7.4.3 vertical displacement in writing mode 1,
    /// `None` in writing mode 0.
    ///
    /// §9.4.4 computes `ty = (w1 − Tj/1000) × Tfs + Tc + Tw` in vertical writing, and the `TJ`
    /// number is "subtracted from the current horizontal or vertical coordinate, depending on the
    /// writing mode" (§9.4.3). With `Tc` and `Tw` zero — spacing in force refuses the page before
    /// this — the number that moves the pen by the removed glyphs' own `w1` is minus their sum,
    /// already in thousandths, and `Th` takes no part: it scales `tx` alone (ADR 1363).
    fn write_run_gap(
        &self,
        out: &mut String,
        run: ([f32; 8], [f32; 8]),
        vertical_w1y: Option<f32>,
    ) {
        match vertical_w1y {
            Some(w1y) => write_number(out, f64::from(-w1y)),
            None => self.write_gap(out, run.0, run.1),
        }
    }

    /// The `TJ` adjustment restoring a removed run's advance: §9.4.4's `w0` read back from the
    /// placed quadrilaterals rather than the font. The run's advance vector in the display
    /// list's space is the last removed glyph's far corner minus the first's near corner; that,
    /// projected onto the text-rendering x-axis and turned into thousandths of text space, is
    /// the number `TJ` subtracts to move the pen the same distance.
    fn write_gap(&self, out: &mut String, first: [f32; 8], last: [f32; 8]) {
        let axis = self.text_linear.then(self.ctm);
        let length_squared = axis.a * axis.a + axis.b * axis.b;
        if !(length_squared.is_finite() && length_squared > 0.0) {
            return;
        }
        let advance = (last[2] - first[0], last[3] - first[1]);
        let text_advance = (advance.0 * axis.a + advance.1 * axis.b) / length_squared;
        let denominator = self.font_size * self.horizontal;
        if denominator == 0.0 {
            return;
        }
        write_number(out, f64::from(-1000.0 * text_advance / denominator));
    }

    /// Counts a show operator's codes and whether any meets the region, without editing.
    fn count_only(&mut self, elements: &[ArrayElement], codes: &Codes) -> Result<bool, String> {
        let mut met = false;
        for element in elements {
            if let ArrayElement::Str(bytes) = element {
                met |= self.codes_meet_region(bytes, codes)?;
            }
        }
        Ok(met)
    }

    /// Tests every code of a string against the region without editing (the refused operators).
    fn string_meets_region(&mut self, bytes: &[u8]) -> Result<bool, String> {
        let codes = self.require_codes()?;
        self.codes_meet_region(bytes, &codes)
    }

    /// Consumes each code of a string, advancing the index, and answers whether any meets the
    /// region.
    fn codes_meet_region(&mut self, bytes: &[u8], codes: &Codes) -> Result<bool, String> {
        let mut met = false;
        for (start, len) in split_codes(bytes, codes) {
            let quad = self.quad_at(self.code_index)?;
            self.code_index = self.code_index.saturating_add(1);
            let code = bytes
                .get(start..start.saturating_add(len))
                .unwrap_or_default();
            let marks = self.code_box(&quad, codes, code)?;
            met |= self.regions.iter().any(|region| overlaps(*region, marks));
        }
        Ok(met)
    }

    /// The box in the display list's space a code is tested against: the quadrilateral the
    /// interpreter placed it in, widened for a Type 3 glyph to the box its description declares.
    ///
    /// A Type 3 glyph's description is a content stream that may mark anywhere the font's glyph
    /// space reaches, so the advance box alone would miss a glyph whose marks fall in the region
    /// while its advance does not. The declared box is carried by §9.6.4's own matrices: glyph
    /// space to text space by `/FontMatrix`, then §9.4.4's text rendering matrix — whose
    /// translation is the placed quadrilateral's first corner, glyph space's origin, because a
    /// Type 3 box's descent is zero — and whose linear part the walk tracks (ADR 1351).
    fn code_box(&self, quad: &[f32; 8], codes: &Codes, code: &[u8]) -> Result<[f32; 4], String> {
        let placed = bbox_of_points(quad);
        let Codes::Type3(marks) = codes else {
            return Ok(placed);
        };
        let Some(&byte) = code.first() else {
            return Ok(placed);
        };
        let stroke = (self.graphics.width, self.graphics.miter_limit);
        let Some([llx, lly, urx, ury]) = marks.glyph_box(self.document, self.page, stroke, byte)?
        else {
            return Ok(placed);
        };
        let text = Transform::new(
            self.font_size * self.horizontal,
            0.0,
            0.0,
            self.font_size,
            0.0,
            0.0,
        )
        .then(self.text_linear)
        .then(linear(self.ctm));
        let glyph_to_display = marks.font.font_matrix().then(text);
        let mut points: Vec<f32> = placed.to_vec();
        for (x, y) in [(llx, lly), (urx, lly), (urx, ury), (llx, ury)] {
            let point = glyph_to_display.apply(Point::new(narrow(x), narrow(y)));
            points.push(point.x + quad[0]);
            points.push(point.y + quad[1]);
        }
        Ok(bbox_of_points(&points))
    }

    /// The placed quadrilateral for a code index, or a refusal where the walk has outrun the
    /// interpreter (the code-count check, met early).
    fn quad_at(&self, index: usize) -> Result<[f32; 8], String> {
        self.quads.get(index).copied().ok_or_else(|| {
            "the content walk showed more codes than the interpreter placed; the page is refused"
                .to_owned()
        })
    }

    fn require_codes(&self) -> Result<Codes, String> {
        self.codes.clone().ok_or_else(|| {
            "the content shows text before any /Tf selects a font; the page is refused".to_owned()
        })
    }

    /// The length of the text-rendering matrix's x-axis in the display list's units, which the
    /// gap adjustment divides a display advance by.
    fn advance_scale(&self) -> f32 {
        let axis = self.text_linear.then(self.ctm);
        (axis.a * axis.a + axis.b * axis.b).sqrt()
    }
}
