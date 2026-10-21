// quorra_window and quorra_settings as two owned C# objects: the one file of this window that calls
// Quorra.Interop's pointer-level declarations (RFC 0010 sections 3.2a and 5, ADRs 2088, 2120, 2136).
// Each method is one entry point, its status checked, its strings and pixels copied into managed
// memory before it returns, so nothing the library owns outlives the call that lent it.

using System;
using System.Collections.Generic;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>One record of what the window owes: its kind, its text, its document and what a repaint changed.</summary>
/// <param name="Kind">A QUORRA_OWED_* kind.</param>
/// <param name="Text">The record's text, for the kinds quorra_owed_text answers; empty for others.</param>
/// <param name="Document">The document a PASSWORD, SCRIPT or SCRIPT_WITHDRAWN record is about.</param>
/// <param name="Changed">A REPAINT record's QUORRA_REPAINT_* bits; zero for others.</param>
/// <param name="Fresh">For a REPAINT record, whether each picture has pixels this window has not copied (ADR 2168).</param>
internal readonly record struct Owed(uint Kind, string Text, ulong Document, uint Changed, IReadOnlyList<bool> Fresh);

/// <summary>One picture the page area shows, its pixels QUORRA_FORMAT_RGBA8 with straight alpha.</summary>
/// <param name="Width">The raster's width in pixels.</param>
/// <param name="Height">The raster's height in pixels.</param>
/// <param name="OriginX">Where the window shows it, in device pixels of the viewport.</param>
/// <param name="OriginY">Where the window shows it, in device pixels of the viewport.</param>
/// <param name="ShownWidth">The extent it is shown at, which differs from the raster's for a frame from before a zoom.</param>
/// <param name="ShownHeight">The extent it is shown at.</param>
/// <param name="Pixels">Four bytes a pixel, rows top to bottom with no padding; null where the
/// picture's pixels are the ones this window copied last, which the repaint said (ADR 2168).</param>
internal sealed record Picture(
    uint Width,
    uint Height,
    float OriginX,
    float OriginY,
    uint ShownWidth,
    uint ShownHeight,
    byte[]? Pixels);

/// <summary>
/// What a repaint draws over the pictures, each quadrilateral eight floats — x0, y0 to x3, y3 — in
/// device pixels of the viewport with y downwards, as quorra_quads_get hands them over (ADR 2104).
/// </summary>
/// <param name="Selection">The selection, the find bar's current occurrence among it.</param>
/// <param name="Matches">Every occurrence of the find bar's needle on the page shown.</param>
/// <param name="Highlights">Annex O's highlighted rectangles.</param>
/// <param name="Focus">§12.5.1's focused annotation, or null where nothing holds the focus.</param>
/// <param name="Traced">§12.9's traced path while measuring, flat x, y pairs; empty while the mode is off.</param>
internal sealed record Marks(
    IReadOnlyList<float[]> Selection,
    IReadOnlyList<float[]> Matches,
    IReadOnlyList<float[]> Highlights,
    float[]? Focus,
    float[] Traced);

/// <summary>One widget of a field on the page shown, which this window places a control over.</summary>
/// <param name="Field">§12.7.4.2's qualified name, which every commit addresses.</param>
/// <param name="Shown">§14.9.3's name for a user interface.</param>
/// <param name="Kind">A QUORRA_CONTROL_*.</param>
/// <param name="Flags">A word of QUORRA_FIELD_* bits.</param>
/// <param name="Value">The field's text value, or null where it has none at all.</param>
/// <param name="Displayed">The value through Table 199's /F, which a control without the keyboard shows.</param>
/// <param name="MaxLength">Table 232's /MaxLen, zero where the field states none.</param>
/// <param name="Cells">Table 231 bit 25's combs, zero where the field is not divided into any.</param>
/// <param name="Options">Table 234's /Opt labels, in /Opt's own order.</param>
/// <param name="Selected">Whether each option is chosen.</param>
/// <param name="Number">The widget annotation's object number.</param>
/// <param name="Generation">Its generation.</param>
/// <param name="Quad">Where the widget is, as a mark's quadrilateral is.</param>
/// <param name="On">Whether a check box's or radio button's widget is in its on state.</param>
internal sealed record Placed(
    string Field,
    string Shown,
    uint Kind,
    uint Flags,
    string? Value,
    string? Displayed,
    uint MaxLength,
    uint Cells,
    IReadOnlyList<string> Options,
    IReadOnlyList<bool> Selected,
    uint Number,
    ushort Generation,
    float[] Quad,
    bool On);

