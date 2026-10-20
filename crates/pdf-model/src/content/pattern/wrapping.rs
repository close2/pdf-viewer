//! §11.7.5.2 and §11.6.6: a finished tiling put into a group, and the transfer function the painting
//! mark states recorded on its tiles (ADR 2081).

use std::sync::Arc;

use pdf_render::{BlendMode, Command, SoftMaskId};

use crate::content::transparency::{
    AlphaSourcesSeen, any_command, command_blends, command_composites, group_alpha_is_shape,
};
use crate::content::{GraphicsState, Interpreter, KnockoutKind};

/// Every leaf of `commands` pushed into §11.7.5.2's channel under one function.
///
/// The recursion is what a group costs: §11.7.5.2's elementary objects are the leaves, and a
/// [`Command::Group`] is the compositing around them rather than an object the clause can choose a
/// function from. See [`Interpreter::record_tiling`], the one caller.
pub(super) fn record(
    commands: &[Command],
    map: Option<&Arc<pdf_render::TransferMap>>,
    into: &mut pdf_render::TransferBuilder,
) {
    for command in commands {
        match command {
            Command::Group { commands, .. } => record(commands, map, into),
            Command::Shaped { object, .. } => record(std::slice::from_ref(object), map, into),
            leaf => into.push(map.map(Arc::clone), leaf),
        }
    }
}

