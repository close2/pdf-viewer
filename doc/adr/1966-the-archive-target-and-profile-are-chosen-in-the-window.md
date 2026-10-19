# 1966 — The archive target and profile are chosen in the window, and the choice is kept

Session 1565, on `doc/todo/72` section 1 as ADR 1954 left it ("a menu or card that chooses the
target and the profile after launch"). Status: **accepted** and **built**. Context: ADRs 1942,
1954, 1145 (the restrictions card), RFC 0004 (print's shape).

## 1. What was missing

ADR 1954 made Shift+A convert only where the window was started with `--archive
<target>:<profile>`; with nothing chosen the key said how to restart the window. A person who had
opened a document and then wanted it archived had no way to do it from the window.

## 2. Decisions

1. **The key asks where nothing is chosen.** `viewer_host::archiving::pressed` answers
   `Pressed::Convert(Command::Archive(..))` for a choice the window holds and
   `Pressed::Choose(Choosing)` for none; there is still no default target, because which part and
   level an archive accepts is the person's to say — what changed is that they are asked in place.
2. **One list, read and never copied.** `Choosing` offers `Target::ALL` and
   `pdf_transform::profiles::SHIPPED` in their own order, each profile with the sentence its file
   states (`[profile] sentence`, read by `Configuration::read` into `Configuration::sentence` and
   reached through `Shipped::sentence`). `tests` in `profiles.rs` hold every shipped profile to a
   sentence. The keyboard starts on PDF/A-2b and `refuse-any-loss`, the profile that loses nothing,
   and nothing converts until the person confirms.
3. **Three faces of the one state.** `quorra` draws `viewer_ui::chrome::ArchiveCard` (the
   restrictions card's shape: ← → target, ↑ ↓ profile, Enter converts, Escape closes, a press
   chooses a cell or row and never converts); GTK puts a `GtkDropDown` of the targets over a
   `GtkListBox` of the profiles; Qt a `QComboBox` over a `QListWidget`, through the bridge's
   `archive_dialogue` flag and six calls. In all three the list has the keyboard, so Down then
   Enter chooses the second profile at the first target in every window (the drive's step 88).
   The native hosts reach the dialogue through one required trait method,
   `NativeWindow::offer_to_archive`, and answer through the provided `archive_chosen`. A C caller
   is its own chooser and gains the one thing it lacked, `quorra_archive_profile_sentence`.
4. **The choice is kept for the window's life**, in the field `--archive` fills, and the window
   says so (`archiving::remembered`): the next Shift+A converts without asking, exactly as a
   launch with the word would. Changing it means a new window; a second chooser for a window that
   already chose is not built, because no request has asked for one.
5. **Closing the chooser converts nothing and says so** (`NOT_CHOSEN`), so a closed dialogue is
   never silence.

## 3. Consequences

`--archive` stays the launch-time choice and is unchanged. The confined window still converts
nothing (ADR 1954 section 2.7) and its Shift+A still says so. `doc/todo/72` section 1 loses its
first owed item.
