//! The adapter that puts the tree on AT-SPI, and the two platforms that have none.
//!
//! # Nothing eager, and where the runtime is
//!
//! `accesskit_unix` needs an executor, because `zbus` is asynchronous. It puts one on **a thread
//! of its own**, created the first time an adapter is: the thread connects to the session bus,
//! watches `org.a11y.Status.IsEnabled`, and leaves every adapter *inactive* — publishing nothing,
//! asking this program for nothing — until an assistive technology is actually there.
//!
//! What this crate adds to that is the one rule `CLAUDE.md`'s startup section makes
//! non-negotiable: **the adapter is not created on the launch path at all**. [`Bridge::new`] is
//! called after the first frame has been presented, so a D-Bus connection is never in front of
//! page one. ADR 0214 has the measurement.
//!
//! # What a client may ask for, and who carries it out
//!
//! Until the five-hundred-and-ninetieth session the tree declared no [`accesskit::Action`] on any
//! node, so a conforming client requested none and anything that arrived anyway was printed by
//! name. Three are declared now — [`accesskit::Action::ScrollIntoView`] on an element that has a
//! place, [`accesskit::Action::Click`] on one whose content is an annotation, and
//! [`accesskit::Action::SetTextSelection`] on the page — and each resolves to an [`Act`], which is
//! a *place* in the viewport's device pixels and nothing else.
//!
//! **The division is the boundary's own.** This crate says where; the host sends the
//! `Command::Scroll` or the `Command::Pointer`, because rule 5 keeps `viewer-core` free of any
//! platform and rule 2 keeps this crate free of a viewer. No message was added to say any of it:
//! a click is a point, a caret is a point, and a scroll is a rectangle a host already knows how to
//! bring into a viewport it owns.
//!
//! **A request arrives on the adapter's thread**, so [`Bridge::new`] takes something to wake the
//! host's loop with. A window resting in `ControlFlow::Wait` would otherwise carry the request out
//! at the next unrelated event, which for a person using only a screen reader is never.
//!
//! # What the other two platforms say
//!
//! macOS and Windows have AccessKit adapters and this program does not use them, because nothing
//! here can test one. That is a shortfall rather than an absence, and it is *named* —
//! [`Bridge::shortfall`] answers with a sentence a host prints, exactly as
//! `pdf_sandbox::Confinement::shortfall` does for the two platforms with no kernel confinement
//! (ADR 0194). A build that quietly did nothing would be the failure that precedent exists to
//! prevent.

use std::sync::{Arc, Mutex, mpsc};

use accesskit::{Action, ActionData, Node, NodeId, Rect, TextDirection, TextPosition, TreeUpdate};

/// What carrying out one request means, in the vocabulary `viewer-core` already has.
///
/// **Three geometric answers rather than three messages.** Each of the actions this crate declares
/// is a question about a *place* on the page — where to scroll to, where to click, where to put a
/// caret — and `Command::Scroll` and `Command::Pointer` already take places in exactly these
/// device pixels. So the boundary needed nothing added to it, which is the test
/// `doc/ui-boundary.md` puts on a new message and the reason there is not one here.
///
/// Everything is in **device pixels of the viewport**, the space
/// [`viewer_core::AccessibilityNode::quads`] and [`viewer_core::Query::Selection`] are in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Act {
    /// Bring a rectangle of the page into view — AT-SPI's `Component.ScrollTo`.
    ///
    /// `[x0, y0, x1, y1]`. How far to move is the host's: it knows the viewport, and a rectangle
    /// already inside it needs no scroll at all.
    Show {
        /// The rectangle to make visible.
        at: [f32; 4],
    },
    /// Click a point of the page — AT-SPI's `Action.DoAction`, whose one action is named `click`.
    ///
    /// The centre of the node's own place, so that what is clicked is what the client's user was
    /// told is there. §12.5.1's activation, §12.6.3's triggers and §12.7.5.2's toggling all follow
    /// from a click at a point, which is why this carries a point rather than an object.
    Click {
        /// Where to press and release.
        at: (f32, f32),
    },
    /// Put the caret, or draw a selection, over the page's own text — AT-SPI's
    /// `Text.SetCaretOffset` and `Text.SetSelection`.
    ///
    /// Two points because the platform sends two: a caret is the degenerate selection whose anchor
    /// and focus are equal, which AccessKit states outright.
    Caret {
        /// The anchor, "which does not change as the selection is expanded or contracted".
        from: (f32, f32),
        /// The active end, or the caret.
        to: (f32, f32),
    },
}

