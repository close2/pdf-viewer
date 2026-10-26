//! Operands: how many an operator takes, which of the pending ones it is given, and the readers
//! that turn them into numbers, points, matrices, strings and names.

use pdf_render::{Point, Transform};
use pdf_syntax::{Name, Object};

use crate::content::reader::Operator;

/// The operands one operator is given, out of everything the stream has stated since the
/// last one.
///
/// ISO 32000-2 §7.8.2 states the rule this implements:
///
/// > In PDF, all of the operands needed by an operator shall immediately precede that
/// > operator. Operators do not return results, and operands shall not be left over when an
/// > operator finishes execution.
///
/// So an operator's operands are the *last* `n` of what precedes it, not the first. On a
/// conforming stream nothing is ever left over and the two readings are the same slice; the
/// sentence decides only what a malformed one means, and there the difference is total —
/// reading from the front shifts every operand by however many the file left behind, which
/// silently mis-draws a mark or, where the shifted operand has the wrong type, drops it with
/// nothing reported.
///
/// [`Operator::operand_count`] is [`None`] for the operators whose own table states no fixed
/// number, and for a keyword Table 50 does not list, and
/// those are given everything: `TJ` and `d` take an array the content lexer flattens into one
/// operand per element, and `sc`, `scn`, `SC` and `SCN` take as many components as the current
/// colour space has (§8.6.8, Table 73). A leftover operand in front of one of those is
/// indistinguishable from one of its own, so nothing here can improve on reading them whole.
pub(super) fn operands_before(pending: &[Object], count: Option<usize>) -> &[Object] {
    let Some(count) = count else {
        return pending;
    };
    let start = pending.len().saturating_sub(count);
    pending.get(start..).unwrap_or(pending)
}

impl Operator {
    /// How many operands the operator takes, where its own table states a fixed number.
    ///
    /// Each count is the operand list in the table Annex A's summary points at for that operator
    /// — Table 56 and 57 for the graphics state, 58 and 59 for paths, 60 for clipping, 73 for
    /// colour, 74 for shading, 87 for `XObject`s, 103, 105, 106 and 107 for text, 111 for a Type 3
    /// glyph's metrics, 33 for compatibility and 352 for marked content.
    ///
    /// An operator this interpreter does not implement reaches here too, and gets [`None`]: it is
    /// reported rather than run, so how many operands it wanted is not a question anything asks.
    pub(in crate::content) const fn operand_count(self) -> Option<usize> {
        Some(match self {
            Self::Save
            | Self::Restore
            | Self::ClosePath
            | Self::EndPath
            | Self::Fill
            | Self::FillObsolete
            | Self::FillEvenOdd
            | Self::Stroke
            | Self::CloseStroke
            | Self::FillStroke
            | Self::FillStrokeEvenOdd
            | Self::CloseFillStroke
            | Self::CloseFillStrokeEvenOdd
            | Self::Clip
            | Self::ClipEvenOdd
            | Self::BeginText
            | Self::EndText
            | Self::NextLine
            | Self::EndMarked
            | Self::BeginCompatibility
            | Self::EndCompatibility
            | Self::BeginImage => 0,
            Self::LineWidth
            | Self::LineCap
            | Self::LineJoin
            | Self::MiterLimit
            | Self::Flatness
            | Self::RenderingIntent
            | Self::ExtGState
            | Self::CharSpacing
            | Self::WordSpacing
            | Self::HorizontalScaling
            | Self::Leading
            | Self::Rise
            | Self::RenderMode
            | Self::ShowText
            | Self::NextLineShowText
            | Self::FillColourSpace
            | Self::StrokeColourSpace
            | Self::FillGray
            | Self::StrokeGray
            | Self::Shade
            | Self::XObject
            | Self::MarkPoint
            | Self::BeginMarked => 1,
            Self::MoveTo
            | Self::LineTo
            | Self::MoveText
            | Self::MoveTextSetLeading
            | Self::Font
            | Self::GlyphWidth
            | Self::MarkPointProperties
            | Self::BeginMarkedProperties => 2,
            Self::FillRgb | Self::StrokeRgb | Self::NextLineShowTextSpaced => 3,
            Self::CurveInitialReplicated
            | Self::CurveFinalReplicated
            | Self::Rectangle
            | Self::FillCmyk
            | Self::StrokeCmyk => 4,
            Self::Concatenate | Self::CurveTo | Self::TextMatrix | Self::GlyphWidthAndBox => 6,
            Self::Dash
            | Self::ShowTextAdjusted
            | Self::FillColour
            | Self::FillColourN
            | Self::StrokeColour
            | Self::StrokeColourN
            | Self::ImageData
            | Self::EndImage => return None,
        })
    }