impl Interpreter<'_> {
    /// Records a finished tiling's marks in §11.7.5.2's channel, as one run.
    ///
    /// ISO 32000-2 §11.7.5.2 chooses the function by "the last (topmost) elementary graphics
    /// object enclosing that point", and its sixth condition makes the object painted with a
    /// tiling pattern that object: the cell's marks decide only whether it is *fully opaque*.
    /// So every tile carries the function in force at the painting operation, and the shapes the
    /// run holds are the tiles themselves — a pixel between two cells is enclosed by no object
    /// here and takes whatever is under it.
    ///
    /// `transfer` is `None` where the clause withholds the function, which is where the mark is
    /// not fully opaque or where some object of the cell is not; the tiles are then recorded as
    /// *occluders*, without which a transferred mark underneath would choose the function at
    /// pixels this tiling covers.
    ///
    /// Nothing walks here on a page that has stated no transfer, which is all but one of the
    /// corpus (`examples/transfer_function_census`). ADR 1266.
    pub(super) fn record_tiling(
        &mut self,
        mark: usize,
        transfer: Option<Arc<crate::content::Transfer>>,
    ) {
        let map = transfer.map(|transfer| self.transfer_map(&transfer));
        if map.is_none() && !self.transfers.is_live() {
            return;
        }
        let tiles = self
            .list
            .commands()
            .get(mark..)
            .unwrap_or_default()
            .to_vec();
        record(&tiles, map.as_ref(), &mut self.transfers);
    }

    /// Wraps a finished tiling in the groups its region and its graphics state ask for.
    ///
    /// `mark` is where the tiles begin, `alpha` is whichever of §11.6.4.4's two constants the
    /// invoking operator is under, `shape` is [`super::Tiled::Stroke`]'s region where there is one, and
    /// `reading` is which of §11.6.4.3's readings the cell's own marks were painted under
    /// ([`Interpreter::run_cell`]). Split out of [`Interpreter::tile`] because it is the only
    /// part of that function that is about compositing rather than about placing cells.
    ///
    /// # The cell's readings and the enclosing record
    ///
    /// §11.6.7 makes what the cell evaluates to "the object's source colour ( 𝐶𝑠 ), object
    /// shape ( f j ), and object opacity ( qi )", so the cell's reading decides the object's
    /// shape and the painting mark's decides only its own constant and mask. Where the tiling
    /// reaches the enclosing content as one group whose raster is its own shape
    /// (`alpha_is_shape`), that object's shape is stated whatever reading the enclosing content
    /// asks it under (ADR 1306); tiles that would stand inline under a reading other than the
    /// mark's are given that group where §11.6.7's NOTE 1 makes an isolated one exact: "in the
    /// common case in which the pattern consists entirely of objects painted with the Normal
    /// blend mode … the pattern cell can be evaluated once and then replicated".
    ///
    /// Everywhere else the tiles are read by whatever the enclosing content asks of it, under
    /// the reading the mark is painted under — so where the cell ran under the other one, each
    /// tile states its shape under the cell's reading first (`Interpreter::seal_elements`), and
    /// the finished tiling is then described by the mark's reading alone (ADR 1319).
    pub(super) fn compose_tiling(
        &mut self,
        mark: usize,
        (alpha, shape): (f32, Option<SoftMaskId>),
        state: &GraphicsState,
        reading: AlphaSourcesSeen,
    ) {
        let painted = AlphaSourcesSeen::of(state.alpha_is_shape);
        let mut reading = reading;
        let now = self.list.command_count();
        if reading != painted && now > mark {
            let tiles = self.list.commands().get(mark..).unwrap_or_default();
            // The two routes `wrap_tiling` takes to one group whose raster is its shape: a
            // group it always builds — a stroke's region, or the state's own parameters — and
            // the isolated one NOTE 1 makes exact for inline tiles.
            let composites =
                alpha < 1.0 || state.blend != BlendMode::Normal || state.soft_mask.is_some();
            let grouped = shape.is_some()
                || composites
                || self.enclosing_knockout == Some(KnockoutKind::Isolated)
                || !any_command(tiles, &command_blends);
            if !(grouped && group_alpha_is_shape(tiles, reading.settled_over(tiles))) {
                if self.seal_elements(mark, now, reading) {
                    reading = painted;
                } else {
                    self.poison_readings();
                }
            }
        }
        self.wrap_tiling(mark, (alpha, shape), state, reading);
    }

    /// [`Interpreter::compose_tiling`]'s groups.
    fn wrap_tiling(
        &mut self,
        mark: usize,
        (alpha, shape): (f32, Option<SoftMaskId>),
        state: &GraphicsState,
        reading: AlphaSourcesSeen,
    ) {
        // A stroke's region, applied once to the finished tiling. §11.6.4.2 makes an object's
        // shape "1.0 inside and 0.0 outside" the mark it makes, and §11.5.2 derives a mask
        // from "the alpha of the group" — so a group holding the stroke alone, taken for its
        // alpha, *is* that shape, and multiplying the tiles by it is §11.3.7.1's `α = f × q`
        // with the tiling supplying `q`.
        //
        // It is a group of its own rather than the one below because the two carry different
        // quantities and a command has one mask slot: this one is the object's shape, the one
        // below is the state's own opacity source (§11.6.4.1's second and third sources). Where
        // the state sets none of its parameters this is the only group, which is why it is not
        // conditioned on `composites`.
        if let Some(shape) = shape {
            let parts = self.list.split_off_commands(mark);
            if parts.is_empty() {
                return;
            }
            let alpha_is_shape = group_alpha_is_shape(&parts, reading.settled_over(&parts));
            self.draw(Command::Group {
                commands: parts,
                // The state's constant rides the group below; this one only shapes.
                alpha: 1.0,
                clip: None,
                mask: Some(shape),
                blend: BlendMode::Normal,
                // §11.4.6's NOTE 6 again: a mask multiplies what the group produced, so the
                // group has to produce the tiling alone rather than the tiling over whatever
                // is under it. That is what isolation means, and it is the construction
                // rather than a choice.
                isolated: true,
                knockout: false,
                alpha_is_shape,
                blending: None,
            });
        }

        // The state's transparency parameters, applied once to the finished tiling. Where
        // they are all at their defaults there is nothing for a group to do and §11.4.4's
        // NOTE 5 says so in as many words — "the effect of compositing objects as a group is
        // the same as that of compositing them separately (without grouping)" — so the
        // commands stay inline and no page pays a buffer for a pattern that composites
        // trivially, which is almost every patterned page in the corpus.
        let composites =
            alpha < 1.0 || state.blend != BlendMode::Normal || state.soft_mask.is_some();
        if !composites {
            if shape.is_none() {
                self.wrap_inline_tiles(mark, reading, state);
            }
            return;
        }
        let parts = self.list.split_off_commands(mark);
        if parts.is_empty() {
            return;
        }
        // §11.6.7 makes the implicit group *non-isolated*, and the display list says so
        // wherever the clause's own NOTE 1 does not make an isolated group exact: "in the common
        // case in which the pattern consists entirely of objects painted with the Normal blend
        // mode … the results depend only on the colour, shape, and opacity of the pattern cell
        // and not on those of the backdrop". A cell no element of which blends is that case and
        // is isolated; a cell where one does gets the backdrop the clause names, which
        // `Command::Group`'s `isolated` states (ADR 0237) and `render-cpu` draws under any blend
        // mode at the `Do` (ADR 1107, ADR 1243).
        //
        // §11.4.6's NOTE 6 decides the one exception, and it is a *backdrop* rather than a mode:
        // §11.6.7 makes the cell an element of whatever paints it, so a cell painted as a direct
        // element of a knockout group takes that group's initial backdrop. Where that backdrop is
        // transparent the isolated construction **is** the clause — §11.4.5's group by that
        // clause's own definition, whatever this one names — and where it is not, the
        // non-isolated group the clause names is what the enclosing group hands a private clone
        // of its initial backdrop to (ADR 1256), so stating it truthfully is what asks for the
        // right backdrop rather than what departs from it. ADR 1265.
        let isolated = self.enclosing_knockout == Some(KnockoutKind::Isolated)
            || !any_command(&parts, &command_blends);
        // Asked of the cell's own marks under the `/AIS` reading the cell ran under, the way
        // every other group is asked — see `group_alpha_is_shape`. Stated truthfully rather
        // than as `false` so that the field means one thing everywhere it is written.
        let alpha_is_shape = group_alpha_is_shape(&parts, reading.settled_over(&parts));
        self.draw(Command::Group {
            commands: parts,
            alpha,
            // The tiles carry the path's clip already; a second copy on the group would be
            // the same region resolved twice.
            clip: None,
            mask: state.soft_mask,
            blend: state.blend,
            isolated,
            knockout: false,
            alpha_is_shape,
            // §8.7.3.1: a pattern cell's colours are resolved in the compositing already in
            // force, so the implicit group introduces no space of its own.
            blending: None,
        });
    }

    /// Gives tiles §11.6.7's implicit group where standing inline would not be the same
    /// picture: under a reading other than the painting mark's, or as marks that composite with
    /// one another where the tiling is to be *one* element of a knockout group.
    ///
    /// §11.6.7 makes the pattern "implicitly enclosed in a non-isolated transparency group: a
    /// non-knockout group for tiling patterns", and §11.4.4's NOTE 5 makes that group the tiles
    /// drawn inline only where its parent is not a knockout group — it asks that the group
    /// have "the same knockout attribute as its parent group". Inside §9.3.8's text object or
    /// §11.7.4.4's pair each tile would otherwise be an element of its own and knock the cell's
    /// other marks out, where the clause composites them with one another (ADR 1320). That
    /// group is the clause's own and is stated non-isolated where a tile blends, isolated where
    /// §11.6.7's NOTE 1 makes that exact.
    ///
    /// For the reading alone the group is built only where it is exact as an isolated one —
    /// nothing in the cell blends, or the enclosing knockout group's initial backdrop is
    /// transparent — and a cell that blends stays inline for `compose_tiling` to seal.
    fn wrap_inline_tiles(&mut self, mark: usize, reading: AlphaSourcesSeen, state: &GraphicsState) {
        let tiles = self.list.commands().get(mark..).unwrap_or_default();
        let isolated = self.enclosing_knockout == Some(KnockoutKind::Isolated)
            || !any_command(tiles, &command_blends);
        let as_one = (self.readings.knockout_elements() || self.enclosing_knockout.is_some())
            && tiles.len() > 1
            && any_command(tiles, &command_composites);
        let for_reading = reading != AlphaSourcesSeen::of(state.alpha_is_shape) && isolated;
        if tiles.is_empty() || !(as_one || for_reading) {
            return;
        }
        let parts = self.list.split_off_commands(mark);
        let alpha_is_shape = group_alpha_is_shape(&parts, reading.settled_over(&parts));
        self.draw(Command::Group {
            commands: parts,
            alpha: 1.0,
            clip: None,
            mask: None,
            blend: BlendMode::Normal,
            isolated,
            knockout: false,
            alpha_is_shape,
            blending: None,
        });
    }
}
