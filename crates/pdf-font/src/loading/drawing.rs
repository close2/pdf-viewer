//! A glyph's outline: the cache both routes share, the extraction and normalisation, the
//! hinting instance a hint-reliant program needs, and the questions asked of the program's `glyf`
//! and outline tables.

use super::{Face, LoadedFont, PathPen, Placed, outlines};
use crate::cff;
use crate::cmap::Code;
use crate::program::Program;
use crate::substituted::wound_counter_clockwise;
use pdf_render::Path;
use skrifa::outline::{
    DrawSettings, Engine, HintingInstance, HintingOptions, OutlineGlyphCollection, Target,
};
use skrifa::prelude::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use std::sync::Arc;

impl LoadedFont {
    /// Returns the outline for a character code, with one em as one unit.
    ///
    /// That is the space PDF's text matrix expects, so the caller multiplies by the font
    /// size and nothing else.
    ///
    /// Returns `None` when the code has no glyph, which includes the ordinary case of a
    /// space in a font with no space outline.
    ///
    /// # §9.6.5.2's `.notdef`, and why it is applied here rather than in the code table
    ///
    /// > If an encoding maps to a character name that does not exist in the Type 1 font program,
    /// > the .notdef glyph shall be substituted.
    ///
    /// The condition is exactly what it says: the *encoding named a glyph* and the program does
    /// not have it. A code the encoding says nothing about is not this sentence's subject and
    /// still reaches nothing.
    ///
    /// It is applied at the drawing step and deliberately not written into the code table,
    /// because the table is what [`Self::glyph_index`] answers with and three of this project's
    /// instruments read that answer: `codes_without_a_glyph` counts the codes a page showed that
    /// reached none (ADR 0152), `simple_code_table` refuses a font whose every code resolved to
    /// nothing, and the whitespace check tells a blank glyph from an absent one. Substituting in
    /// the table would tell all three that every unresolved code had been drawn — a `shall`
    /// obeyed by blinding the gates that watch it.
    #[must_use]
    pub fn outline(&self, code: Code) -> Option<Arc<Path>> {
        let placed = match self.placed_glyph(code) {
            Some(placed) => placed,
            None => Placed {
                face: Face::Own,
                glyph: self.notdef.filter(|_| self.substitutes_notdef(code))?,
            },
        };
        self.cached_outline(placed)
    }

    /// One glyph's outline, through the cache both routes into this font share.
    ///
    /// Separated from [`Self::outline`] because [`Self::character_glyph`] arrives at a glyph
    /// without a code and must not build a second cache to do it: a face drawing an interface's
    /// own text reuses the same few dozen glyphs exactly as a page does.
    pub(super) fn cached_outline(&self, placed: Placed) -> Option<Arc<Path>> {
        if let Some(cached) = outlines(&self.outlines).get(&placed) {
            return cached.clone();
        }
        let built = self.build_outline(placed);
        outlines(&self.outlines).insert(placed, built.clone());
        built
    }

    /// Extracts and normalises one glyph outline.
    fn build_outline(&self, placed: Placed) -> Option<Arc<Path>> {
        let Placed { face, glyph } = placed;
        // §9.8.3.3's class face is a second face with its own em square, and it is always an
        // `sfnt`: `substitute::installed_covering` is the only thing that chose it and this
        // machine's catalogue admits no other container.
        let class = match face {
            Face::Own => None,
            Face::Class(index) => Some(self.classes.get(index)?.as_ref()?),
        };
        let data = class.map_or(&self.data, |class| &class.data);
        let units_per_em = class.map_or(self.units_per_em, |class| class.units_per_em);
        let program = if class.is_some() {
            Program::Sfnt
        } else {
            self.program
        };
        let mut pen = PathPen {
            path: Path::new(),
            scale: 1.0 / units_per_em,
            stretch: self.stretch,
            last: None,
        };

        match program {
            Program::BareCff => cff::draw(data, glyph, &mut pen).ok()?,
            Program::Type1 => self.type1.as_ref()?.draw(glyph, &mut pen).ok()?,
            Program::Sfnt => {
                let font = FontRef::new(data).ok()?;
                let glyphs = font.outline_glyphs();
                let outline = glyphs.get(GlyphId::from(glyph))?;
                // Unhinted and unscaled by default: hinting is a device-resolution decision,
                // and this outline is resolution-independent because the text matrix scales
                // it later. The exception is the hint-reliant family — see [`Self::hinting`]
                // — whose instructions *construct* the contours rather than grid-fit them;
                // those draw through the interpreter, and a bytecode failure falls back to
                // the unhinted skeleton, which is the picture every glyph of the family drew
                // before ADR 0727 rather than no glyph at all.
                let unhinted = || DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
                // A class face draws unhinted: [`Self::hinting`] is memoised against the *own*
                // face's em square and glyph set, and handing another face's outline to it would
                // grid-fit with the wrong instance. No class face is in the hint-reliant family
                // anyway — that family is a document's own embedded program (ADR 0727).
                match class.is_none().then(|| self.hinting(&glyphs)).flatten() {
                    Some(instance) => {
                        if outline
                            .draw(DrawSettings::hinted(instance, false), &mut pen)
                            .is_err()
                        {
                            pen.path = Path::new();
                            pen.last = None;
                            outline.draw(unhinted(), &mut pen).ok()?;
                        }
                    }
                    None => {
                        outline.draw(unhinted(), &mut pen).ok()?;
                    }
                }
            }
        }

        // A contour at one point states no edge, and a glyph's scan conversion is the font's own
        // (ADR 2330): see [`without_point_contours`]. A glyph left with no contour is the blank
        // glyph this method already answers `None` for.
        let path = without_point_contours(pen.path);
        if path.is_empty() {
            return None;
        }
        // §9.3.6 NOTE 2 makes a glyph's contour direction visible where two glyphs overlap
        // inside one clipping path, and a face this program chose has no direction the
        // document stated — so every substituted outline is wound the same way, and an
        // embedded program's is left exactly as its producer drew it.
        if self.substituted {
            return Some(Arc::new(wound_counter_clockwise(path)));
        }
        Some(Arc::new(path))
    }

