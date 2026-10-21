// The window's loop, as quorra.h states it for a toolkit (RFC 0010 sections 3.2a and 6, step N3a;
// ADRs 2088, 2136, 2137, 2152, 2153, 2168, 2169): one call into quorra_window for each thing that happened to
// the window, then what the window owes taken and acted on in order, then its timer armed at the
// interval it asks for. Nothing calls back into this file; every record arrives after the call
// that made it.

using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Input;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Input;
using Microsoft.UI.Xaml.Media;
using Quorra.Interop;
using Windows.ApplicationModel.DataTransfer;
using Windows.UI.Core;
// One name from Windows.System, whose DispatcherQueueTimer would otherwise collide with the
// Windows App SDK's in Microsoft.UI.Dispatching.
using VirtualKey = Windows.System.VirtualKey;

namespace Quorra.App;

/// <summary>The window: one quorra_window, its page area and its status line.</summary>
public sealed partial class MainWindow : Window
{
    // The distance a wheel notch moves, in logical pixels: the number viewer-gtk and viewer-qt
    // chose, because a wheel is not a fact about a toolkit.
    private const double ScrollStep = 48.0;

    // One notch, in the units of WinUI's MouseWheelDelta.
    private const double Notch = 120.0;

    // How many batches one settle takes before it says the window never stopped owing: an answer
    // can owe a record and a record an answer, and a loop that never ends would hang the window.
    private const int SettleBound = 64;

    private readonly DispatcherQueueTimer timer;
    private readonly PageControls controls;
    private readonly PageNotes notes;
    private readonly List<string> beside = new();
    private readonly HashSet<uint> named = new();
    private QuorraWindow? window;
    private double scale;
    private bool sized;
    private bool pressed;
    private bool presentOwed;
    // Where the pictures were when the controls were last placed: a control moves with the page,
    // and reading the field tree again on a repaint that moved nothing is a walk for nothing.
    private string? placedAt;

    /// <summary>A window on the documents <paramref name="arguments"/> name, under the words it states.</summary>
    internal MainWindow(IReadOnlyList<string> arguments)
    {
        InitializeComponent();
        Title = "quorra";
        controls = new PageControls(
            Area.ControlLayer,
            new Commits(
                (field, text) => Commit("a field typed into", open => open.SetFieldText(field.Field, text)),
                field => Commit("a field finished with", open => open.CommitField(field.Field)),
                field => Commit("a button clicked", open => open.ToggleField(field.Number, field.Generation, !field.On)),
                (field, chosen) => Commit("an option chosen", open => open.SetFieldOptions(field.Field, chosen))),
            () => Area.Focus(FocusState.Programmatic));
        notes = new PageNotes(
            Area.NoteLayer,
            note => Commit("a popup closed", open => open.Activate(note.Number, note.Generation)),
            Log);
        timer = DispatcherQueue.CreateTimer();
        timer.IsRepeating = false;
        timer.Tick += (_, _) => Guard("the timer", () =>
        {
            window?.Tick();
            Settle();
        });
        Area.Loaded += (_, _) =>
        {
            Area.Focus(FocusState.Programmatic);
            // A move to a monitor of another scale changes the device pixels and not the size, and
            // a size change, which SizeChanged has already told the window, is not one.
            Area.XamlRoot.Changed += (root, _) =>
            {
                if (root.RasterizationScale != scale)
                {
                    Guard("a change of display", Resize);
                }
            };
        };
        Closed += (_, _) =>
        {
            timer.Stop();
            CompositionTarget.Rendered -= OnRendered;
            window?.Dispose();
            window = null;
        };
        Guard("the open", () => Open(arguments));
    }

    // The library checked, the words read, the first document opened; the rest open beside it
    // once the first resize has joined the first, as drive_a_window.c opens its second.
    private void Open(IReadOnlyList<string> arguments)
    {
        string? refusal = Library.Refusal();
        if (refusal is not null)
        {
            Say(refusal);
            return;
        }
        string? differences = Library.Differences();
        if (differences is not null)
        {
            Log(differences);
        }
        using QuorraSettings settings = QuorraSettings.Create();
        var paths = new List<string>();
        foreach (string argument in arguments)
        {
            if (!argument.StartsWith("--", StringComparison.Ordinal))
            {
                paths.Add(argument);
                continue;
            }
            string? refused = settings.Take(argument);
            if (refused is not null)
            {
                Say(refused);
                return;
            }
        }
        if (paths.Count == 0)
        {
            Say("no document: Quorra.App.exe <file.pdf> [<file.pdf> ...]; a file picker is RFC 0010's step N4");
            return;
        }
        window = QuorraWindow.Open(paths[0], settings);
        beside.AddRange(paths.GetRange(1, paths.Count - 1));
        Settle();
    }

