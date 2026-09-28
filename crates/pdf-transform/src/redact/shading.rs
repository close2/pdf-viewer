//! The `sh` operator under a redaction region, ISO 32000-2 §8.7.4.2 and §12.5.6.23.
//!
//! Table 76's `sh` paints "the shape and colour shading described by a shading dictionary,
//! subject to the current clipping path", so where its marks fall is the clip and nothing in the
//! content stream names them one by one: there is no code to delete and no path to cut. What
//! bounds the marks is the clip, and the clip is what is cut — the painted area becomes the
//! current clip intersected with the region's complement, built by the same nine-cell construction
//! [`super::paths`] applies to a painted path, so the edge the marks stop at is the one whose
//! rounding [`super::paths::Cut::margin_holds`] proves cannot reach back into the region.
//!
//! **That is a clip hiding marks, and the clause forbids it for image data**: "clipping or image
//! masks shall not be used to hide that data". So the construction is taken only where the
//! shading holds no data of its own located in the region. An axial or radial shading (§8.7.4.5.3,
//! §8.7.4.5.4) states two points or two circles and a function of one parameter; where that
//! function is §7.10.3's exponential, or §7.10.4's stitching of exponentials, the dictionary
//! states a colour at each end of each piece and nothing about where on the page any of it lands
//! — the region's marks are the gradient law evaluated there, and nothing is left once they are
//! not painted. A function-based shading (§8.7.4.5.2) is a function of two coordinates, a mesh
//! (§8.7.4.5.5 to §8.7.4.5.8) is vertices and colours placed in the plane, and a sampled or
//! calculator function (§7.10.2, §7.10.5) is a table or a program that may hold values only the
//! region's parameters reach: each is data located in the region, which cutting the clip would
//! hide rather than destroy, so a page where one meets the region is refused by name. ADR 1351.

use pdf_render::Transform;
use pdf_render::geom::Point;
use pdf_syntax::object::{Name, Object};

use super::{Operand, Walk, cut_to_complement, mapping, overlaps, paths};

/// How deep a §7.10.4 stitching function's `/Functions` are followed before the shading is
/// refused rather than read further.
const MAX_STITCHING_DEPTH: usize = 8;