    /// The interpreter instance for a hint-reliant font program, built on first use.
    ///
    /// `None` for every font the library does not place in that family, and for a family
    /// member whose `fpgm`/`prep` programs fail — both of which draw unhinted, exactly as
    /// every font did before ADR 0727.
    ///
    /// The size is one pixel per design unit, and that is the decision this method exists to
    /// hold: an instruction that rounds to the pixel grid then rounds to the grid the outline
    /// was designed on — a near no-op — while the constructive moves this family's programs
    /// exist for are carried out in full. So the outline stays resolution-independent, which
    /// is what [`Self::outlines`]' cache and every caller rely on, and the grid-fitting half
    /// of hinting stays declined, which is this tree's standing scan-conversion position
    /// (`doc/todo/_scan-conversion.md`). [`Target::Mono`] and [`Engine::Interpreter`] are the
    /// combination [`OutlineGlyphCollection::require_interpreter`]'s own documentation
    /// prescribes for this family.
    fn hinting(&self, glyphs: &OutlineGlyphCollection) -> Option<&HintingInstance> {
        self.hinting
            .get_or_init(|| {
                if !glyphs.require_interpreter() {
                    return None;
                }
                HintingInstance::new(
                    glyphs,
                    Size::new(self.units_per_em),
                    LocationRef::default(),
                    HintingOptions {
                        engine: Engine::Interpreter,
                        target: Target::Mono,
                    },
                )
                .ok()
            })
            .as_ref()
    }

    /// The glyph at which the `glyf` composite `code` reaches includes itself, where it does.
    ///
    /// Only the producer's own `TrueType` program is asked: a substitute's glyphs are this
    /// machine's, and a name-keyed or CFF program has no composite glyphs of this kind.
    /// [`crate::sfnt::composite_cycle`] holds the reading (ADR 1411).
    #[must_use]
    pub fn composite_cycle(&self, code: Code) -> Option<u16> {
        if self.substituted || self.program != Program::Sfnt {
            return None;
        }
        let placed = self.placed_glyph(code)?;
        if placed.face != Face::Own {
            return None;
        }
        crate::sfnt::composite_cycle(&self.data, placed.glyph)
    }

    /// Whether the producer's own program describes no outline for any glyph it holds.
    ///
    /// The case §9.7.6.3 settles by construction: a code whose CID the program lacks is given "the
    /// glyph for CID 0 (which shall be present)", and a program whose every glyph — CID 0
    /// included — is empty therefore draws nothing for any code by the clause's own route. That
    /// is the font the file states, not a mark this reader lost, and an OCR layer's invisible font
    /// is its ordinary shape. A program with even one outline is answered `false`, so a subset
    /// short of the glyphs its page shows is still what a page reports (ADR 1411).
    ///
    /// Walks glyph indices in order and stops at the first outline, so it costs one glyph for
    /// any ordinary font; the whole walk is bounded by the program's own glyph count, 65 535 at
    /// most, and happens only where a code already reached no outline. `false` for a substitute,
    /// which is this machine's face, and for a Type 1 program, whose glyphs are read by name.
    #[must_use]
    pub fn holds_no_outline(&self) -> bool {
        if self.substituted {
            return false;
        }
        // A glyph that could not be read is not one the program states empty, so an error
        // answers `false` as an outline does: a damaged program is not silenced by this.
        let states_empty = |glyph: u16| {
            let mut pen = PathPen {
                path: Path::new(),
                scale: 1.0,
                stretch: 1.0,
                last: None,
            };
            let drawn = match self.program {
                Program::Sfnt => FontRef::new(&self.data).ok().is_some_and(|font| {
                    font.outline_glyphs()
                        .get(GlyphId::from(glyph))
                        .is_some_and(|outline| {
                            outline
                                .draw(
                                    DrawSettings::unhinted(
                                        Size::unscaled(),
                                        LocationRef::default(),
                                    ),
                                    &mut pen,
                                )
                                .is_ok()
                        })
                }),
                Program::BareCff => cff::draw(&self.data, glyph, &mut pen).is_ok(),
                Program::Type1 => false,
            };
            drawn && pen.path.is_empty()
        };
        let count = match self.program {
            Program::Sfnt => FontRef::new(&self.data)
                .ok()
                .and_then(|font| font.maxp().ok())
                .map(|maxp| u32::from(maxp.num_glyphs())),
            Program::BareCff => cff::glyph_count(&self.data).ok(),
            Program::Type1 => None,
        };
        let Some(count) = count.and_then(|count| u16::try_from(count).ok()) else {
            return false;
        };
        count > 0 && (0..count).all(states_empty)
    }
}

