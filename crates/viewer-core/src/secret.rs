//! §7.6.4.1's password, held so that it cannot be printed and does not outlive its use.
//!
//! # Why this is a type and not a `String`
//!
//! [`Command::Open`]'s password was an `Option<String>` until the six-hundred-and-ninety-fifth
//! session, and [`Command`] derives [`Debug`]. Two of the three windowed hosts trace a command by
//! writing `format!("{command:?}")` into their launch log, so what stood between a reader's
//! password and a file on disk was the *field order* of a struct variant — `bytes` is declared
//! before `password` and a `Vec<u8>`'s `Debug` is about five characters a byte, so the hosts' own
//! 120-character truncation happened to cut the line before the secret. That is an accident, and
//! an accident is not a security property: a variant reordered, a truncation widened or a document
//! of twenty bytes would each have undone it silently.
//!
//! So the password is its own type, and the type is what carries the three obligations:
//!
//! - **it does not print.** [`Debug`] says how many characters there are and not one of them, and
//!   there is deliberately no [`Display`] and no `AsRef<str>` — the one way to read it is
//!   [`Secret::reveal`], which is named so that a reader of a call site can see the moment it
//!   happens;
//! - **it does not linger.** The buffer is overwritten when the value is dropped;
//! - **it does not grow into a second copy.** A password is edited a character at a time in a
//!   host's prompt, and a `String` that reallocates leaves the old bytes in freed memory where
//!   nothing can reach them to be cleared. [`Secret::new`] reserves [`RESERVED`] bytes for that
//!   reason, and the number is the standard's rather than a guess.
//!
//! # What the zeroing does and does not guarantee
//!
//! [`Secret::drop`] fills the buffer with zeroes and then hands it to [`std::hint::black_box`],
//! which is documented as opaque to the optimiser — so the write is not dead-store-eliminated in
//! any build this project produces. It is **best effort and says so**: `black_box` promises no
//! more than that, a value the compiler chose to keep in a register or spill to the stack is not
//! this buffer, and a page swapped to disk was never ours to clear.
//!
//! The alternative is `zeroize`, whose `Zeroizing` writes through a volatile pointer and gives the
//! guarantee outright. It was **not** taken, and the reason is `CLAUDE.md` principle 3 rather than
//! dependency-counting: a volatile write is `unsafe` code, this crate carries
//! `#![forbid(unsafe_code)]` because it touches PDF bytes, and a dependency added to reach an
//! `unsafe` primitive is that rule paid off in another crate's ledger. What is written down here
//! is the weaker guarantee, honestly, rather than a stronger one bought elsewhere.
//!
//! [`Command`]: crate::Command
//! [`Command::Open`]: crate::Command::Open
//! [`Display`]: std::fmt::Display

/// How many bytes a password buffer reserves before anybody types into it.
///
/// **The standard's number rather than a chosen one.** ISO 32000-2 §7.6.4.1, on a revision 6
/// password:
///
/// > the password string shall be converted to UTF-8 encoding, and then truncated to the first
/// > 127 bytes if the string is longer than 127 bytes
///
/// So 127 bytes is where the standard stops reading, and a buffer of 128 is a buffer no password
/// that can affect the outcome will ever grow out of. A longer one would reallocate — leaving the
/// bytes typed so far in freed memory that [`Secret::drop`] can no longer reach — and it would
/// also be a password whose tail the file encryption key algorithm never sees.
pub const RESERVED: usize = 128;

/// §7.6.4.1's user or owner password.
///
/// Built from what a person typed, handed to [`crate::Command::Open`], and dropped with the
/// command. See the module documentation for what it guarantees and what it does not.
///
/// **Not [`Clone`], deliberately.** A copy is a second buffer to clear and a second lifetime to
/// reason about, and no consumer in this tree has needed one: a host that must open the same
/// document twice asks the person again, which is what §7.6.4.1's prompt is.
pub struct Secret {
    /// What was typed, in the buffer [`Self::new`] reserved for it.
    text: String,
}