    /// Whether this is one of Table 59's path-painting operators, which end a path object.
    pub(in crate::content) const fn paints(self) -> bool {
        matches!(
            self,
            Self::Stroke
                | Self::CloseStroke
                | Self::Fill
                | Self::FillObsolete
                | Self::FillEvenOdd
                | Self::FillStroke
                | Self::FillStrokeEvenOdd
                | Self::CloseFillStroke
                | Self::CloseFillStrokeEvenOdd
                | Self::EndPath
        )
    }

    /// The operators ISO 32000-2 §8.6.8 says an uncoloured figure's content stream may not use.
    ///
    /// The clause's own list, in full: the twelve colour operators of Table 73, plus `ri` and
    /// `sh`. The last two are worth noticing rather than copying — `ri` sets a rendering intent,
    /// which is colour-related without being a colour, and `sh` paints a *shading*, which carries
    /// its own colours and so cannot belong to a figure whose colour comes from outside it.
    ///
    /// `gs` is not here, because an `/ExtGState` sets much more than colour — including the line
    /// width and dash pattern §9.6.4 tells a glyph description to set explicitly. The clause
    /// lists the entries *within* it that are ignored, and `apply_ext_gstate` drops those.
    pub(in crate::content) const fn is_colour(self) -> bool {
        matches!(
            self,
            Self::StrokeColourSpace
                | Self::FillColourSpace
                | Self::StrokeColour
                | Self::StrokeColourN
                | Self::FillColour
                | Self::FillColourN
                | Self::StrokeGray
                | Self::FillGray
                | Self::StrokeRgb
                | Self::FillRgb
                | Self::StrokeCmyk
                | Self::FillCmyk
                | Self::RenderingIntent
                | Self::Shade
        )
    }
}

/// Reads operand `index` as an integer code.
///
/// Accepts a real that happens to be integral, since producers write `1.0` where `1` is
/// meant.
pub(super) fn integer_at(operands: &[Object], index: usize) -> Option<i64> {
    let value = operands.get(index)?;
    value.as_integer().or_else(|| {
        let number = value.as_number()?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "guarded to an integral value below 1000 by the condition"
        )]
        let code = number as i64;
        (number.is_finite() && number.fract() == 0.0 && number.abs() < 1000.0).then_some(code)
    })
}

/// Reads operand `index` as a number.
pub(in crate::content) fn number_at(operands: &[Object], index: usize) -> Option<f32> {
    let value = operands.get(index)?.as_number()?;
    if !value.is_finite() {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "content-stream coordinates are page-scale; a value outside f32's range \
                  cannot describe a position on a page"
    )]
    Some(value as f32)
}

/// Reads the first `N` operands as numbers, requiring all of them.
///
/// **`N` is a constant rather than an argument, and the array is the whole reason.** Annex A
/// gives every operator that reaches here a fixed operand count — `Operator::operand_count` is
/// that table — so the answer's size is known where it is asked for, and returning it on the
/// stack costs no allocation at all. The `Vec` this used to return did: with a `filter_map`'s
/// lower size hint of zero, `collect` began at capacity nought and grew, so six numbers were a
/// `malloc` and two `realloc`s. On the witness of `doc/todo/44` — one page, 3.19 M operators, most of
/// them `c` — that was **12.00% of the whole interpretation** in the collect and its
/// reallocation, and `points_from`'s second `Vec` another 2.83% (ADR 0370's table).
///
/// What it costs in readability is one type parameter at six call sites; what it buys is
/// measured in ADR 0370.
pub(super) fn numbers_from<const N: usize>(operands: &[Object]) -> Option<[f32; N]> {
    let mut values = [0.0_f32; N];
    for (index, slot) in values.iter_mut().enumerate() {
        *slot = number_at(operands, index)?;
    }
    Some(values)
}