    private void OnPageSizeChanged(object sender, SizeChangedEventArgs e) => Guard("a resize", Resize);

    private void Resize()
    {
        if (window is null || Area.XamlRoot is null)
        {
            return;
        }
        double now = Area.XamlRoot.RasterizationScale;
        uint width = (uint)Math.Round(Area.ActualWidth * now);
        uint height = (uint)Math.Round(Area.ActualHeight * now);
        // A minimised window has no viewport, and quorra_window_resize refuses an empty one.
        if (width == 0 || height == 0)
        {
            return;
        }
        scale = now;
        window.Resize(width, height, (float)now);
        if (!sized)
        {
            sized = true;
            foreach (string path in beside)
            {
                window.OpenBeside(path);
            }
            beside.Clear();
        }
        Settle();
    }

    // What the window owes, until it owes nothing, then its timer armed or stopped.
    private void Settle()
    {
        for (int batch = 0; batch < SettleBound; batch++)
        {
            if (window is null)
            {
                return;
            }
            List<Owed> owed = window.TakeOwed();
            if (owed.Count == 0)
            {
                timer.Stop();
                int interval = window.Interval;
                if (interval >= 0)
                {
                    timer.Interval = TimeSpan.FromMilliseconds(interval);
                    timer.Start();
                }
                return;
            }
            foreach (Owed record in owed)
            {
                Act(record);
            }
        }
        Say($"the window was still owing records after {SettleBound} batches, so this window stopped taking them");
    }

    private void Act(Owed owed)
    {
        if (window is not QuorraWindow open)
        {
            return;
        }
        switch (owed.Kind)
        {
            case Native.QUORRA_OWED_SAID:
                Say(owed.Text);
                break;
            case Native.QUORRA_OWED_TITLE:
                Title = owed.Text;
                break;
            case Native.QUORRA_OWED_REPAINT:
                Repaint(open, owed);
                break;
            case Native.QUORRA_OWED_CONTROLS:
                controls.Rebuild(open.Controls(), scale);
                placedAt = null;
                Measured(open);
                break;
            case Native.QUORRA_OWED_POPUPS:
                notes.Show(open.Popups(), scale);
                break;
            case Native.QUORRA_OWED_CLIPBOARD:
                Copy(owed.Text);
                break;
            // Four records wait for an answer this step has no dialogue to ask for. Each is
            // declined in the library's own terms for declining, and the decline is said, so
            // nothing waits on a card nobody sees (ADR 2137).
            case Native.QUORRA_OWED_PASSWORD:
                Say($"{owed.Text}: this window has no password prompt yet, so the document stays closed");
                open.Password(owed.Document, null);
                break;
            case Native.QUORRA_OWED_QUESTION:
                Say($"{owed.Text} (declined: this window has no dialogue for the question yet)");
                open.Answer(false);
                break;
            case Native.QUORRA_OWED_ARCHIVE:
                Say($"{owed.Text} (closed with nothing chosen: this window has no chooser yet)");
                open.Archive(-1, -1);
                break;
            case Native.QUORRA_OWED_SCRIPT:
                Say($"{owed.Text}: a script asked, and this window has no dialogue to answer it with");
                open.AnswerScript(owed.Document, Native.QUORRA_SCRIPT_ANSWER_UNANSWERABLE);
                break;
            case Native.QUORRA_OWED_PRINT:
                Say($"{owed.Text} (this window has no printer yet, so the print is over)");
                open.PrintFinished();
                break;
            // No dialogue was opened for the script's question, so there is none to close.
            case Native.QUORRA_OWED_SCRIPT_WITHDRAWN:
                break;
            // The panels, the strip, the chrome, a placement and the system sound are the next
            // steps' (RFC 0010 step N3a). Each is named in the log the first time it is owed rather
            // than dropped.
            default:
                if (named.Add(owed.Kind))
                {
                    Log($"owed {Library.OwedName(owed.Kind)}, which this window does not show yet");
                }
                break;
        }
    }