/// A glyph outline less its contours at one point — ISO 32000-2 §10.7.4's last paragraph.
///
/// > Scan conversion of character glyphs may be performed by a different algorithm from the
/// > preceding one.
///
/// A font program's contour whose every point, control points included, is one point — a
/// TrueType contour of a single point, or a move and a closepath with nothing drawn between them,
/// or a curve of no length — states no edge, and a font's own rasteriser fills by edges, so it
/// marks nothing. §8.5.3.3.1's degenerate subpath, which paints the pixel under its point (ADR
/// 2330), is a rule about a path a content stream constructs, and the permission above keeps it off
/// a glyph's contours: dozens of corpus fonts carry such a contour inside ordinary letters, where
/// painting it would put a speck beside the glyph. A contour with an extent keeps §10.7.4's marks.
fn without_point_contours(path: Path) -> Path {
    use pdf_render::PathCommand;
    let commands = path.commands();
    let mut kept = Path::new();
    let mut start = 0usize;
    let mut dropped = false;
    while start < commands.len() {
        let mut end = start;
        let mut first: Option<pdf_render::Point> = None;
        let mut one_point = true;
        let mut opened = false;
        while end < commands.len() {
            let points: &[pdf_render::Point] = match &commands[end] {
                PathCommand::MoveTo(p) => {
                    if opened {
                        break;
                    }
                    std::slice::from_ref(p)
                }
                PathCommand::LineTo(p) => std::slice::from_ref(p),
                PathCommand::CurveTo(a, b, c) => &[*a, *b, *c],
                PathCommand::Close => {
                    end = end.saturating_add(1);
                    break;
                }
            };
            opened = true;
            for point in points {
                match first {
                    None => first = Some(*point),
                    Some(at) => one_point &= at == *point,
                }
            }
            end = end.saturating_add(1);
        }
        let contour = commands.get(start..end).unwrap_or_default();
        // A run of `h` alone names no point and is kept as the program stated it.
        if first.is_some() && one_point {
            dropped = true;
        } else {
            kept.extend(contour);
        }
        start = end.max(start.saturating_add(1));
    }
    if dropped { kept } else { path }
}

#[cfg(test)]
mod tests {
    use super::without_point_contours;
    use pdf_render::{Path, PathCommand, Point};

    fn path(commands: &[PathCommand]) -> Path {
        let mut path = Path::new();
        for command in commands {
            path.push(*command);
        }
        path
    }

    fn square() -> [PathCommand; 5] {
        [
            PathCommand::MoveTo(Point::new(0.1, 0.1)),
            PathCommand::LineTo(Point::new(0.5, 0.1)),
            PathCommand::LineTo(Point::new(0.5, 0.5)),
            PathCommand::LineTo(Point::new(0.1, 0.5)),
            PathCommand::Close,
        ]
    }

    /// The shape corpus fonts carry inside ordinary letters: a move and a closepath after the
    /// last contour. It goes, and the letter's own contour stays exactly as drawn.
    #[test]
    fn a_contour_at_one_point_is_dropped_and_the_letter_kept() {
        let mut commands = square().to_vec();
        commands.push(PathCommand::MoveTo(Point::new(0.27, 0.53)));
        commands.push(PathCommand::Close);
        assert_eq!(without_point_contours(path(&commands)).commands(), square());
    }

    /// A curve of no length is a contour at one point too, and a glyph that is nothing else is
    /// left with no contour — the blank glyph `build_outline` answers `None` for.
    #[test]
    fn a_curve_of_no_length_is_a_point_and_a_glyph_of_only_that_is_blank() {
        let origin = Point::new(0.0, 0.0);
        let dot = path(&[
            PathCommand::MoveTo(origin),
            PathCommand::CurveTo(origin, origin, origin),
            PathCommand::Close,
        ]);
        assert!(without_point_contours(dot).is_empty());
    }

    /// A contour with an extent is not this rule's, even one with no area: it keeps §10.7.4's
    /// mark. And an outline with no contour at one point is returned as it was.
    #[test]
    fn a_contour_with_an_extent_is_kept() {
        let mut commands = square().to_vec();
        commands.push(PathCommand::MoveTo(Point::new(0.2, 0.7)));
        commands.push(PathCommand::LineTo(Point::new(0.6, 0.9)));
        commands.push(PathCommand::Close);
        assert_eq!(
            without_point_contours(path(&commands)).commands(),
            commands.as_slice()
        );
    }
}
