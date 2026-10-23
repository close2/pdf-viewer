//! The two refusals this transport states: a message it does not carry, and one it cannot read.
//!
//! The bytes are unchanged by the split that made this module (ADR 2256).

/// A message this transport deliberately does not carry, and why.
///
/// Produced where a `match` over `viewer-core`'s vocabulary reaches a variant that stays on one
/// side of the boundary. It is an error rather than a silent skip because a host that asked for
/// something and got nothing back would have no way to tell that from an answer of nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{message} does not cross this boundary: {reason}")]
pub struct Uncarried {
    /// The variant's name, as `viewer-core` spells it.
    pub message: &'static str,
    /// Why it stays where it is.
    pub reason: &'static str,
}

/// Why a message could not be read.
///
/// Every variant names what was being read when it failed, because "malformed input" without a
/// subject is a diagnosis nobody can act on.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// The bytes ended in the middle of a field.
    #[error("the message ended while reading {what}")]
    Truncated {
        /// What was being read.
        what: &'static str,
    },
    /// A discriminant this build does not define.
    #[error("{what} {value} is not one this build defines")]
    Unrecognised {
        /// Which enumeration.
        what: &'static str,
        /// The value on the wire.
        value: u32,
    },
    /// A string field was not UTF-8.
    ///
    /// Every string crossing this boundary has already been decoded from the document's own
    /// encoding by `pdf-syntax`, so one that is not UTF-8 here is a transport fault rather than
    /// a document's.
    #[error("{what} was not UTF-8")]
    NotText {
        /// Which field.
        what: &'static str,
    },
    /// A length field describes more bytes than the message holds.
    #[error("{what} claims {claimed} bytes and the message holds {available}")]
    Overlong {
        /// Which field.
        what: &'static str,
        /// What the length said.
        claimed: usize,
        /// What was left.
        available: usize,
    },
    /// Bytes were left over after a message was read.
    ///
    /// Checked rather than ignored: a trailing field this build does not know about means the
    /// two sides disagree about the format, and the magic did not catch it.
    #[error("{left} byte(s) left after reading {what}")]
    Trailing {
        /// What was read.
        what: &'static str,
        /// How many bytes were not consumed.
        left: usize,
    },
    /// A tree on the wire nests deeper than this reader follows.
    ///
    /// Four of the answers a panel is made of are trees — the outline, the layer order,
    /// §12.3.5.2's folders and a collection's own items — and a decoder that followed one as deep
    /// as it was told to would let a message of a few hundred bytes exhaust the host's stack.
    /// The bound is `panels::MAX_TREE_DEPTH` and it is well past what any of the readers that
    /// *produce* these trees will hand over.
    #[error("{what} nests deeper than the {limit} this reader follows")]
    TooDeep {
        /// Which tree.
        what: &'static str,
        /// How deep this reader goes.
        limit: usize,
    },
    /// A field whose bytes are there and which this process could not find room for.
    ///
    /// **Not a malformed message.** The length was checked against what the message holds before
    /// this is reached, so the sender told the truth and it is the reader that ran out. It is a
    /// variant rather than an abort because the confined worker runs under an address-space
    /// ceiling: an allocation that crosses `RLIMIT_AS` kills the process, and a host learns of
    /// that as a signal number, which looks exactly like a crash. `try_reserve` turns it into
    /// this, and this into a refusal naming the field and the size.
    #[error("{what} needs {bytes} bytes and this process could not find room for them")]
    NoRoom {
        /// Which field.
        what: &'static str,
        /// How many bytes it needed.
        bytes: usize,
    },
    /// An index into a table larger than the table the message stated.
    ///
    /// A display list is mostly indices — a clip identifier, a soft mask identifier, a path,
    /// a shading — and every one of them is checked against a table this decoder has already
    /// read *whole*, never against a length the message asserts. This is that refusal. See
    /// `display_list`, whose tables precede its command stream for exactly this reason.
    #[error("{what} names entry {index} of a table holding {held}")]
    OutOfTable {
        /// Which table.
        what: &'static str,
        /// The index the message stated.
        index: usize,
        /// How many entries the table read before it holds.
        held: usize,
    },
    /// The message says a descriptor rides beside it, and none arrived.
    ///
    /// A document on disk crosses as its open file, sent with `SCM_RIGHTS` beside the frame's
    /// header (ADR 0812). A frame that states the descriptor form and arrives with nothing
    /// beside it is one of three things — a sender on a pipe rather than a socket, a receiver
    /// whose descriptor table was full so the kernel dropped it (`MSG_CTRUNC`), or a message
    /// somebody made up — and in all three the honest answer is a refusal naming the field
    /// rather than an empty document.
    #[error("{what} was to arrive as a descriptor beside the message, and none did")]
    NoDescriptor {
        /// Which field.
        what: &'static str,
    },
    /// The message is well formed and describes a value that cannot exist.
    ///
    /// **Not a truncation and not an unknown discriminant.** Every field was there and every
    /// discriminant was one this build defines; what the fields *say* is something the type
    /// they describe does not admit — an image whose dimensions and samples disagree, a clip
    /// table whose numbering cannot be rebuilt, a blending grid that is not `side⁴` samples.
    /// A host that accepted one would hand a rasteriser a value no interpreter can produce.
    #[error("{what} could not be rebuilt: {why}")]
    Unbuildable {
        /// Which value.
        what: &'static str,
        /// Why it is not one.
        why: &'static str,
    },
}
