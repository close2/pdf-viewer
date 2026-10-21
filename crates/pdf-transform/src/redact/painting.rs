//! Path construction and painting under a redaction region, ISO 32000-2 §8.5 and §12.5.6.23.
//!
//! The construction operators (§8.5.2) build a [`PathObject`] a subpath at a time; the painting
//! operator (§8.5.3) decides whether the object is kept, dropped or cut to the complement of the
//! region ([`super::paths`]), and a stroke is restated as the outline it paints (§8.5.3.2).

use pdf_syntax::object::{Dictionary, Name, Object};

use super::{
    ARC_TOLERANCE, ArrayElement, Operand, PathObject, Walk, admits_a_cut, cut_to_complement,
    mapping, overlaps, paths, plain_numbers, split_open, split_subpaths,
};

impl Walk<'_> {
    /// `m` (§8.5.2.1): begins a new subpath at the given point.
    pub(super) fn begin_subpath(&mut self, operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        if let Some(path) = self.path.as_mut()
            && let [x, y] = numbers.as_slice()
        {
            path.begin(*x, *y);
        }
    }

    /// `l` (§8.5.2.1): appends a straight segment to the subpath under construction.
    pub(super) fn extend_subpath(&mut self, operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        if let Some(path) = self.path.as_mut()
            && let [x, y] = numbers.as_slice()
        {
            path.line(*x, *y);
        }
    }

    /// `c`, `v`, `y` (§8.5.2.2): a cubic Bézier, in the three spellings the clause gives it.
    ///
    /// `c` states all three remaining control points, and the curve runs from the current point
    /// to the last of them.
    /// `v` states two pairs and uses the current point as its first control point; `y` states two
    /// pairs and uses the final point as its second. All three are one curve, and the cut splits
    /// it at the parameter where it crosses the region's edge rather than flattening it
    /// (`paths::crossings`, ADR 1236).
    pub(super) fn curve_subpath(&mut self, keyword: &[u8], operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        let Some(path) = self.path.as_mut() else {
            return;
        };
        match (keyword, numbers.as_slice()) {
            (b"c", [x1, y1, x2, y2, x3, y3]) => path.curve(
                kurbo::Point::new(*x1, *y1),
                kurbo::Point::new(*x2, *y2),
                kurbo::Point::new(*x3, *y3),
            ),
            (b"v", [x2, y2, x3, y3]) => {
                let current = path.current_point;
                path.curve(
                    current,
                    kurbo::Point::new(*x2, *y2),
                    kurbo::Point::new(*x3, *y3),
                );
            }
            (b"y", [x1, y1, x3, y3]) => {
                let end = kurbo::Point::new(*x3, *y3);
                path.curve(kurbo::Point::new(*x1, *y1), end, end);
            }
            _ => {}
        }
    }

    /// `re` (§8.5.2.1): a complete rectangular subpath, which closes itself.
    pub(super) fn add_rectangle(&mut self, operands: &[(Operand, usize)]) {
        self.open_path(operands);
        let numbers = plain_numbers(operands);
        if let Some(path) = self.path.as_mut()
            && let [x, y, w, h] = numbers.as_slice()
        {
            path.rectangle(*x, *y, *w, *h);
        }
    }

    /// `h` (§8.5.2.1): closes the subpath under construction.
    pub(super) fn close_subpath(&mut self) {
        if let Some(path) = self.path.as_mut() {
            path.close();
        }
    }

    /// `d` (§8.4.3.6): the dash pattern and phase.
    ///
    /// The pattern decides which stretches of a path are marked at all, so it is applied
    /// **before** the outline is cut: `kurbo::stroke` takes it in the style and emits each dash
    /// as its own contour of the outline, which the cut then treats like any other geometry.
    /// A malformed array — one with a negative entry, or one whose entries are all zero — leaves
    /// the solid line §8.4.3.6 says a conforming file would have written.
    pub(super) fn set_dash(&mut self, operands: &[(Operand, usize)]) {
        let Some((Operand::Array(items), _)) = operands.first() else {
            return;
        };
        let dashes: Vec<f64> = items
            .iter()
            .filter_map(|item| match item {
                ArrayElement::Number(value) => Some(*value),
                ArrayElement::Str(_) => None,
            })
            .collect();
        if dashes.iter().any(|dash| *dash < 0.0) || dashes.iter().all(|dash| *dash == 0.0) {
            self.graphics.dashes = Vec::new();
            self.graphics.dash_phase = 0.0;
            return;
        }
        self.graphics.dashes = dashes;
        self.graphics.dash_phase = plain_numbers(operands).first().copied().unwrap_or_default();
    }

    /// Opens a path object if none is open, remembering where its first operand starts — the
    /// offset a cut's replacement bytes begin at.
    fn open_path(&mut self, operands: &[(Operand, usize)]) {
        if self.path.is_none() {
            let start = operands.first().map_or(0, |(_, at)| *at);
            self.path = Some(PathObject::new(start, self.ctm));
        }
    }

    /// A painted path that meets the region has the region's share of its marks **cut out**
    /// (§12.5.6.23: the content is removed, not covered), or the page is refused by name where
    /// the cut cannot be taken exactly (trap 5, principle 1).
    ///
    /// The surviving geometry replaces the whole path object — its construction operators and
    /// its painting operator — so the coordinates that described the removed marks are gone from
    /// the file rather than clipped away. [`paths`] is the construction and its exactness
    /// argument; what is decided here is which paths it may be applied to.
    ///
    /// **A painting operator states up to two marks and each is cut on its own.** §8.5.3 gives
    /// `B` and its three relatives a fill *and* a stroke, and the two are different regions in
    /// different colours: the fill is the path's interior with §8.5.3.1's implicit close, the
    /// stroke is the outline §8.5.3.2 marks along it. So the replacement is the cut fill painted
    /// first and the cut outline second, which is the clause's own order — "\[f]ill and then
    /// stroke the path". ADR 1236.
    pub(super) fn paint_path(
        &mut self,
        keyword: &[u8],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some(mut path) = self.path.take() else {
            return Ok(());
        };
        path.finish();
        self.narrow_clip(&path);
        let Some(bbox) = path.bbox else {
            return Ok(());
        };
        // §8.5.3.1 and §8.5.3.2: which of the two marks this operator makes, and whether it
        // closes the current subpath first ("s … the same effect as the sequence h S").
        let (fill, stroked, closes) = match keyword {
            b"f" | b"F" => (Some("f"), false, false),
            b"f*" => (Some("f*"), false, false),
            b"S" => (None, true, false),
            b"s" => (None, true, true),
            b"B" => (Some("f"), true, false),
            b"B*" => (Some("f*"), true, false),
            b"b" => (Some("f"), true, true),
            b"b*" => (Some("f*"), true, true),
            _ => return Ok(()),
        };
        // A stroke's marks reach past the path's own points by half the line width and by what a
        // join adds, so the box the region is tested against is widened by that much first.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a line width carried into device space is a page coordinate, inside f32"
        )]
        let widened = {
            let reach = if stroked {
                (self.graphics.stroke_reach() * mapping(path.ctm).norm()) as f32
            } else {
                0.0
            };
            [
                bbox[0] - reach,
                bbox[1] - reach,
                bbox[2] + reach,
                bbox[3] + reach,
            ]
        };
        if !self.regions.iter().any(|region| overlaps(*region, widened)) {
            // The path is nowhere near a redaction: its bytes cross the output untouched.
            return Ok(());
        }
        admits_a_cut(&path)?;
        if fill.is_some()
            && let Some(name) = self.graphics.fill_pattern.clone()
        {
            self.pattern_admits_a_cut(&name)?;
        }
        if stroked && let Some(name) = self.graphics.stroke_pattern.clone() {
            self.pattern_admits_a_cut(&name)?;
        }
        let regions = self.region_bounds();
        let to_display = mapping(path.ctm);
        let mut replacement: Vec<u8> = Vec::new();
        if let Some(fill) = fill {
            let cut = cut_to_complement(&path.filled_rings(), &regions, to_display)?;
            if !cut.is_empty() {
                let mut text = String::new();
                paths::write_polygons(&mut text, &cut);
                text.push_str(fill);
                text.push('\n');
                replacement.extend_from_slice(text.as_bytes());
            }
        }
        if stroked && self.graphics.width <= 0.0 {
            replacement.extend_from_slice(&self.cut_hairline(&path, closes, &regions, to_display)?);
        } else if stroked {
            let outline = self.stroke_outline(&path, closes);
            let cut = cut_to_complement(&outline, &regions, to_display)?;
            if !cut.is_empty() {
                let colour = self.graphics.non_stroking_colour();
                let restated = self.restate_stroke();
                let mut text = String::new();
                paths::write_polygons(&mut text, &cut);
                // Balanced inside the replacement (§8.4.2), so the fill colour, alpha and
                // overprint the producer set for whatever comes next are the ones that come back.
                replacement.extend_from_slice(b"q\n");
                replacement.extend_from_slice(&restated);
                replacement.extend_from_slice(&colour);
                replacement.extend_from_slice(text.as_bytes());
                // §8.5.3.2's marks are the region the outline encloses, and `kurbo::stroke` winds
                // every contour of that outline the same way — so the nonzero rule is the one
                // that paints it, and `f*` would put holes where the outline overlaps itself.
                replacement.extend_from_slice(b"f\nQ\n");
            }
        }
        // §8.5.4's boundary, re-stated after the marks it used to accompany. The producer's own
        // bytes for the whole path object, then `n`: the marks above were painted under the clip
        // already in force, and this sets the clip to its intersection with the path the producer
        // constructed — which is the order the clause states and the same clip every later mark
        // on the page is held to (ADR 1248).
        if let Some(bytes) = path.clip_bytes.as_ref() {
            replacement.extend_from_slice(bytes);
            replacement.extend_from_slice(b"\nn\n");
        }
        self.paths_cut = self.paths_cut.saturating_add(1);
        self.edits.push((
            path.start,
            keyword_start.saturating_add(keyword.len()),
            replacement,
        ));
        Ok(())
    }

    /// The `gs` that gives a fill of a surviving outline the stroke's own alpha and overprint, or
    /// nothing where the fill's already are.
    ///
    /// A fill takes the non-stroking members of two pairs Table 51 keeps apart — §11.6.4.4's two
    /// alpha constants, one for strokes and one for everything else, and the two overprint
    /// parameters "one for stroking and one for all other painting operations" —
    /// and no operator sets either, so where a stroke's differ from the fill's they are restated
    /// by a graphics state parameter dictionary holding the stroke's values under the fill's keys,
    /// `/ca` and `/op`. The dictionary states the producer's own numbers and nothing else, so
    /// the outline composites exactly as the stroke did; the stream's resources gain it under a
    /// name no entry already has (ADR 1351).
    fn restate_stroke(&mut self) -> Vec<u8> {
        let mut state = Dictionary::new();
        if (self.graphics.stroking_alpha - self.graphics.fill_alpha).abs() > 0.0 {
            state.insert(
                Name::new(&b"ca"[..]),
                Object::Real(self.graphics.stroking_alpha),
            );
        }
        if self.graphics.stroke_overprint != self.graphics.fill_overprint {
            state.insert(
                Name::new(&b"op"[..]),
                Object::Boolean(self.graphics.stroke_overprint),
            );
        }
        if state.is_empty() {
            return Vec::new();
        }
        state.insert(
            Name::new(&b"Type"[..]),
            Object::Name(Name::new(&b"ExtGState"[..])),
        );
        let name = if let Some((name, _)) = self
            .added_states
            .iter()
            .find(|(_, earlier)| *earlier == state)
        {
            name.clone()
        } else {
            let taken = self.document.get_key(&self.resources, "ExtGState");
            let taken = taken.as_dict();
            let mut ordinal = self.added_states.len().saturating_add(1);
            loop {
                let candidate = format!("RedactStroke{ordinal}").into_bytes();
                let used = taken.is_some_and(|dict| {
                    dict.get_by_name(&Name::new(candidate.as_slice())).is_some()
                }) || self.added_states.iter().any(|(name, _)| *name == candidate);
                if !used {
                    break candidate;
                }
                ordinal = ordinal.saturating_add(1);
            }
        };
        if !self
            .added_states
            .iter()
            .any(|(earlier, _)| *earlier == name)
        {
            self.added_states.push((name.clone(), state));
        }
        let mut out = b"/".to_vec();
        out.extend_from_slice(&name);
        out.extend_from_slice(b" gs\n");
        out
    }

    /// A zero-width stroke cut as the path it is drawn along, and restroked at the same width
    /// (ADR 1363).
    ///
    /// §8.4.3.2 states the width in the device's terms — "\[a] line width of 0 shall denote the
    /// thinnest line that can be rendered at device resolution: 1 device pixel wide" — so the
    /// marks are the path at whatever resolution draws it, and no outline in user space states
    /// them. The path is therefore cut as a line ([`paths::subtract_along`]): the stretches inside
    /// the region go, the rest are stroked again by the same `S` under the same graphics state,
    /// whose width is still 0. A dash pattern is applied first, as it is for a wide stroke, since
    /// it decides which stretches are marked at all; the dashes are written out as subpaths of
    /// their own and stroked solid, inside a §8.4.2-balanced `q`/`Q`, because a pattern restarted
    /// at each surviving piece would mark different stretches.
    fn cut_hairline(
        &self,
        path: &PathObject,
        closes: bool,
        regions: &[[f64; 4]],
        to_display: paths::Mapping,
    ) -> Result<Vec<u8>, String> {
        let source = path.stroked_path(closes);
        let subpaths = if self.graphics.dashes.is_empty() {
            split_open(&source)
        } else {
            let dashed: kurbo::BezPath = kurbo::dash(
                source.elements().iter().copied(),
                self.graphics.dash_phase,
                &self.graphics.dashes,
            )
            .collect();
            split_open(&dashed)
        };
        let cut = paths::subtract_along(&subpaths, regions, to_display).ok_or_else(|| {
            "§12.5.6.23: cutting a zero-width stroke against these regions exceeds this build's \
             bound on the surviving pieces; the page is refused rather than truncated"
                .to_owned()
        })?;
        if !cut.margin_holds() {
            return Err(
                "§7.3.3: a coordinate of the cut path is large enough that writing it back and \
                 reading it as a single-precision real could move the cut end inside the region; \
                 the page is refused rather than leave a sliver of the removed marks"
                    .to_owned(),
            );
        }
        if cut.polygons.is_empty() {
            return Ok(Vec::new());
        }
        let mut text = String::new();
        let dashed = !self.graphics.dashes.is_empty();
        if dashed {
            text.push_str("q\n[] 0 d\n");
        }
        paths::write_open(&mut text, &cut.polygons);
        text.push_str("S\n");
        if dashed {
            text.push_str("Q\n");
        }
        Ok(text.into_bytes())
    }

    /// The outline §8.5.3.2's stroke marks, as the subpaths a fill of it would paint.
    ///
    /// > The S operator shall paint a line along the current path.
    ///
    /// That line is the region a pen of the current width sweeps along the path, closed off by
    /// §8.4.3.3's caps and turned by §8.4.3.4's joins, with §8.4.3.6's dash pattern deciding
    /// which stretches are marked at all. `kurbo::stroke` computes it, dashes included, and what
    /// comes back is geometry the cut takes exactly as it takes a fill's.
    ///
    /// Offsetting a straight segment and closing it with a butt or projecting-square cap and a
    /// miter or bevel join is computed in closed form. A round cap, a round join and the offset of
    /// a curved segment are arcs no path can state, so they come back as cubics within
    /// [`ARC_TOLERANCE`] of the arc and are cut at their roots like any §8.5.2.2 curve: the
    /// outline outside the region is the producer's mark re-expressed to that bound, and the
    /// one inside it is gone. ADR 1324.
    fn stroke_outline(&self, path: &PathObject, closes: bool) -> Vec<paths::SubPath> {
        let source = path.stroked_path(closes);
        // `admits_a_cut` has refused a singular transform, so the norm is positive.
        let tolerance = ARC_TOLERANCE / mapping(path.ctm).norm();
        let outline = kurbo::stroke(
            source.elements().iter().copied(),
            &self.graphics.stroke_style(),
            &kurbo::StrokeOpts::default(),
            tolerance,
        );
        split_subpaths(&outline)
    }
}