/// What an assistive technology asked this program to do.
///
/// **This used to be a name to print and nothing else**, because the tree declared no actions at
/// all and a conforming client therefore requested none. Since the five-hundred-and-ninetieth
/// session three are declared and [`Self::means`] says what each one is in this program's terms;
/// an action that is *not* declared can still arrive — `Component.ScrollTo` and
/// `Text.SetCaretOffset` are offered by the adapter on the strength of the node's bounds and its
/// text runs rather than of a declaration — and one this crate cannot place answers `None` and is
/// still printed by name, which is the half of trap 5 that has not changed.
#[derive(Debug, Clone, PartialEq)]
pub struct Requested {
    /// Which node it was asked of.
    pub node: NodeId,
    /// What was asked.
    pub action: Action,
    /// What carrying it out means here, where the published tree can say.
    pub means: Option<Act>,
}

/// The bridge between this program's tree and the platform's.
///
/// Holds the current tree so that the platform can ask for it from another thread —
/// `accesskit::ActivationHandler::request_initial_tree` is called on the adapter's own thread,
/// which is why the tree is behind a lock rather than rebuilt on demand.
#[derive(Debug)]
pub struct Bridge {
    /// The whole tree as it stands, which is what an attaching client is given.
    current: Arc<Mutex<Option<TreeUpdate>>>,
    /// What clients have asked for and the host has not yet drained.
    asked: mpsc::Receiver<Asked>,
    /// The platform adapter, where this platform has one.
    #[cfg(target_os = "linux")]
    adapter: accesskit_unix::Adapter,
    /// Kept so that [`Bridge::requested`] answers with an empty list rather than a disconnected
    /// channel on a platform with no adapter to fill it.
    #[cfg(not(target_os = "linux"))]
    _asking: mpsc::Sender<Asked>,
}

/// One request as the platform's thread put it on the queue, before the tree is consulted.
///
/// The [`ActionData`] is kept because one of the three actions needs it: a text selection names
/// two positions inside text runs, and the *point* those are is a question about the tree — which
/// this thread may not ask, because the tree is behind the same lock the platform reads it
/// through. So the data crosses and [`Bridge::requested`] resolves it on the host's thread.
#[derive(Debug)]
struct Asked {
    /// Which node it was asked of.
    node: NodeId,
    /// What was asked.
    action: Action,
    /// Whatever the action carries, which is a selection for the only action that has one here.
    data: Option<ActionData>,
}

/// Answers the platform's request for a tree with whatever the host last published.
#[cfg(target_os = "linux")]
struct Activation {
    /// The same lock [`Bridge::current`] holds.
    current: Arc<Mutex<Option<TreeUpdate>>>,
}

#[cfg(target_os = "linux")]
impl accesskit::ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        // A poisoned lock means a panic while a tree was being stored, which under this
        // workspace's `panic = "abort"` cannot happen — and if it somehow did, answering `None`
        // is what the trait documents for "not ready yet": the adapter waits and asks again.
        self.current.lock().ok().and_then(|held| held.clone())
    }
}

/// Hands a client's request to the host rather than dropping it.
#[cfg(target_os = "linux")]
struct Actions {
    /// The host's end of the queue.
    asking: mpsc::Sender<Asked>,
    /// What wakes the host's event loop, because this runs on a thread the host is not in.
    ///
    /// **Without it a request is carried out whenever something else happens**, which on a window
    /// resting in `ControlFlow::Wait` is never: a screen reader asking to scroll to a paragraph
    /// would appear to do nothing until the person moved the mouse. That is not a latency
    /// question, it is the difference between the action working and not.
    wake: Box<dyn Fn() + Send>,
}

#[cfg(target_os = "linux")]
impl accesskit::ActionHandler for Actions {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        // The host has gone away: the program is shutting down and there is nobody to tell.
        let sent = self.asking.send(Asked {
            node: request.target_node,
            action: request.action,
            data: request.data,
        });
        if sent.is_ok() {
            (self.wake)();
        }
    }
}