    // What the record says changed and nothing else: the pictures read where they or their places
    // did and copied where they are new, then the marks over them where those moved (quorra.h), and
    // the controls moved where the page did (ADR 2168).
    private void Repaint(QuorraWindow open, Owed owed)
    {
        if (scale <= 0)
        {
            return;
        }
        if ((owed.Changed & (Native.QUORRA_REPAINT_FRAMES | Native.QUORRA_REPAINT_PLACES)) != 0)
        {
            List<Picture> pictures = open.Pictures(owed.Fresh);
            Area.Show(pictures, scale);
            string at = Geometry(pictures, scale);
            if (controls.Count > 0 && at != placedAt && controls.Place(open.Controls(), scale))
            {
                placedAt = at;
                Measured(open);
            }
        }
        if ((owed.Changed & Native.QUORRA_REPAINT_MARKS) != 0)
        {
            Area.ShowMarks(open.Marks(), scale);
        }
        if (!presentOwed)
        {
            presentOwed = true;
            CompositionTarget.Rendered += OnRendered;
        }
    }

    // Each control's size against its rectangle told to the window, which is what W's fit sends; the
    // finding said in the log, as viewer-qt says it in its trace (ADR 2169).
    private void Measured(QuorraWindow open)
    {
        List<int> sizes = controls.Measure();
        float fits = open.ControlsMeasured(sizes);
        if (fits > 0)
        {
            Log($"{sizes.Count / 4} control(s) measured; every one fits its /Rect at {fits:0.000}, which W sends");
        }
    }

    // Where each picture is and how large it is shown, which a control over the page follows.
    private static string Geometry(List<Picture> pictures, double scale)
    {
        var at = new System.Text.StringBuilder();
        at.Append(scale);
        foreach (Picture picture in pictures)
        {
            at.Append(' ').Append(picture.OriginX).Append(',').Append(picture.OriginY)
                .Append(',').Append(picture.ShownWidth).Append(',').Append(picture.ShownHeight);
        }
        return at.ToString();
    }

    // What a person did to a control, carried to the window, and then what it owes: the controls
    // show what the fields took, which a format, a keystroke script or /MaxLen can make differ
    // from what was typed, and a calculation can change another field's.
    private void Commit(string what, Action<QuorraWindow> commit)
    {
        if (window is not QuorraWindow open)
        {
            return;
        }
        Guard(what, () =>
        {
            commit(open);
            Settle();
            if (window is QuorraWindow still)
            {
                controls.Place(still.Controls(), scale);
            }
        });
    }

    // The frame XAML composes after the pictures changed is the one that shows them, so the
    // window is told they are on the screen from there and not from the line that set them.
    private void OnRendered(object? sender, object e)
    {
        CompositionTarget.Rendered -= OnRendered;
        presentOwed = false;
        Guard("a frame shown", () =>
        {
            window?.Presented();
            Settle();
        });
    }

    private void OnPageKeyDown(object sender, KeyRoutedEventArgs e)
    {
        // A key typed into a control over the page is the control's.
        if (!Area.IsPage(e.OriginalSource))
        {
            return;
        }
        bool shift = Held(VirtualKey.Shift);
        bool command = Held(VirtualKey.Control);
        if (window is not QuorraWindow open || Keys.Of(e.Key, shift) is not uint key)
        {
            return;
        }
        e.Handled = true;
        Guard("a key", () =>
        {
            uint act = open.Press(key, shift, command);
            Settle();
            WindowAct(open, act);
        });
    }

    // What only a window can do, after quorra_window_press has done the rest.
    private void WindowAct(QuorraWindow open, uint act)
    {
        switch (act)
        {
            case Native.QUORRA_ACT_NOTHING:
                return;
            case Native.QUORRA_ACT_NEXT_DOCUMENT:
                (nuint count, nuint focused) = open.Strip;
                if (count > 0)
                {
                    open.Choose((focused + 1) % count);
                }
                break;
            case Native.QUORRA_ACT_CLOSE_DOCUMENT:
                if (open.CloseDocument())
                {
                    Close();
                    return;
                }
                break;
            default:
                Say($"{Library.ActName(act)}: not in this window yet (RFC 0010 step N3a)");
                return;
        }
        Settle();
    }