impl Walk<'_> {
    /// Narrows the clip bound by a path that §8.5.4's `W` or `W*` made a clipping boundary.
    ///
    /// The path's box contains the path, so the intersection contains the clip it sets. A
    /// clipping path with no points names no area, and nothing is painted through it.
    pub(super) fn narrow_clip(&mut self, path: &super::PathObject) {
        if !path.clips {
            return;
        }
        let bounds = path.bbox.unwrap_or([0.0, 0.0, 0.0, 0.0]);
        self.graphics.clip = Some(match self.graphics.clip {
            Some(clip) => [
                clip[0].max(bounds[0]),
                clip[1].max(bounds[1]),
                clip[2].min(bounds[2]).max(clip[0].max(bounds[0])),
                clip[3].min(bounds[3]).max(clip[1].max(bounds[1])),
            ],
            None => bounds,
        });
    }

    /// `sh` (§8.7.4.2): left alone where its marks cannot reach the region, painted through the
    /// clip intersected with the region's complement where they can and the shading holds no
    /// data of its own there, and refused by name otherwise.
    pub(super) fn shade(
        &mut self,
        content: &[u8],
        operands: &[(Operand, usize)],
        keyword_start: usize,
    ) -> Result<(), String> {
        let Some((name, start)) = operands.iter().find_map(|(operand, start)| match operand {
            Operand::Name(bytes) => Some((bytes.clone(), *start)),
            _ => None,
        }) else {
            return Ok(());
        };
        let shown = String::from_utf8_lossy(&name).into_owned();
        let shadings = self.document.get_key(&self.resources, "Shading");
        let shading = shadings
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name.as_slice())))
            .map(|entry| self.document.resolve(entry));
        let Some(dict) = shading.as_ref().and_then(Object::as_dict) else {
            return Err(format!(
                "§8.7.4.2: the content paints /{shown}, which /Resources /Shading does not \
                 define; the page is refused"
            ));
        };
        let extent = self.shading_extent(dict);
        if let Some(extent) = extent
            && !self.regions.iter().any(|region| overlaps(*region, extent))
        {
            return Ok(());
        }
        if let Some(located) = self.located_data(dict) {
            return Err(format!(
                "the shading /{shown} meets the region, and {located}, which cutting the clip \
                 would hide rather than destroy; the page is refused"
            ));
        }
        if self.ctm.determinant() == 0.0 {
            return Err(
                "§8.3.4: the transform in force where sh meets the region is singular; the page \
                 is refused"
                    .to_owned(),
            );
        }
        let window = self.painted_window(extent)?;
        let regions = self.region_bounds();
        let cut = cut_to_complement(&[window], &regions, mapping(self.ctm))?;
        let end = keyword_start.saturating_add(2);
        let own = content.get(start..end).unwrap_or_default();
        // Balanced inside the replacement (§8.4.2), so the clip every later mark is held to is
        // the producer's. A window the regions take whole leaves nothing to paint, and the
        // operator goes with its marks.
        let mut bytes = Vec::new();
        if !cut.is_empty() {
            let mut replacement = String::from("q\n");
            paths::write_polygons(&mut replacement, &cut);
            replacement.push_str("W n\n");
            bytes.extend_from_slice(replacement.as_bytes());
            bytes.extend_from_slice(own);
            bytes.extend_from_slice(b"\nQ\n");
        }
        // Counted with the painted paths: the report's count is of marks removed geometrically,
        // and a shading's are cut at the same edge a path's are.
        self.paths_cut = self.paths_cut.saturating_add(1);
        self.edits.push((start, end, bytes));
        Ok(())
    }

    /// A box in the display list's space enclosing what `sh` can paint: the clip, narrowed by
    /// Table 77's `/BBox` — "a temporary clipping boundary when the shading is painted" — where
    /// the shading states one. `None` where neither bounds it.
    fn shading_extent(&self, dict: &pdf_syntax::object::Dictionary) -> Option<[f32; 4]> {
        let mut extent = self.graphics.clip;
        if let Some([x0, y0, x1, y1]) = super::numbers(self.document, dict, "BBox")
            .as_deref()
            .and_then(|values| <[f32; 4]>::try_from(values).ok())
        {
            let corners = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
                .map(|(x, y)| self.ctm.apply(Point::new(x, y)));
            let flat: Vec<f32> = corners
                .iter()
                .flat_map(|point| [point.x, point.y])
                .collect();
            let bbox = super::bbox_of_points(&flat);
            extent = Some(extent.map_or(bbox, |extent| {
                [
                    extent[0].max(bbox[0]),
                    extent[1].max(bbox[1]),
                    extent[2].min(bbox[2]),
                    extent[3].min(bbox[3]),
                ]
            }));
        }
        extent
    }

    /// The area `sh` paints as a polygon in the content stream's user space: the extent, widened
    /// by one unit and by the regions so the cut has a whole window to take them out of, mapped
    /// back through the inverse of the transform in force. Where nothing clips the page, the
    /// page's media box stands for the clip, being the most any mark on it can be seen through.
    fn painted_window(&self, extent: Option<[f32; 4]>) -> Result<paths::SubPath, String> {
        let page = {
            let media = self.page.media_box;
            let base = pdf_model::content::base_transform(self.page);
            let corners = [
                (media[0], media[1]),
                (media[2], media[1]),
                (media[2], media[3]),
                (media[0], media[3]),
            ]
            .map(|(x, y)| base.apply(Point::new(x, y)));
            let flat: Vec<f32> = corners
                .iter()
                .flat_map(|point| [point.x, point.y])
                .collect();
            super::bbox_of_points(&flat)
        };
        let mut window = extent.unwrap_or(page);
        for region in &self.regions {
            window = [
                window[0].min(region[0]),
                window[1].min(region[1]),
                window[2].max(region[2]),
                window[3].max(region[3]),
            ];
        }
        let window = [
            window[0] - 1.0,
            window[1] - 1.0,
            window[2] + 1.0,
            window[3] + 1.0,
        ];
        let inverse: Transform = self.ctm.invert().ok_or_else(|| {
            "§8.3.4: the transform in force where sh meets the region has no inverse; the page \
             is refused"
                .to_owned()
        })?;
        let corners = [
            (window[0], window[1]),
            (window[2], window[1]),
            (window[2], window[3]),
            (window[0], window[3]),
        ]
        .map(|(x, y)| inverse.apply(Point::new(x, y)));
        let mut polygon = paths::SubPath::new();
        for (index, corner) in corners.iter().enumerate() {
            let point = kurbo::Point::new(f64::from(corner.x), f64::from(corner.y));
            if index == 0 {
                polygon.move_to(point);
            } else {
                polygon.line_to(point);
            }
        }
        polygon.close_path();
        Ok(polygon)
    }

    /// Why a shading's colours are data placed in the plane, or `None` where they are a gradient
    /// law that holds nothing of its own located in the region.
    pub(super) fn located_data(&self, dict: &pdf_syntax::object::Dictionary) -> Option<String> {
        let kind = self.document.get_key(dict, "ShadingType").as_integer();
        if !matches!(kind, Some(2 | 3)) {
            return Some(format!(
                "§8.7.4.3's ShadingType {} places its colours in the plane — a function of two \
                 coordinates or a mesh of vertices",
                kind.map_or_else(|| "absent".to_owned(), |kind| kind.to_string())
            ));
        }
        if !self.analytic(&self.document.get_key(dict, "Function"), 0) {
            return Some(
                "its §7.10 function is sampled or calculated, and may hold values only the \
                 region's parameters reach"
                    .to_owned(),
            );
        }
        None
    }

    /// A painted path whose colour is a §8.7.4 shading pattern meets the region: refused where
    /// the pattern's shading holds data placed in the plane, for the reason `sh` is (ADR 1351).
    ///
    /// The cut removes the path's marks in the region, and the pattern goes on being named by the
    /// marks that survive, so its dictionary is carried whole: a gradient law is carried with
    /// nothing of the region's in it, a mesh or a sampled function with the region's colours in
    /// it. A §8.7.3 tiling pattern's cell repeats across the whole plane and is the same data wherever
    /// it is drawn, so nothing of it is the region's own.
    pub(super) fn pattern_admits_a_cut(&self, name: &[u8]) -> Result<(), String> {
        let shown = String::from_utf8_lossy(name);
        let patterns = self.document.get_key(&self.resources, "Pattern");
        let pattern = patterns
            .as_dict()
            .and_then(|dict| dict.get_by_name(&Name::new(name)))
            .map(|entry| self.document.resolve(entry));
        let Some(pattern) = pattern.as_ref().and_then(Object::as_dict) else {
            return Err(format!(
                "§8.7.2: a path painted in the pattern /{shown} meets the region, and \
                 /Resources /Pattern does not define it; the page is refused"
            ));
        };
        if self.document.get_key(pattern, "PatternType").as_integer() != Some(2) {
            return Ok(());
        }
        let shading = self.document.get_key(pattern, "Shading");
        let located = shading.as_dict().map_or_else(
            || Some("it names no shading dictionary".to_owned()),
            |dict| self.located_data(dict),
        );
        match located {
            Some(located) => Err(format!(
                "§8.7.4.1: a path painted in the shading pattern /{shown} meets the region, and \
                 {located}, which the pattern would carry past the cut; the page is refused"
            )),
            None => Ok(()),
        }
    }

    /// Whether a shading's `/Function` is analytic: §7.10.3's exponential, or §7.10.4's stitching
    /// of functions that are, given one per component or as one function.
    fn analytic(&self, function: &Object, depth: usize) -> bool {
        if depth > MAX_STITCHING_DEPTH {
            return false;
        }
        let function = self.document.resolve(function);
        if let Some(items) = function.as_array() {
            return !items.is_empty()
                && items
                    .iter()
                    .all(|item| self.analytic(item, depth.saturating_add(1)));
        }
        let Some(dict) = function.as_dict() else {
            return false;
        };
        match self.document.get_key(dict, "FunctionType").as_integer() {
            Some(2) => true,
            Some(3) => self
                .document
                .get_key(dict, "Functions")
                .as_array()
                .is_some_and(|items| {
                    !items.is_empty()
                        && items
                            .iter()
                            .all(|item| self.analytic(item, depth.saturating_add(1)))
                }),
            _ => false,
        }
    }
}