/// <summary>One of §12.5.6.14's open popup windows, which this window draws as its own furniture.</summary>
/// <param name="Number">The popup annotation's object number, which closes it when activated.</param>
/// <param name="Generation">Its generation.</param>
/// <param name="Quad">Where the window is, as a mark's quadrilateral is.</param>
/// <param name="Title">§12.5.6.2's /T, "displayed in the title bar".</param>
/// <param name="Contents">Table 166's /Contents.</param>
/// <param name="Modified">Table 166's /M, as the file spells it.</param>
/// <param name="Colour">Table 166's /C as three DeviceRGB components, or null where none is stated.</param>
/// <param name="Rich">Whether Table 172's /RC states a rich note, which this window does not draw yet.</param>
internal sealed record Note(
    uint Number,
    ushort Generation,
    float[] Quad,
    string Title,
    string Contents,
    string Modified,
    float[]? Colour,
    bool Rich);

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

    /// <summary>
    /// Control and the wheel: <paramref name="travel"/> turned away from the person, in the
    /// QUORRA_WHEEL_* <paramref name="unit"/>, about the pointer in device pixels of the viewport;
    /// the steps taken, positive in (ADR 2152).
    /// </summary>
    internal int ZoomWheel(float travel, uint unit, float x, float y)
    {
        int steps = 0;
        Library.Check(
            "quorra_window_zoom_wheel",
            Native.quorra_window_zoom_wheel(handle, travel, unit, x, y, &steps));
        return steps;
    }

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
                uint changed = 0;
                var fresh = new List<bool>();
                if (kind == Native.QUORRA_OWED_REPAINT)
                {
                    Library.Check("quorra_owed_repaint", Native.quorra_owed_repaint(owed, index, &changed));
                    // One answer per picture shown, QUORRA_OUT_OF_RANGE past the last.
                    for (nuint frame = 0; ; frame++)
                    {
                        byte isFresh = 0;
                        int status = Native.quorra_owed_repaint_fresh(owed, index, frame, &isFresh);
                        if (status == Native.QUORRA_OUT_OF_RANGE)
                        {
                            break;
                        }
                        Library.Check("quorra_owed_repaint_fresh", status);
                        fresh.Add(isFresh != 0);
                    }
                }
                records.Add(new Owed(kind, text, document, changed, fresh));
            }
            return records;
        }
        finally
        {
            Native.quorra_owed_free(owed);
        }
    }

    /// <summary>The pictures the page area shows: the viewer's frames and the stand-ins.</summary>
    /// <remarks>
    /// Only the pixels of a picture <paramref name="fresh"/> calls new are copied; a picture past
    /// its list, which a caller holding no record has, is copied whole.
    /// </remarks>
    internal List<Picture> Pictures(IReadOnlyList<bool> fresh)
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
            byte[]? pixels = null;
            if (frame >= (nuint)fresh.Count || fresh[(int)frame])
            {
                pixels = new byte[(int)info.bytes];
                nuint written = 0;
                fixed (byte* into = pixels)
                {
                    Library.Check(
                        "quorra_window_frame_copy",
                        Native.quorra_window_frame_copy(handle, frame, into, info.bytes, &written));
                }
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

    /// <summary>The marks over the page shown, read through the handles the window hands over.</summary>
    /// <remarks>QUORRA_NO_ANSWER, a window with no document in front yet, is no marks.</remarks>
    internal Marks Marks()
    {
        nint selection = 0;
        int selected = Native.quorra_window_selection_quads(handle, &selection);
        nint highlight = 0;
        int highlighted = Native.quorra_window_highlight_quads(handle, &highlight);
        var corners = new float[8];
        int focused;
        fixed (float* quad = corners)
        {
            focused = Native.quorra_window_focused_annotation(handle, null, null, quad);
        }
        if (focused != Native.QUORRA_NO_ANSWER)
        {
            Library.Check("quorra_window_focused_annotation", focused);
        }
        return new Marks(
            Shapes("quorra_window_selection_quads", selected, selection),
            Occurrences(),
            Shapes("quorra_window_highlight_quads", highlighted, highlight),
            focused == Native.QUORRA_OK ? corners : null,
            Traced());
    }

    /// <summary>
    /// Each placed control's size, four a control in logical pixels — the width and height it was
    /// asked to occupy, then the least it can be — and the magnification at which every one fits,
    /// or zero where they fit now (ADR 2169).
    /// </summary>
    internal float ControlsMeasured(IReadOnlyList<int> sizes)
    {
        var values = new int[sizes.Count];
        for (int index = 0; index < values.Length; index++)
        {
            values[index] = sizes[index];
        }
        float magnification = 0;
        fixed (int* first = values)
        {
            Library.Check(
                "quorra_window_controls_measured",
                Native.quorra_window_controls_measured(handle, values.Length == 0 ? null : first, (nuint)(values.Length / 4), &magnification));
        }
        return magnification;
    }

    /// <summary>Activates an object: here, a popup window's, which closes it (quorra.h).</summary>
    internal void Activate(uint number, ushort generation) =>
        Library.Check("quorra_window_activate", Native.quorra_window_activate(handle, number, generation));

    /// <summary>§12.5.6.14's open popup windows on the page shown, in /Annots order.</summary>
    internal List<Note> Popups()
    {
        nint taken = 0;
        int status = Native.quorra_window_popups_read(handle, &taken);
        var notes = new List<Note>();
        if (status == Native.QUORRA_NO_ANSWER)
        {
            return notes;
        }
        Library.Check("quorra_window_popups_read", status);
        nint popups = taken;
        try
        {
            nuint count = Native.quorra_popups_len(popups);
            for (nuint index = 0; index < count; index++)
            {
                nuint at = index;
                uint number = 0;
                ushort generation = 0;
                byte hasParent = 0;
                uint parentNumber = 0;
                ushort parentGeneration = 0;
                Library.Check(
                    "quorra_popup_object",
                    Native.quorra_popup_object(popups, at, &number, &generation, &hasParent, &parentNumber, &parentGeneration));
                var quad = new float[8];
                fixed (float* into = quad)
                {
                    Library.Check("quorra_popup_quad", Native.quorra_popup_quad(popups, at, into));
                }
                var colour = new float[3];
                int coloured;
                fixed (float* into = colour)
                {
                    coloured = Native.quorra_popup_colour(popups, at, into);
                }
                if (coloured != Native.QUORRA_NO_ANSWER)
                {
                    Library.Check("quorra_popup_colour", coloured);
                }
                nuint paragraphs = 0;
                nuint unapplied = 0;
                int rich = Native.quorra_popup_rich(popups, at, 0, &paragraphs, &unapplied);
                if (rich != Native.QUORRA_NO_ANSWER)
                {
                    Library.Check("quorra_popup_rich", rich);
                }
                notes.Add(new Note(
                    number,
                    generation,
                    quad,
                    NoteText(popups, at, Native.QUORRA_NOTE_TITLE),
                    NoteText(popups, at, Native.QUORRA_NOTE_CONTENTS),
                    NoteText(popups, at, Native.QUORRA_NOTE_MODIFIED),
                    coloured == Native.QUORRA_OK ? colour : null,
                    rich == Native.QUORRA_OK));
            }
            return notes;
        }
        finally
        {
            Native.quorra_popups_free(popups);
        }
    }

    // One of a popup's three strings, as quorra_popup_text answers it.
    private static string NoteText(nint popups, nuint index, uint which) =>
        Library.ReadText(
            "quorra_popup_text",
            (into, cap, needed) => Native.quorra_popup_text(popups, index, which, into, cap, needed));

    // §12.9's traced path, in the two-call idiom over floats.
    private float[] Traced()
    {
        nuint needed = 0;
        // The sizing call answers QUORRA_BUFFER_TOO_SMALL wherever there is a point to copy.
        int sized = Native.quorra_window_traced_path(handle, null, 0, &needed);
        if (sized != Native.QUORRA_BUFFER_TOO_SMALL)
        {
            Library.Check("quorra_window_traced_path", sized);
        }
        var points = new float[checked((int)needed)];
        if (points.Length == 0)
        {
            return points;
        }
        fixed (float* into = points)
        {
            Library.Check(
                "quorra_window_traced_path",
                Native.quorra_window_traced_path(handle, into, needed, &needed));
        }
        return points;
    }

    /// <summary>The fields with a widget on the page shown, one record for each widget.</summary>
    /// <remarks>
    /// A walk of §12.7.4.1's field tree, so it is read when the controls are built or moved and
    /// after a commit, never on a pointer's move (quorra.h).
    /// </remarks>
    internal List<Placed> Controls()
    {
        nint taken = 0;
        int status = Native.quorra_window_fields_read(handle, &taken);
        var placed = new List<Placed>();
        if (status == Native.QUORRA_NO_ANSWER)
        {
            return placed;
        }
        Library.Check("quorra_window_fields_read", status);
        nint fields = taken;
        try
        {
            nuint count = Native.quorra_fields_len(fields);
            for (nuint index = 0; index < count; index++)
            {
                nuint field = index;
                string name = Library.ReadText(
                    "quorra_field_name",
                    (into, cap, needed) => Native.quorra_field_name(fields, field, Native.QUORRA_TEXT_QUALIFIED, into, cap, needed));
                string shown = Library.ReadText(
                    "quorra_field_name",
                    (into, cap, needed) => Native.quorra_field_name(fields, field, Native.QUORRA_TEXT_SHOWN, into, cap, needed));
                uint kind = 0;
                uint flags = 0;
                Library.Check("quorra_field_control", Native.quorra_field_control(fields, field, &kind, &flags));
                uint maxLength = 0;
                uint cells = 0;
                Library.Check("quorra_field_limits", Native.quorra_field_limits(fields, field, &maxLength, &cells));
                string? value = Library.ReadTextOrNone(
                    "quorra_field_value",
                    (into, cap, needed) => Native.quorra_field_value(fields, field, into, cap, needed));
                string? displayed = Library.ReadTextOrNone(
                    "quorra_field_displayed",
                    (into, cap, needed) => Native.quorra_field_displayed(fields, field, into, cap, needed));
                (List<string> options, List<bool> chosen) = Options(fields, field);
                nuint widgets = 0;
                Library.Check("quorra_field_widget_count", Native.quorra_field_widget_count(fields, field, &widgets));
                for (nuint widget = 0; widget < widgets; widget++)
                {
                    uint number = 0;
                    ushort generation = 0;
                    byte on = 0;
                    var quad = new float[8];
                    fixed (float* into = quad)
                    {
                        Library.Check(
                            "quorra_field_widget",
                            Native.quorra_field_widget(fields, field, widget, &number, &generation, into, &on));
                    }
                    placed.Add(new Placed(
                        name, shown, kind, flags, value, displayed, maxLength, cells, options, chosen,
                        number, generation, quad, on != 0));
                }
            }
            return placed;
        }
        finally
        {
            Native.quorra_fields_free(fields);
        }
    }

    /// <summary>A placed control's text, as typed.</summary>
    internal void SetFieldText(string field, string text) =>
        Library.Check("quorra_window_set_field_text", Native.quorra_window_set_field_text(handle, field, text));

    /// <summary>A placed control finished with: Enter in a single-line field, or the keyboard taken elsewhere.</summary>
    internal void CommitField(string field) =>
        Library.Check("quorra_window_commit_field", Native.quorra_window_commit_field(handle, field));

    /// <summary>A check box's or radio button's widget clicked towards <paramref name="on"/>.</summary>
    internal void ToggleField(uint number, ushort generation, bool on) =>
        Library.Check("quorra_window_toggle_field", Native.quorra_window_toggle_field(handle, number, generation, on));

    /// <summary>Which of a choice field's options are chosen, by their index in /Opt.</summary>
    internal void SetFieldOptions(string field, IReadOnlyList<nuint> chosen)
    {
        var options = new nuint[chosen.Count];
        for (int index = 0; index < options.Length; index++)
        {
            options[index] = chosen[index];
        }
        fixed (nuint* first = options)
        {
            Library.Check(
                "quorra_window_set_field_options",
                Native.quorra_window_set_field_options(handle, field, options.Length == 0 ? null : first, (nuint)options.Length));
        }
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

    // A handle of quadrilaterals copied out and freed; QUORRA_NO_ANSWER is a window with no
    // document in front yet, which has none.
    private static List<float[]> Shapes(string entryPoint, int status, nint quads)
    {
        var shapes = new List<float[]>();
        if (status == Native.QUORRA_NO_ANSWER)
        {
            return shapes;
        }
        Library.Check(entryPoint, status);
        try
        {
            nuint count = Native.quorra_quads_len(quads);
            for (nuint index = 0; index < count; index++)
            {
                var corners = new float[8];
                fixed (float* into = corners)
                {
                    Library.Check("quorra_quads_get", Native.quorra_quads_get(quads, index, into));
                }
                shapes.Add(corners);
            }
            return shapes;
        }
        finally
        {
            Native.quorra_quads_free(quads);
        }
    }

    // Every occurrence of the find bar's needle on the page shown, each occurrence's shapes in one list.
    private List<float[]> Occurrences()
    {
        nint taken = 0;
        int status = Native.quorra_window_find_matches(handle, &taken);
        var shapes = new List<float[]>();
        if (status == Native.QUORRA_NO_ANSWER)
        {
            return shapes;
        }
        Library.Check("quorra_window_find_matches", status);
        nint matches = taken;
        try
        {
            nuint count = Native.quorra_matches_len(matches);
            for (nuint index = 0; index < count; index++)
            {
                nint quads = 0;
                int found = Native.quorra_matches_quads(matches, index, &quads);
                shapes.AddRange(Shapes("quorra_matches_quads", found, quads));
            }
            return shapes;
        }
        finally
        {
            Native.quorra_matches_free(matches);
        }
    }

    // A choice field's /Opt labels and whether each is chosen; none for any other field.
    private static (List<string> Labels, List<bool> Chosen) Options(nint fields, nuint field)
    {
        var labels = new List<string>();
        var chosen = new List<bool>();
        nuint count = 0;
        Library.Check("quorra_field_option_count", Native.quorra_field_option_count(fields, field, &count));
        for (nuint index = 0; index < count; index++)
        {
            nuint option = index;
            labels.Add(Library.ReadText(
                "quorra_field_option",
                (into, cap, needed) => Native.quorra_field_option(fields, field, option, Native.QUORRA_TEXT_LABEL, into, cap, needed)));
            byte selected = 0;
            Library.Check(
                "quorra_field_option_selected",
                Native.quorra_field_option_selected(fields, field, option, &selected));
            chosen.Add(selected != 0);
        }
        return (labels, chosen);
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