impl Secret {
    /// An empty password, with room for one the standard will read whole.
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: String::with_capacity(RESERVED),
        }
    }

    /// Appends what one key press typed.
    ///
    /// A host's prompt is edited a character at a time, so this is the ordinary way a `Secret`
    /// comes to hold anything — and it is why [`RESERVED`] exists.
    pub fn push_str(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// Removes the last character. Answers whether there was one.
    pub fn backspace(&mut self) -> bool {
        self.text.pop().is_some()
    }

    /// Whether nothing has been typed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// How many characters have been typed.
    ///
    /// Characters rather than bytes, because the one thing a prompt draws from this is a row of
    /// Table 231 bit 14's bullets, and a bullet stands for a character a person pressed.
    #[must_use]
    pub fn characters(&self) -> usize {
        self.text.chars().count()
    }

    /// The password itself.
    ///
    /// Named for what it does. The only caller that should reach for it is the one about to hand
    /// the password to [`pdf_syntax`]'s authentication — anything else is a copy this type exists
    /// to prevent.
    #[must_use]
    pub fn reveal(&self) -> &str {
        &self.text
    }
}

impl Default for Secret {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for Secret {
    /// A password this program was handed whole — a C caller's argument, a decoded message.
    ///
    /// The `String` moves in rather than being copied, so there is one buffer; what this cannot
    /// do is clear whatever the *caller's* copy was in, which is why a host that builds one
    /// character by character uses [`Secret::new`] instead.
    fn from(text: String) -> Self {
        Self { text }
    }
}

/// How many characters, and not one of them.
///
/// The whole reason this type exists: a host that traces a [`crate::Command`] gets a line saying a
/// password was there and how long it was, which is what a launch log has any business knowing.
impl std::fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "Secret({} character(s))", self.characters())
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // `into_bytes` hands back the *same* allocation — `String` is a `Vec<u8>` with a promise
        // about its contents — so this clears the buffer the password was typed into rather than
        // a copy of it. `black_box` is what stops the write being removed as dead: nothing reads
        // `bytes` afterwards, and without it an optimiser is entitled to notice.
        let mut bytes = std::mem::take(&mut self.text).into_bytes();
        bytes.fill(0);
        std::hint::black_box(&mut bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::{RESERVED, Secret};

    /// The property the type exists for: a trace of a command says nothing a password is made of.
    #[test]
    fn a_password_does_not_print_itself() {
        let mut secret = Secret::new();
        secret.push_str("correct horse battery staple");
        let printed = format!("{secret:?}");
        assert!(
            !printed.contains("horse"),
            "the password printed itself: {printed}"
        );
        assert!(
            printed.contains("28"),
            "a trace should still say how long it was: {printed}"
        );
    }

    /// And through the command it is carried in, which is where the leak actually was.
    #[test]
    fn a_command_carrying_one_does_not_print_it_either() {
        let mut secret = Secret::new();
        secret.push_str("hunter2");
        let command = crate::Command::Open {
            id: crate::DocumentId(1),
            bytes: b"%PDF-1.7".to_vec().into(),
            password: Some(secret),
            fragment: None,
        };
        let printed = format!("{command:?}");
        assert!(
            !printed.contains("hunter2"),
            "the password reached a trace: {printed}"
        );
    }

    /// §7.6.4.1's 127-byte truncation is the reason for the number, so a password the standard
    /// reads whole is one this buffer never reallocates for.
    #[test]
    fn a_password_the_standard_reads_whole_never_outgrows_its_buffer() {
        let mut secret = Secret::new();
        let capacity = secret.text.capacity();
        assert!(capacity >= RESERVED);
        for _ in 0..127 {
            secret.push_str("x");
        }
        assert_eq!(
            secret.text.capacity(),
            capacity,
            "the buffer moved, so the bytes typed so far are in freed memory"
        );
    }

    /// Backspace and the count a prompt draws its bullets from.
    #[test]
    fn a_prompt_can_edit_and_count_without_reading() {
        let mut secret = Secret::new();
        secret.push_str("ätest");
        assert_eq!(secret.characters(), 5, "characters, not bytes");
        assert!(secret.backspace());
        assert_eq!(secret.characters(), 4);
        assert_eq!(secret.reveal(), "ätes");
        while secret.backspace() {}
        assert!(secret.is_empty());
        assert!(!secret.backspace(), "an empty password has nothing to undo");
    }
}
