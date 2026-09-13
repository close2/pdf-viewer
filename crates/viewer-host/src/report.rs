//! When a host asks the document what it says about itself.
//!
//! `viewer_core::Command::Report` answers eight clauses about the *file* rather than about any
//! page — §12.11's requirements, §12.8's signatures, §7.11.4's embedded files, §14.13.2's
//! associated files that are not embedded, §7.5's rebuilt cross-reference table, Annex I's
//! version, §14.8.6.2's namespaces and §14.8.6.3's unenclosed `MathML`. The core used to say them
//! as part of opening the document and no longer does, because §12.8.1's byte range digest reads
//! and hashes the signed part of the file and `CLAUDE.md` principle 2 keeps off a launch
//! "[a]nything not needed to show page one".
//!
//! **So the asking is a host's, and *when* is the only decision in it.** Once the reader has
//! their page: every host here acknowledges the frame it has just presented, and that is the
//! moment. Asking earlier would move the same work back in front of the first page while hiding
//! it from `crates/viewer-ui/tests/launch_path.rs`, which measures `Command::Open` — a gate
//! green and a program no faster, which is the worst of the four outcomes.
//!
//! One small state machine here rather than four fields in four hosts, for `password::Asking`'s
//! reason: the rule is the same in all of them and a host that forgot it would go quietly silent
//! about a signature. ADR 1044.

/// Whether the document that is open still owes its report.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Due(bool);

impl Due {
    /// A document opened, so its report is owed.
    ///
    /// Every document, rather than the first: Annex O's `ef` and §12.6.4.4's embedded go-to both
    /// put a second document in front of the reader without restarting the program, and what the
    /// *new* file says about itself is a different set of sentences.
    pub const fn opened(&mut self) {
        self.0 = true;
    }

    /// A frame has been presented; whether this is the moment to ask.
    ///
    /// True exactly once per opened document: the report does not change while a document is
    /// open — it is a function of an immutable file — so a host that asked on every frame would
    /// print the same sentences at every scroll.
    pub const fn after_a_frame(&mut self) -> bool {
        let owed = self.0;
        self.0 = false;
        owed
    }
}