    private void OnPagePointerPressed(object sender, PointerRoutedEventArgs e)
    {
        if (!e.GetCurrentPoint(Area).Properties.IsLeftButtonPressed || !Area.IsPage(e.OriginalSource))
        {
            return;
        }
        Area.Focus(FocusState.Pointer);
        // The capture keeps a drag that leaves the area reporting to it; a capture refused, as
        // another element holding the pointer can refuse it, only shortens what the drag reports.
        Area.CapturePointer(e.Pointer);
        pressed = true;
        Pointer(e, Native.QUORRA_POINTER_PRESSED);
    }

    private void OnPagePointerMoved(object sender, PointerRoutedEventArgs e) =>
        Pointer(e, pressed ? Native.QUORRA_POINTER_DRAGGED : Native.QUORRA_POINTER_MOVED);

    private void OnPagePointerReleased(object sender, PointerRoutedEventArgs e)
    {
        if (!pressed)
        {
            return;
        }
        pressed = false;
        Area.ReleasePointerCapture(e.Pointer);
        Pointer(e, Native.QUORRA_POINTER_RELEASED);
    }

    // The pointer in device pixels of the viewport, as quorra_window_pointer takes it.
    private void Pointer(PointerRoutedEventArgs e, uint action)
    {
        if (window is not QuorraWindow open || scale <= 0)
        {
            return;
        }
        Windows.Foundation.Point at = e.GetCurrentPoint(Area).Position;
        e.Handled = true;
        Guard("the pointer", () =>
        {
            Area.ShowLinkCursor(open.Pointer((float)(at.X * scale), (float)(at.Y * scale), action));
            Settle();
        });
    }

    private void OnPageWheel(object sender, PointerRoutedEventArgs e)
    {
        if (window is not QuorraWindow open || scale <= 0)
        {
            return;
        }
        e.Handled = true;
        PointerPoint point = e.GetCurrentPoint(Area);
        PointerPointProperties wheel = point.Properties;
        // Control and the wheel magnify about the pointer, as in the other three windows; a notch
        // is one line, and the window carries the fraction a high-resolution wheel or a precision
        // touchpad's pinch leaves (ADR 2152).
        if (Held(VirtualKey.Control) && !wheel.IsHorizontalMouseWheel)
        {
            float travel = (float)(wheel.MouseWheelDelta / Notch);
            float x = (float)(point.Position.X * scale);
            float y = (float)(point.Position.Y * scale);
            Guard("Control and the wheel", () =>
            {
                open.ZoomWheel(travel, Native.QUORRA_WHEEL_LINES, x, y);
                Settle();
            });
            return;
        }
        float distance = (float)(wheel.MouseWheelDelta / Notch * ScrollStep * scale);
        Guard("the wheel", () =>
        {
            // A vertical notch away from the person is a positive delta and shows what is above,
            // which is a negative dy (quorra.h: a positive dy moves the content up); a horizontal
            // tilt to the right is a positive delta and shows what is to the right.
            if (wheel.IsHorizontalMouseWheel)
            {
                open.Scroll(distance, 0);
            }
            else
            {
                open.Scroll(0, -distance);
            }
            Settle();
        });
    }

    private static bool Held(VirtualKey key) =>
        InputKeyboardSource.GetKeyStateForCurrentThread(key).HasFlag(CoreVirtualKeyStates.Down);

    private void Copy(string text)
    {
        var package = new DataPackage();
        package.SetText(text);
        try
        {
            Clipboard.SetContent(package);
        }
        // Another program holding the clipboard open is the one failure the platform reports
        // here, and it is said rather than ended on.
        catch (COMException busy)
        {
            Say($"the clipboard refused the copy: {busy.Message}");
        }
    }

    // The one place a refusal from the library is handled: said, and the window carries on.
    private void Guard(string what, Action action)
    {
        try
        {
            action();
        }
        catch (QuorraException refused)
        {
            Say($"{what}: {refused.Message}");
        }
    }

    // The status line, and the log beside it, which is the console as every window's is.
    private void Say(string sentence)
    {
        StatusLine.Text = sentence;
        Log(sentence);
    }

    private static void Log(string sentence) => Console.Error.WriteLine(sentence);
}