/// Notices that the last assistive technology went away.
#[cfg(target_os = "linux")]
struct Deactivation;

#[cfg(target_os = "linux")]
impl accesskit::DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {
        // Nothing to release: the tree this crate holds is one page's worth of plain data, and
        // rebuilding it costs one page turn's work. The adapter will ask for it again.
    }
}

impl Bridge {
    /// Brings the platform adapter up.
    ///
    /// **Not on the launch path.** Called after the first frame is on the screen; on Linux it
    /// spawns `accesskit_unix`'s thread, which connects to the session bus and publishes nothing
    /// until an assistive technology is present.
    ///
    /// `wake` is called from that thread whenever a client asks for an action, and must do nothing
    /// but rouse the host's event loop — the request itself is read back by [`Self::requested`] on
    /// the host's own thread. A host with no loop to wake passes a closure that does nothing, and
    /// then carries requests out at whatever else makes it come round; this is deliberately an
    /// argument rather than a default, because the do-nothing case is the one that looks like a
    /// working bridge and is not.
    #[must_use]
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let current = Arc::new(Mutex::new(None));
        let (asking, asked) = mpsc::channel();
        #[cfg(target_os = "linux")]
        {
            let adapter = accesskit_unix::Adapter::new(
                Activation {
                    current: Arc::clone(&current),
                },
                Actions {
                    asking: asking.clone(),
                    wake: Box::new(wake),
                },
                Deactivation,
            );
            drop(asking);
            Self {
                current,
                asked,
                adapter,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            // No adapter, and therefore no handlers to give one: what this platform does instead
            // is `shortfall`, which says so in a sentence the host prints. The tree is still
            // built and still stored, so the half of this crate that maps §14.8.4 onto AccessKit
            // compiles and is tested here exactly as it is on Linux.
            //
            // `wake` is *dropped* rather than renamed to `_wake`, and the difference is the
            // whole reason this line exists. The parameter stays in the signature on every
            // platform so that a host writes one call, and dropping it here releases whatever
            // it captured at the same point in `new` where the Linux build hands it to
            // `Actions` — an underscore would say the argument is unused, where this says
            // where it goes and when.
            drop(wake);
            Self {
                current,
                asked,
                _asking: asking,
            }
        }
    }

