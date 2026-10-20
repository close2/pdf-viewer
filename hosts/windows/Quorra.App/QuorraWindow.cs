// quorra_window and quorra_settings as two owned C# objects: the one file of this window that calls
// Quorra.Interop's pointer-level declarations (RFC 0010 sections 3.2a and 5, ADRs 2088, 2120, 2136).
// Each method is one entry point, its status checked, its strings and pixels copied into managed
// memory before it returns, so nothing the library owns outlives the call that lent it.

using System;
using System.Collections.Generic;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>One record of what the window owes: its kind, its text and its document.</summary>
/// <param name="Kind">A QUORRA_OWED_* kind.</param>
/// <param name="Text">The record's text, for the kinds quorra_owed_text answers; empty for others.</param>
/// <param name="Document">The document a PASSWORD, SCRIPT or SCRIPT_WITHDRAWN record is about.</param>
internal readonly record struct Owed(uint Kind, string Text, ulong Document);

/// <summary>One picture the page area shows, its pixels QUORRA_FORMAT_RGBA8 with straight alpha.</summary>
/// <param name="Width">The raster's width in pixels.</param>
/// <param name="Height">The raster's height in pixels.</param>
/// <param name="OriginX">Where the window shows it, in device pixels of the viewport.</param>
/// <param name="OriginY">Where the window shows it, in device pixels of the viewport.</param>
/// <param name="ShownWidth">The extent it is shown at, which differs from the raster's for a frame from before a zoom.</param>
/// <param name="ShownHeight">The extent it is shown at.</param>
/// <param name="Pixels">Four bytes a pixel, rows top to bottom with no padding.</param>
internal sealed record Picture(
    uint Width,
    uint Height,
    float OriginX,
    float OriginY,
    uint ShownWidth,
    uint ShownHeight,
    byte[] Pixels);

/// <summary>The reader's levels a window opens under, read from command-line words (ADR 2120).</summary>
internal sealed unsafe class QuorraSettings : IDisposable
{
    private nint handle;

    private QuorraSettings(nint handle)
    {
        this.handle = handle;
    }

    /// <summary>What a window with no command line holds.</summary>
    internal static QuorraSettings Create()
    {
        nint settings = 0;
        Library.Check("quorra_settings_new", Native.quorra_settings_new(&settings));
        return new QuorraSettings(settings);
    }

    /// <summary>
    /// One command-line word, read as quorra, quorra-gtk and quorra-qt read theirs: null where it
    /// is now in force, and otherwise the sentence saying why it is not.
    /// </summary>
    internal string? Take(string word)
    {
        byte taken = 0;
        int status = Native.quorra_settings_take(handle, word, &taken);
        if (status == Native.QUORRA_WRONG_KIND)
        {
            nint settings = handle;
            return Library.ReadText(
                "quorra_settings_refusal",
                (into, cap, needed) => Native.quorra_settings_refusal(settings, into, cap, needed));
        }
        Library.Check("quorra_settings_take", status);
        return taken != 0 ? null : $"{word}: not a word this window takes";
    }

    /// <summary>The handle, for quorra_window_open_under, which takes a copy.</summary>
    internal nint Handle => handle;

    /// <summary>Releases the handle; a window opened under it keeps its own copy.</summary>
    public void Dispose()
    {
        if (handle != 0)
        {
            Native.quorra_settings_free(handle);
            handle = 0;
        }
    }
}

/// <summary>quorra_window: the loop the toolkit windows share, held for this one.</summary>
internal sealed unsafe class QuorraWindow : IDisposable
{
    private nint handle;

    private QuorraWindow(nint handle)
    {
        this.handle = handle;
    }

    /// <summary>Opens a window on the file at <paramref name="path"/>, at the levels stated.</summary>
    /// <remarks>Nothing is drawn until the first <see cref="Resize"/>, which joins the open.</remarks>
    internal static QuorraWindow Open(string path, QuorraSettings settings)
    {
        nint window = 0;
        Library.Check(
            "quorra_window_open_under",
            Native.quorra_window_open_under(path, null, settings.Handle, &window));
        return new QuorraWindow(window);
    }