/// Reads `N` coordinate pairs, requiring all of them.
pub(super) fn points_from<const N: usize>(operands: &[Object]) -> Option<[Point; N]> {
    let mut points = [Point::new(0.0, 0.0); N];
    // The pairs are read directly rather than through `numbers_from`, because a const
    // parameter cannot be doubled in a type without `generic_const_exprs`. Two `number_at`s
    // per point is what the old two-step did anyway, minus the intermediate array.
    let mut index = 0_usize;
    for slot in &mut points {
        let x = number_at(operands, index)?;
        let y = number_at(operands, index.saturating_add(1))?;
        *slot = Point::new(x, y);
        index = index.saturating_add(2);
    }
    Some(points)
}

/// Reads six operands as a matrix.
pub(super) fn matrix_from(operands: &[Object]) -> Option<Transform> {
    let values = numbers_from::<6>(operands)?;
    Some(Transform::new(
        values[0], values[1], values[2], values[3], values[4], values[5],
    ))
}

/// Reads operand `index` as a string.
pub(super) fn string_at(operands: &[Object], index: usize) -> Option<Vec<u8>> {
    operands.get(index)?.as_string().map(<[u8]>::to_vec)
}

/// Narrows a PDF number to `f32`.
pub(in crate::content) fn narrow(value: f64) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a text adjustment outside f32's range is not a position on a page"
    )]
    {
        value as f32
    }
}

/// Reads operand `index` as a name, keeping the bytes §7.3.5 says a name is.
///
/// > Uniquely defined means that any two name objects that, after all escaping is expanded (see
/// > below), and the resulting sequences of bytes are not an exact binary match denote different
/// > objects.
///
/// So the operand travels to `resources.rs` as a [`Name`]: a `String` built with
/// `from_utf8_lossy` on the way turns every byte outside UTF-8 into U+FFFD and makes the
/// resource it names unfindable (ADR 0438). A *tag* is compared against one of the standard's
/// own ASCII names and may be read as bytes here too, which is what `== b"OC"` is.
pub(in crate::content) fn name_at(operands: &[Object], index: usize) -> Option<Name> {
    operands.get(index)?.as_name().cloned()
}

#[cfg(test)]
mod tests {
    use super::Operator;

    /// Every operator Table 50 lists is recognised from its own spelling and from no other,
    /// so classifying a keyword once cannot hand the dispatch a different arm than its bytes
    /// would have reached.
    #[test]
    fn each_operator_is_recognised_from_its_spelling() {
        for operator in Operator::ALL {
            assert_eq!(
                Operator::of(operator.name()),
                Some(operator),
                "{operator:?}"
            );
        }
        let mut names: Vec<&[u8]> = Operator::ALL
            .iter()
            .map(|operator| operator.name())
            .collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            Operator::ALL.len(),
            "two operators share a spelling"
        );
    }

    /// Table 50's misprint "Sh" is not Table 76's `sh`, and §7.3.2's `true`, which a content
    /// stream lexes as a keyword, is no operator.
    #[test]
    fn keywords_table_50_does_not_list_are_not_operators() {
        for word in [&b"Sh"[..], b"true", b"null", b"qq", b"", b"BDCX", b"T"] {
            assert_eq!(
                Operator::of(word),
                None,
                "{}",
                String::from_utf8_lossy(word)
            );
        }
    }

    /// The fixed operand counts, read off the operator tables Annex A points at: one per
    /// table's own operand list, and none for the four colour setters whose count is the
    /// colour space's, `d` and `TJ`, whose array the content lexer flattens, and Table 90's two
    /// that `BI` reads.
    #[test]
    fn operand_counts_are_the_tables_own() {
        let counted = |count: Option<usize>| -> Vec<&[u8]> {
            Operator::ALL
                .iter()
                .filter(|operator| operator.operand_count() == count)
                .map(|operator| operator.name())
                .collect()
        };
        assert_eq!(
            counted(None),
            [&b"d"[..], b"TJ", b"SC", b"SCN", b"sc", b"scn", b"ID", b"EI"]
        );
        assert_eq!(counted(Some(6)), [&b"cm"[..], b"c", b"Tm", b"d1"]);
        assert_eq!(counted(Some(4)), [&b"v"[..], b"y", b"re", b"K", b"k"]);
        assert_eq!(counted(Some(3)), [&b"\""[..], b"RG", b"rg"]);
        assert_eq!(
            counted(Some(2)),
            [&b"m"[..], b"l", b"Tf", b"Td", b"TD", b"d0", b"DP", b"BDC"]
        );
    }
}
