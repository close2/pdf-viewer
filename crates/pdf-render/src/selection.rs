//! A list's top-level commands as a reader draws them: every one, or the ones a window keeps.
//!
//! A frame that shows part of a page is drawn from the commands that reach it
//! ([`DisplayList::reaching`], ADR 1841), named by their indices into the page's own list rather
//! than copied into a list of their own. On the owner's witness a window of 83 % of the page keeps
//! 4 333 950 of its 4 408 405 commands: copying them is a reference count apiece, 43 to 49 ms of the
//! frame, and letting the copy go another 18 to 21 (ADR 1941); drawn through a [`Selection`], that
//! window lands 118 to 270 ms sooner on four threads and 100 to 130 on twenty-four, and ties the
//! page it is most of (ADR 1980).
//!
//! **A reader of a selection reads what the copy held, in its order.** Each command keeps its
//! index into the whole list, which is what [`MarkBounds`](crate::MarkBounds) is indexed by, and
//! its position in the selection, which is what the copy's own index was; the clips, soft masks
//! and page boundary are the whole list's, which the copy carried unchanged. So a reader asking
//! either, of either, has the copy's answer, and the pixels are the copy's.

use std::ops::Range;

use crate::{Command, DisplayList};

/// The top-level commands of [`Self::list`] a reader draws, in painting order.
///
/// `Copy`, because it is two references: a reader hands it on rather than borrowing it.
#[derive(Clone, Copy, Debug)]
pub struct Selection<'a> {
    list: &'a DisplayList,
    kept: Kept<'a>,
}

/// Which commands a [`Selection`] names.
#[derive(Clone, Copy, Debug)]
enum Kept<'a> {
    /// A run of the list's commands, by index from `start` to before `end` — the whole list, or a
    /// part of it.
    Run {
        /// The first command's index.
        start: usize,
        /// The index past the last command.
        end: usize,
    },
    /// The commands at these indices, ascending.
    Indices(&'a [u32]),
}

impl<'a> Selection<'a> {
    /// Every command of `list`.
    #[must_use]
    pub fn whole(list: &'a DisplayList) -> Self {
        Self {
            list,
            kept: Kept::Run {
                start: 0,
                end: list.commands().len(),
            },
        }
    }

    /// `window`'s commands where there is one, and every command of `list` where there is not:
    /// what a backend draws a target from once [`DisplayList::reaching`] has answered.
    #[must_use]
    pub fn drawn(list: &'a DisplayList, window: Option<&'a Window<'_>>) -> Self {
        window.map_or_else(|| Self::whole(list), Window::selection)
    }

    /// The commands of `list` at `indices`, ascending — what [`Window::selection`] hands a
    /// reader.
    fn of(list: &'a DisplayList, indices: &'a [u32]) -> Self {
        Self {
            list,
            kept: Kept::Indices(indices),
        }
    }

    /// The list the commands are of, whose clips, soft masks and page every one of them names.
    #[must_use]
    pub fn list(self) -> &'a DisplayList {
        self.list
    }

    /// How many commands are selected.
    #[must_use]
    pub fn len(self) -> usize {
        match self.kept {
            Kept::Run { start, end } => end.saturating_sub(start),
            Kept::Indices(indices) => indices.len(),
        }
    }

    /// Whether no command is selected.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Whether every command of the list is selected, in which case a reader may read the list
    /// itself.
    #[must_use]
    pub fn is_whole(self) -> bool {
        matches!(self.kept, Kept::Run { start: 0, end } if end == self.list.commands().len())
    }

    /// The selected commands at `positions` — the positions in this selection, not indices into
    /// the list — or as many of them as there are.
    #[must_use]
    pub fn part(self, positions: Range<usize>) -> Self {
        let length = self.len();
        let (from, to) = (positions.start.min(length), positions.end.min(length));
        let kept = match self.kept {
            Kept::Run { start, .. } => Kept::Run {
                start: start.saturating_add(from),
                end: start.saturating_add(to.max(from)),
            },
            Kept::Indices(indices) => {
                Kept::Indices(indices.get(from..to.max(from)).unwrap_or_default())
            }
        };
        Self {
            list: self.list,
            kept,
        }
    }

    /// The indices into the list these commands occupy, where they are a run of it: every command
    /// of a whole list or of a part of one, and `None` for a window's.
    #[must_use]
    pub fn run(self) -> Option<Range<usize>> {
        match self.kept {
            Kept::Run { start, end } => Some(start..end),
            Kept::Indices(_) => None,
        }
    }

    /// Each selected command with its index into the list, in painting order.
    #[must_use]
    pub fn indexed(self) -> Indexed<'a> {
        let commands = self.list.commands();
        match self.kept {
            Kept::Run { start, end } => Indexed::Run {
                first: start,
                commands: commands
                    .get(start..end)
                    .unwrap_or_default()
                    .iter()
                    .enumerate(),
            },
            Kept::Indices(indices) => Indexed::Indices {
                commands,
                indices: indices.iter(),
            },
        }
    }

    /// Each selected command, in painting order.
    pub fn commands(self) -> impl Iterator<Item = &'a Command> + Clone + 'a {
        self.indexed().map(|(_, command)| command)
    }
}