    /// <summary>Another document in the strip, brought to the front.</summary>
    internal void OpenBeside(string path) =>
        Library.Check("quorra_window_open_beside", Native.quorra_window_open_beside(handle, path));

    /// <summary>The viewport in device pixels, at <paramref name="scale"/> device pixels per logical one.</summary>
    internal void Resize(uint width, uint height, float scale) =>
        Library.Check("quorra_window_resize", Native.quorra_window_resize(handle, width, height, scale));

    /// <summary>One look at what runs beside the window.</summary>
    internal void Tick() => Library.Check("quorra_window_tick", Native.quorra_window_tick(handle));

    /// <summary>Milliseconds until the window wants <see cref="Tick"/>, or -1 while nothing runs beside it.</summary>
    internal int Interval => Native.quorra_window_interval(handle);

    /// <summary>A key, with Shift and the platform's command modifier; the QUORRA_ACT_* left for this window.</summary>
    internal uint Press(uint key, bool shift, bool command)
    {
        uint act = Native.QUORRA_ACT_NOTHING;
        float by = 0;
        Library.Check("quorra_window_press", Native.quorra_window_press(handle, key, shift, command, &act, &by));
        return act;
    }

    /// <summary>The wheel, in device pixels; a positive <paramref name="dy"/> moves the content up.</summary>
    internal void Scroll(float dx, float dy) =>
        Library.Check("quorra_window_scroll", Native.quorra_window_scroll(handle, dx, dy));

    /// <summary>The pointer in device pixels of the viewport; whether it is now over a link.</summary>
    internal bool Pointer(float x, float y, uint action)
    {
        byte overLink = 0;
        Library.Check("quorra_window_pointer", Native.quorra_window_pointer(handle, x, y, action, &overLink));
        return overLink != 0;
    }

    /// <summary>The page area is on the screen; the first call that shows a page is the launch's first frame.</summary>
    internal void Presented() => Library.Check("quorra_window_presented", Native.quorra_window_presented(handle));

    /// <summary>How many documents the strip holds, and which is in front.</summary>
    internal (nuint Count, nuint Focused) Strip =>
        (Native.quorra_window_document_count(handle), Native.quorra_window_focused_document(handle));

    /// <summary>Brings the strip's document at <paramref name="index"/> to the front.</summary>
    internal void Choose(nuint index) =>
        Library.Check("quorra_window_choose_document", Native.quorra_window_choose_document(handle, index));

    /// <summary>Closes the document in front; true where it was the last, and the window is the caller's to close.</summary>
    internal bool CloseDocument()
    {
        byte last = 0;
        Library.Check("quorra_window_close_document", Native.quorra_window_close_document(handle, &last));
        return last != 0;
    }

    /// <summary>The answer to a PASSWORD record: null for a prompt closed without one.</summary>
    internal void Password(ulong document, string? password) =>
        Library.Check("quorra_window_password", Native.quorra_window_password(handle, document, password));

    /// <summary>The answer to a QUESTION record.</summary>
    internal void Answer(bool proceed) =>
        Library.Check("quorra_window_answer", Native.quorra_window_answer(handle, proceed));

    /// <summary>The answer to an ARCHIVE record: a negative index is a chooser closed with nothing chosen.</summary>
    internal void Archive(int target, int profile) =>
        Library.Check("quorra_window_archive", Native.quorra_window_archive(handle, target, profile));

    /// <summary>The answer to a SCRIPT record, as a QUORRA_SCRIPT_ANSWER_*.</summary>
    internal void AnswerScript(ulong document, uint how) =>
        Library.Check(
            "quorra_window_answer_script",
            Native.quorra_window_answer_script(handle, document, how, 0, null));