    /// What this build cannot do, in a sentence a host prints.
    ///
    /// `None` on Linux, where AT-SPI is reached through `accesskit_unix`. `Some` on macOS and
    /// Windows, which have AccessKit adapters this program does not use — the same shape
    /// `pdf_sandbox::Confinement::shortfall` takes, and for the same reason: a platform that
    /// silently exposed nothing would look identical to one where the bridge is broken.
    #[must_use]
    pub fn shortfall() -> Option<&'static str> {
        #[cfg(target_os = "linux")]
        {
            None
        }
        #[cfg(not(target_os = "linux"))]
        {
            Some(
                "this build has no accessibility bridge — AccessKit's macOS and Windows adapters \
                 exist and are not wired in here, so a screen reader sees the window and not the \
                 page's structure. ISO 32000-2 §14.7's tree is still read and is still what \
                 `Query::AccessibilityTree` answers with",
            )
        }
    }

    /// Whether this build has an adapter that needs to be told where the window is.
    ///
    /// **A separate question from [`Self::shortfall`], and the reason is that the two will stop
    /// agreeing.** AT-SPI reports a node's bounds in *screen* coordinates, so an X11 host has to
    /// hand `accesskit_unix` the window's own place before any node is where it says it is.
    /// AccessKit's Windows and macOS adapters take a window *handle* and let the platform do
    /// that arithmetic, so when `doc/todo/31` wires them in, `shortfall` will answer `None` there
    /// while this still answers `false`. A host that had asked "is there a bridge" would then
    /// start paying for a position nobody wants.
    ///
    /// **What it is worth**: the two calls a host makes to satisfy this — winit's
    /// `outer_position` and `inner_position` — are **synchronous X11 round trips**, measured at
    /// **1.8 to 3.2 ms together** on this machine over twenty page turns (ADR 0228). That was
    /// being paid on every page turn, on every platform, including the two where the result was
    /// dropped by the `#[cfg]` in [`Self::placed`].
    #[must_use]
    pub const fn wants_window_bounds() -> bool {
        cfg!(target_os = "linux")
    }

    /// Publishes a page.
    ///
    /// Stores the tree for a client that attaches later, and gives it to the platform now if one
    /// is already attached. `update_if_active` does nothing at all while no assistive technology
    /// is present, which is what makes calling this on every page turn free.
    pub fn publish(&mut self, view: &crate::DocumentView) {
        let update = crate::tree::build(view);
        if let Ok(mut held) = self.current.lock() {
            *held = Some(update.clone());
        }
        #[cfg(target_os = "linux")]
        self.adapter.update_if_active(|| update);
        #[cfg(not(target_os = "linux"))]
        drop(update);
    }

    /// Tells the platform where the window is, which is what X11 needs to place a node.
    ///
    /// AT-SPI reports a node's position in *screen* coordinates and this crate's bounds are in
    /// the window's, so the adapter needs the window's own place to add them. Under Wayland an
    /// application cannot learn its own position and `accesskit_unix` says so; this is called
    /// where the host knows, and skipped where it does not.
    #[cfg_attr(
        not(target_os = "linux"),
        expect(
            unused_variables,
            reason = "the platforms with no adapter have nowhere to put this, and \
                      `shortfall` is what says so"
        )
    )]
    pub fn placed(&mut self, outer: (f32, f32, f32, f32), inner: (f32, f32, f32, f32)) {
        #[cfg(target_os = "linux")]
        self.adapter
            .set_root_window_bounds(rect(outer), rect(inner));
    }

    /// What clients have asked for since this was last called, each with what it means here.
    ///
    /// Drained rather than kept, and **resolved against the tree the client walked** rather than
    /// against a second copy of the page: the request names a node, the published
    /// [`accesskit::TreeUpdate`] says where that node is, and a place is the whole of what any of
    /// the three actions needs. That is one model of the page instead of two, and it cannot drift
    /// from what an assistive technology was told — which the second copy could, in exactly the
    /// window between a page turn and the request that followed it.
    pub fn requested(&mut self) -> Vec<Requested> {
        let asked: Vec<Asked> = self.asked.try_iter().collect();
        if asked.is_empty() {
            return Vec::new();
        }
        let held = self.current.lock().ok().and_then(|held| held.clone());
        asked
            .into_iter()
            .map(|one| Requested {
                node: one.node,
                action: one.action,
                means: held
                    .as_ref()
                    .and_then(|tree| means(tree, one.node, one.action, one.data.as_ref())),
            })
            .collect()
    }
}

/// What one request means on the page that was published, or `None` where nothing can say.
///
/// `None` is a real answer and is printed by the host: a node the update does not hold — the tree
/// moved under the request — an element with no place, or an action this program declares nowhere
/// and cannot invent a meaning for.
fn means(tree: &TreeUpdate, id: NodeId, action: Action, data: Option<&ActionData>) -> Option<Act> {
    match action {
        // `Action::ScrollToPoint` is deliberately not here: AT-SPI's `Component.ScrollToPoint`
        // asks for the node to be moved *to a stated point*, which is a different request from
        // "make it visible", and answering it with this one would be carrying out something
        // nobody asked for. It is printed by name instead.
        Action::ScrollIntoView => {
            let bounds = held(tree, id)?.bounds()?;
            Some(Act::Show {
                at: corners(bounds),
            })
        }
        Action::Click => {
            let bounds = held(tree, id)?.bounds()?;
            Some(Act::Click { at: centre(bounds) })
        }
        Action::SetTextSelection => match data? {
            ActionData::SetTextSelection(selection) => Some(Act::Caret {
                from: caret(tree, selection.anchor)?,
                to: caret(tree, selection.focus)?,
            }),
            _ => None,
        },
        _ => None,
    }
}

/// One node of the published update, by identifier.
fn held(tree: &TreeUpdate, id: NodeId) -> Option<&Node> {
    tree.nodes
        .iter()
        .find(|(at, _)| *at == id)
        .map(|(_, node)| node)
}