/// [`Selection::indexed`]'s iterator: each command with its index into the list.
///
/// A type of its own rather than a chain of the two cases, because the page's own strip loop
/// reads every command of a whole list through it once per strip, and a run is then the slice's
/// own iterator with nothing asked of it per command but the index it already counts.
#[derive(Clone, Debug)]
pub enum Indexed<'a> {
    /// A run of the list, `first` the index of its first command.
    Run {
        /// The index of the run's first command.
        first: usize,
        /// The run's commands, counted from 0.
        commands: std::iter::Enumerate<std::slice::Iter<'a, Command>>,
    },
    /// The commands at `indices`.
    Indices {
        /// Every command of the list.
        commands: &'a [Command],
        /// The indices not yet read, ascending.
        indices: std::slice::Iter<'a, u32>,
    },
}

impl<'a> Iterator for Indexed<'a> {
    type Item = (usize, &'a Command);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Run { first, commands } => commands
                .next()
                .map(|(offset, command)| (first.saturating_add(offset), command)),
            // An index a window kept is one of the list's by construction; one that is not would
            // be passed over rather than read past the list's end.
            Self::Indices { commands, indices } => indices.find_map(|index| {
                let index = *index as usize;
                commands.get(index).map(|command| (index, command))
            }),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Run { commands, .. } => commands.size_hint(),
            Self::Indices { indices, .. } => (0, Some(indices.len())),
        }
    }
}

/// The commands of a list that reach a target, by their indices into it: what
/// [`DisplayList::reaching`] answers with, and a reader draws through [`Self::selection`].
#[derive(Debug)]
pub struct Window<'a> {
    list: &'a DisplayList,
    /// The kept commands' indices, ascending.
    ///
    /// Thirty-two bits an index, for the reason the planner's band keys are (ADR 1940): the
    /// owner's witness keeps 4 333 950 of them at a window of 83 % of the page, 17 MB at four
    /// bytes and 35 MB at eight. A list of more commands than they count is drawn whole rather
    /// than culled ([`DisplayList::reaching`]).
    kept: Vec<u32>,
}

impl<'a> Window<'a> {
    /// The kept commands of `list` at `kept`, ascending.
    pub(crate) fn new(list: &'a DisplayList, kept: Vec<u32>) -> Self {
        Self { list, kept }
    }

    /// The kept commands, for a reader to draw.
    #[must_use]
    pub fn selection(&self) -> Selection<'_> {
        Selection::of(self.list, &self.kept)
    }

    /// How many commands the window keeps.
    #[must_use]
    pub fn len(&self) -> usize {
        self.kept.len()
    }

    /// Whether the window keeps no command at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kept.is_empty()
    }

    /// The kept commands as a list of their own — the copy a selection is drawn without, for an
    /// instrument or a test that compares a drawing of the two or hands the planner a list.
    #[must_use]
    pub fn copied(&self) -> DisplayList {
        self.list
            .with_commands(self.selection().commands().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, FillRule, Paint, Path, PathCommand, Point, Size, Transform};
    use std::sync::Arc;

    fn list_of(count: u16) -> DisplayList {
        let mut list = DisplayList::new(Size::new(100.0, 100.0));
        for at in 0..count {
            let mut path = Path::new();
            path.push(PathCommand::MoveTo(Point::new(f32::from(at), 0.0)));
            path.push(PathCommand::LineTo(Point::new(f32::from(at) + 1.0, 1.0)));
            list.push(Command::Fill {
                path: Arc::new(path),
                transform: Transform::IDENTITY,
                fill_rule: FillRule::NonZero,
                paint: Paint::Solid(Color::BLACK),
                clip: None,
                mask: None,
                blend: crate::BlendMode::Normal,
            });
        }
        list
    }

    #[test]
    fn a_selection_names_its_commands_by_index_and_by_position() {
        let list = list_of(6);
        let window = Window::new(&list, vec![1, 2, 4]);
        let selected = window.selection();
        assert_eq!(selected.len(), 3);
        assert!(!selected.is_whole());
        let indices: Vec<usize> = selected.indexed().map(|(index, _)| index).collect();
        assert_eq!(indices, [1, 2, 4]);
        let part: Vec<usize> = selected
            .part(1..9)
            .indexed()
            .map(|(index, _)| index)
            .collect();
        assert_eq!(
            part,
            [2, 4],
            "a part is of positions, and stops where the selection does"
        );
        assert_eq!(selected.run(), None);
        assert_eq!(
            window.copied().commands(),
            [
                list.commands()[1].clone(),
                list.commands()[2].clone(),
                list.commands()[4].clone()
            ]
        );
        let whole = Selection::whole(&list);
        assert!(whole.is_whole());
        assert_eq!(whole.part(2..4).run(), Some(2..4));
        assert!(!whole.part(2..4).is_whole());
        assert!(whole.commands().eq(list.commands()));
    }
}