    /// <summary>The print a PRINT record let through is over.</summary>
    internal void PrintFinished() =>
        Library.Check("quorra_window_print_finish", Native.quorra_window_print_finish(handle));

    /// <summary>Everything owed since the last call, copied out and the batch freed.</summary>
    internal List<Owed> TakeOwed()
    {
        nint taken = 0;
        Library.Check("quorra_window_take_owed", Native.quorra_window_take_owed(handle, &taken));
        // A copy, because a local whose address was taken cannot be captured by the lambda below.
        nint owed = taken;
        try
        {
            var records = new List<Owed>();
            nuint count = Native.quorra_owed_count(owed);
            for (nuint index = 0; index < count; index++)
            {
                uint kind = 0;
                Library.Check("quorra_owed_kind", Native.quorra_owed_kind(owed, index, &kind));
                nuint at = index;
                string text = CarriesText(kind)
                    ? Library.ReadText(
                        "quorra_owed_text",
                        (into, cap, needed) => Native.quorra_owed_text(owed, at, into, cap, needed))
                    : string.Empty;
                ulong document = 0;
                if (kind is Native.QUORRA_OWED_PASSWORD
                    or Native.QUORRA_OWED_SCRIPT
                    or Native.QUORRA_OWED_SCRIPT_WITHDRAWN)
                {
                    Library.Check("quorra_owed_document", Native.quorra_owed_document(owed, index, &document));
                }
                records.Add(new Owed(kind, text, document));
            }
            return records;
        }
        finally
        {
            Native.quorra_owed_free(owed);
        }
    }

    /// <summary>The pictures the page area shows: the viewer's frames and the stand-ins.</summary>
    internal List<Picture> Pictures()
    {
        var pictures = new List<Picture>();
        nuint count = Native.quorra_window_frame_count(handle);
        for (nuint frame = 0; frame < count; frame++)
        {
            quorra_frame info = default;
            uint shownWidth = 0;
            uint shownHeight = 0;
            Library.Check(
                "quorra_window_frame",
                Native.quorra_window_frame(handle, frame, &info, &shownWidth, &shownHeight));
            // One layout exists, and it is a number rather than an assumption so that a second
            // would be refused here by name instead of drawn as the wrong colours (ADR 0247).
            if (info.format != Native.QUORRA_FORMAT_RGBA8)
            {
                throw new QuorraException($"quorra_window_frame: picture {frame} is in layout {info.format}, which this window does not draw");
            }
            if (info.bytes > int.MaxValue)
            {
                throw new QuorraException($"quorra_window_frame: picture {frame} is {info.bytes} bytes, past what a .NET array holds");
            }
            byte[] pixels = new byte[(int)info.bytes];
            nuint written = 0;
            fixed (byte* into = pixels)
            {
                Library.Check(
                    "quorra_window_frame_copy",
                    Native.quorra_window_frame_copy(handle, frame, into, info.bytes, &written));
            }
            pictures.Add(new Picture(
                info.width,
                info.height,
                info.origin_x,
                info.origin_y,
                shownWidth,
                shownHeight,
                pixels));
        }
        return pictures;
    }

    /// <summary>Releases the window and every document it holds.</summary>
    public void Dispose()
    {
        if (handle != 0)
        {
            Native.quorra_window_free(handle);
            handle = 0;
        }
    }

    // The kinds quorra_owed_text answers; it refuses the others with QUORRA_WRONG_KIND (quorra.h).
    private static bool CarriesText(uint kind) => kind is Native.QUORRA_OWED_SAID
        or Native.QUORRA_OWED_TITLE
        or Native.QUORRA_OWED_PASSWORD
        or Native.QUORRA_OWED_QUESTION
        or Native.QUORRA_OWED_CLIPBOARD
        or Native.QUORRA_OWED_ARCHIVE
        or Native.QUORRA_OWED_SCRIPT
        or Native.QUORRA_OWED_PRINT;
}