/// Where a text position is on the screen, as the point a press at it would land on.
///
/// AccessKit states a position as a character's index inside a [`accesskit::Role::TextRun`], and
/// the run carries each character's offset along its own reading direction and its width. The
/// point wanted is the character's **leading** edge, because that is what a caret before it means
/// — and it is also what `viewer_core`'s own hit test answers with: a press at a glyph's leading
/// edge is nearer that glyph than any other and lands on the near half of it, so it becomes the
/// offset in front of the character rather than behind it.
///
/// The index may be one past the last character, which AccessKit defines as the end of the line;
/// the point is then the last character's trailing edge, which the same hit test reads as the
/// offset after it.
fn caret(tree: &TreeUpdate, position: TextPosition) -> Option<(f32, f32)> {
    let run = held(tree, position.node)?;
    let bounds = run.bounds()?;
    let positions = run.character_positions()?;
    let widths = run.character_widths()?;
    // An index past the last character is the end of the line, which AccessKit states outright:
    // the point is then the last character's trailing edge.
    let along = if let Some(start) = positions.get(position.character_index) {
        *start
    } else {
        positions.last()? + widths.last().copied().unwrap_or(0.0)
    };
    // The inverse of what `tree::along` wrote, one axis at a time, with the other axis taken at
    // the run's middle: a run is one line of glyphs, so its centre across the reading direction is
    // inside every one of them.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the run's rectangle was built from `f32` corners in `tree::surrounding`; the \
                  widening to `f64` is AccessKit's rectangle type and the narrowing back is exact"
    )]
    let point = match run.text_direction().unwrap_or(TextDirection::LeftToRight) {
        TextDirection::LeftToRight => (bounds.x0 as f32 + along, middle(bounds.y0, bounds.y1)),
        TextDirection::RightToLeft => (bounds.x1 as f32 - along, middle(bounds.y0, bounds.y1)),
        TextDirection::TopToBottom => (middle(bounds.x0, bounds.x1), bounds.y0 as f32 + along),
        TextDirection::BottomToTop => (middle(bounds.x0, bounds.x1), bounds.y1 as f32 - along),
    };
    Some(point)
}

/// The middle of two edges of AccessKit's rectangle, in this program's own pixels.
#[expect(
    clippy::cast_possible_truncation,
    reason = "see `caret`: these corners were `f32` before AccessKit widened them"
)]
fn middle(low: f64, high: f64) -> f32 {
    ((low + high) * 0.5) as f32
}

/// A published rectangle back in the device pixels a `Command` takes.
#[expect(
    clippy::cast_possible_truncation,
    reason = "see `caret`: these corners were `f32` before AccessKit widened them"
)]
fn corners(bounds: Rect) -> [f32; 4] {
    [
        bounds.x0 as f32,
        bounds.y0 as f32,
        bounds.x1 as f32,
        bounds.y1 as f32,
    ]
}

/// The middle of a published rectangle, which is where a click on the node lands.
fn centre(bounds: Rect) -> (f32, f32) {
    (middle(bounds.x0, bounds.x1), middle(bounds.y0, bounds.y1))
}

/// A rectangle in the platform's own type.
#[cfg(target_os = "linux")]
fn rect(bounds: (f32, f32, f32, f32)) -> Rect {
    Rect {
        x0: f64::from(bounds.0),
        y0: f64::from(bounds.1),
        x1: f64::from(bounds.2),
        y1: f64::from(bounds.3),
    }
}

#[cfg(test)]
mod tests {
    use accesskit::{
        Action, ActionData, Node, NodeId, Role, TextPosition, TextSelection, Tree, TreeId,
        TreeUpdate,
    };

    use super::{Act, Bridge, means};

    /// A tree of one node, which is all any of these questions is about.
    fn published(id: NodeId, node: Node) -> TreeUpdate {
        TreeUpdate {
            nodes: vec![(id, node)],
            tree: Some(Tree::new(id)),
            tree_id: TreeId::ROOT,
            focus: id,
        }
    }

