//! What a display-list command does to the page, read without drawing it: where it marks, whether it
//! blends, whether it lets what is under it show through. The group analyses ask these of a whole
//! list (ADR 2080).

use pdf_render::{BlendMode, Command, Paint, Path, PathCommand, Point, Rect, Transform};

/// A glyph outline's bounding box in page space, for §9.3.8's overlap test.
///
/// Built from the control points rather than from the curves' extremes, so it contains the
/// outline rather than hugging it. Both approximations run the same way — the box is a
/// superset of the ink — which is what the caller needs.
pub(in crate::content) fn outline_bounds(outline: &Path, transform: Transform) -> Option<Rect> {
    let mut bounds: Option<Rect> = None;
    let mut add = |point: Point| {
        let mapped = transform.apply(point);
        bounds = Some(match bounds {
            Some(rect) => rect.union(Rect::from_corners(mapped, mapped)),
            None => Rect::from_corners(mapped, mapped),
        });
    };
    for command in outline.commands() {
        match *command {
            PathCommand::MoveTo(point) | PathCommand::LineTo(point) => add(point),
            PathCommand::CurveTo(first, second, end) => {
                add(first);
                add(second);
                add(end);
            }
            PathCommand::Close => {}
        }
    }
    bounds
}

/// Whether any command in a group, at any depth, satisfies `wanted`.
///
/// Recursive because a group's elements may themselves be groups: §11.4.3 calls both an
/// *element*, and a question about what a group contains is a question about its tree.
pub(in crate::content) fn any_command(
    commands: &[Command],
    wanted: &dyn Fn(&Command) -> bool,
) -> bool {
    commands.iter().any(|command| {
        wanted(command)
            || match command {
                Command::Group { commands, .. } => any_command(commands, wanted),
                _ => false,
            }
    })
}

/// Whether a command asks to be blended with what is under it, rather than painted over it.
///
/// A [`Command::Shaped`] blends as its object does: the shape half is drawn Normal by
/// construction ([`super::shape::stated_shape`]), and the object is what §11.3.3 composites. **A `Shaped`
/// does not fall to the arm below**, because there a knockout group drawn on transparency with
/// a stated element would be reported as "non-isolated, and an element blends with the
/// backdrop it excludes" whenever Table 145 says `/I false` — a report about a blend mode that
/// nothing carries (ADR 1009).
pub(in crate::content) fn command_blends(command: &Command) -> bool {
    match command {
        Command::Fill { blend, .. }
        | Command::Stroke { blend, .. }
        | Command::Image { blend, .. }
        | Command::Group { blend, .. } => *blend != BlendMode::Normal,
        Command::Shaped { object, .. } => command_blends(object),
        // `Command` is non-exhaustive. A command whose blending is unknown counts as
        // blending, because both callers decide whether to *report*: an unnecessary report
        // is recoverable, a missed one is a page drawn wrong in silence.
        _ => true,
    }
}

/// Whether a command's result can let what is under it show through.
///
/// The union of blending and of any transparency at all — a constant alpha below one, a
/// colour or shading carrying alpha, an image with a soft mask or a stencil, and a soft mask
/// in the graphics state. A group is transparent if its own alpha is, or if anything inside
/// it is: a group of half-opaque objects produces a half-opaque result.
///
/// Scanning an image's samples is linear in them, which is why the one caller asks this only
/// for a knockout group.
pub(in crate::content) fn command_composites(command: &Command) -> bool {
    if command_blends(command) {
        return true;
    }
    // A soft mask is §11.6.4.1's third source of opacity, so an object painted through one
    // composites however opaque its colour is. `knockout_smask.pdf` is why this line is
    // here: its knockout group paints an opaque blue over an opaque red *under a mask*, and
    // without this the §11.4.6 report saw two opaque fills and stayed quiet about a page
    // three references draw the other way.
    if command.mask().is_some() {
        return true;
    }
    match command {
        Command::Fill { paint, .. } | Command::Stroke { paint, .. } => match paint {
            Paint::Solid(colour) => colour.a < 1.0,
            Paint::Shading(shading) => !shading.is_opaque(),
            // `Paint` is non-exhaustive, and a paint whose opacity is unknown is treated as
            // compositing: this decides whether to *report*, and an unnecessary report is
            // recoverable where a missed one is a page drawn wrong in silence.
            _ => true,
        },
        Command::Image { image, alpha, .. } => *alpha < 1.0 || !image.is_opaque(),
        Command::Group {
            commands, alpha, ..
        } => *alpha < 1.0 || any_command(commands, &command_composites),
        _ => true,
    }
}

/// Where a command marks the page, as a box containing its ink.
///
/// A superset rather than a tight fit — control points rather than curve extremes, and a
/// stroke widened by its whole line width rather than by half of it — because the one caller
/// is an overlap test for a *report*, where saying two elements might overlap when they do
/// not is recoverable and the reverse is a missed gap.
pub(super) fn command_bounds(command: &Command) -> Option<Rect> {
    match command {
        Command::Fill {
            path, transform, ..
        } => outline_bounds(path, *transform),
        Command::Stroke {
            path,
            transform,
            stroke,
            ..
        } => {
            let bounds = outline_bounds(path, *transform)?;
            // The width is in the path's space, so it reaches the page scaled by the
            // transform — by the *largest* factor the transform stretches a length, since the
            // margin has to hold in every direction. This used to be the determinant's square
            // root, described as "an over-estimate for a sheared one", which is the wrong way
            // round: a shear can leave the determinant at 1 while tripling a length, so the
            // margin was too small and an overlap could be missed. `Transform::max_stretch`
            // is the bound the comment claimed.
            let margin = stroke.width * transform.max_stretch();
            Some(Rect::from_corners(
                Point::new(bounds.min.x - margin, bounds.min.y - margin),
                Point::new(bounds.max.x + margin, bounds.max.y + margin),
            ))
        }
        Command::Image { transform, .. } => {
            // An image occupies the unit square, which the command's transform places.
            let mut square = Path::new();
            square.push(PathCommand::MoveTo(Point::new(0.0, 0.0)));
            square.push(PathCommand::LineTo(Point::new(1.0, 0.0)));
            square.push(PathCommand::LineTo(Point::new(1.0, 1.0)));
            square.push(PathCommand::LineTo(Point::new(0.0, 1.0)));
            outline_bounds(&square, *transform)
        }
        Command::Group { commands, .. } => commands
            .iter()
            .filter_map(command_bounds)
            .reduce(Rect::union),
        // Unbounded, which the overlap test reads as covering everything.
        _ => None,
    }
}