    /// A request to show a node answers with the node's own rectangle.
    #[test]
    fn showing_a_node_answers_with_where_it_is() {
        let mut figure = Node::new(Role::Image);
        figure.set_bounds(accesskit::Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 30.0,
            y1: 60.0,
        });
        let tree = published(NodeId(16), figure);
        assert_eq!(
            means(&tree, NodeId(16), Action::ScrollIntoView, None),
            Some(Act::Show {
                at: [10.0, 20.0, 30.0, 60.0]
            })
        );
    }

    /// A click answers with the middle of the node, which is where the client's user was told it is.
    #[test]
    fn clicking_a_node_answers_with_its_middle() {
        let mut widget = Node::new(Role::CheckBox);
        widget.set_bounds(accesskit::Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 30.0,
            y1: 60.0,
        });
        let tree = published(NodeId(16), widget);
        assert_eq!(
            means(&tree, NodeId(16), Action::Click, None),
            Some(Act::Click { at: (20.0, 40.0) })
        );
    }

    /// A caret answers with the leading edge of the character it names, on the run's own baseline.
    ///
    /// The arithmetic is the inverse of what `tree::along` wrote: the run's rectangle plus the
    /// character's offset along the reading direction. The **third** case is the one worth pinning
    /// — an index one past the last character is AccessKit's end of the line, and the point is then
    /// the last character's trailing edge, which `viewer_core`'s hit test reads as the offset after
    /// it rather than before.
    #[test]
    fn a_caret_answers_with_the_leading_edge_of_the_character_it_names() {
        let mut run = Node::new(Role::TextRun);
        run.set_bounds(accesskit::Rect {
            x0: 100.0,
            y0: 200.0,
            x1: 130.0,
            y1: 210.0,
        });
        run.set_character_positions(vec![0.0, 10.0, 20.0]);
        run.set_character_widths(vec![10.0, 10.0, 10.0]);
        let tree = published(NodeId(2_000_000), run);
        let at = |index: usize| TextPosition {
            node: NodeId(2_000_000),
            character_index: index,
        };
        let asked = |anchor, focus| {
            means(
                &tree,
                NodeId(2),
                Action::SetTextSelection,
                Some(&ActionData::SetTextSelection(TextSelection {
                    anchor: at(anchor),
                    focus: at(focus),
                })),
            )
        };
        assert_eq!(
            asked(0, 0),
            Some(Act::Caret {
                from: (100.0, 205.0),
                to: (100.0, 205.0)
            })
        );
        assert_eq!(
            asked(1, 2),
            Some(Act::Caret {
                from: (110.0, 205.0),
                to: (120.0, 205.0)
            })
        );
        assert_eq!(
            asked(0, 3),
            Some(Act::Caret {
                from: (100.0, 205.0),
                to: (130.0, 205.0)
            }),
            "one past the last character is the end of the line"
        );
    }

    /// What cannot be placed answers `None`, which the host prints by name.
    ///
    /// Three ways, and each is a different thing going wrong: a node the published tree no longer
    /// holds, an element with no place, and an action this program declares nowhere. Trap 5 is why
    /// all three are one answer rather than a silent no-op — the host says the action arrived and
    /// says it was not carried out.
    #[test]
    fn what_cannot_be_placed_says_so_rather_than_doing_something_else() {
        let tree = published(NodeId(16), Node::new(Role::Paragraph));
        assert_eq!(means(&tree, NodeId(99), Action::Click, None), None);
        assert_eq!(means(&tree, NodeId(16), Action::Click, None), None);
        assert_eq!(
            means(&tree, NodeId(16), Action::ShowContextMenu, None),
            None
        );
        // And a selection with no data, which is a client sending the action without its argument.
        assert_eq!(
            means(&tree, NodeId(16), Action::SetTextSelection, None),
            None
        );
    }

    /// A build with no adapter has nowhere to put a window's place on the screen.
    ///
    /// The two questions are separate on purpose — see [`Bridge::wants_window_bounds`] — and
    /// they are separate in one direction only: an adapter may not need the bounds, but the
    /// absence of an adapter certainly does not need them. A host pays two synchronous X11
    /// round trips to answer this, so the implication is worth a guard rather than a comment:
    /// `doc/todo/31` will make `shortfall` answer `None` on two more platforms, and if
    /// `wants_window_bounds` were ever written as its negation this would be the thing that
    /// noticed.
    #[test]
    fn a_build_with_no_bridge_wants_no_window_bounds() {
        assert!(
            Bridge::shortfall().is_none() || !Bridge::wants_window_bounds(),
            "a platform with no adapter asked to be told where the window is"
        );
    }
}
