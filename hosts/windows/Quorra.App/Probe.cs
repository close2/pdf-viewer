// The window's own account of what it did for §12.2's Table 147 and Table 29's FullScreen, written
// to its log under --probe for CI's smoke step (ADRs 2436, 2446, 2478). Every sentence but the
// two about a resize asked and waited for states what the window reads back from its own
// AppWindow and page area, never what it was asked, so a step that holds these sentences to the
// table holds the window rather than the library, whose half drive_a_window.c already proves (ADR
// 2308). The measured pair and the client size read after a resize are said so that a failing run
// says which of the two sizes moved first.

using System;
using Microsoft.UI.Windowing;

namespace Quorra.App;

/// <summary>What a placement read back: the page area, the outer window and its work area, in logical pixels.</summary>
/// <param name="AreaWidth">The page area's width.</param>
/// <param name="AreaHeight">The page area's height.</param>
/// <param name="Width">The window's outer width, its frame included.</param>
/// <param name="Height">The window's outer height.</param>
/// <param name="X">The window's outer left edge.</param>
/// <param name="Y">The window's outer top edge.</param>
/// <param name="Work">The work area of the display the window is on: left, top, width, height.</param>
internal readonly record struct ReadBack(
    int AreaWidth, int AreaHeight, int Width, int Height, int X, int Y, (int, int, int, int) Work);

/// <summary>
/// The window's inner size and its page area's, in logical pixels, from the one layout pass whose
/// page area size the library was last told (ADR 2478).
/// </summary>
/// <param name="Width">The inner width: the window's content, everything inside its frame.</param>
/// <param name="Height">The inner height.</param>
/// <param name="AreaWidth">The page area's width, the viewport the library was told.</param>
/// <param name="AreaHeight">The page area's height.</param>
internal readonly record struct Measure(int Width, int Height, int AreaWidth, int AreaHeight);

/// <summary>The --probe word's sentences, each written once per change of what it states.</summary>
internal sealed class WindowProbe
{
    /// <summary>The command-line word that turns the probe on; it is the window's, not a reader's setting.</summary>
    internal const string Word = "--probe";

    private readonly Action<string> log;
    private AppWindowPresenterKind? presenter;
    private string? pane;
    private bool escaped;
    private bool waiting;
    private bool empty;

    /// <summary>A probe writing to <paramref name="log"/>, the window's own log.</summary>
    internal WindowProbe(Action<string> log) => this.log = log;

    /// <summary>The library asked for the window's inner size to change: Table 147's /FitWindow at work.</summary>
    internal void AskedToResize(int width, int height)
    {
        waiting = false;
        log($"probe: asked to resize the window to {width} by {height}");
    }

    /// <summary>
    /// A placement was owed before the window's last resize reached the page area, so it waits
    /// for that size: said once per resize (ADR 2446).
    /// </summary>
    internal void Waiting()
    {
        if (waiting) return;
        waiting = true;
        log("probe: the placement waits for the page area to take the resize");
    }

    /// <summary>
    /// What a placement is answered from: the pair the library's viewport was told with, beside the
    /// client size AppWindow reads at the moment of asking (ADR 2478).
    /// </summary>
    internal void Measured(Measure told, int clientWidth, int clientHeight)
    {
        log($"probe: measured: the window {told.Width} by {told.Height} around the page area " +
            $"{told.AreaWidth} by {told.AreaHeight} last told, the client area {clientWidth} by {clientHeight} now");
    }

    /// <summary>The client size AppWindow reads straight after a resize was carried out.</summary>
    internal void ClientAfterResize(int width, int height)
    {
        log($"probe: the client area reads {width} by {height} after the resize");
    }

    /// <summary>
    /// The page area was laid out with no extent, so the library is told no size and owes no frame
    /// for it: said once until a size is told again.
    /// </summary>
    internal void Empty(uint width, uint height)
    {
        if (empty) return;
        empty = true;
        log($"probe: the page area is {width} by {height} device pixels, so no size is told");
    }

    /// <summary>A size was told to the library, so a later empty page area is said again.</summary>
    internal void Told() => empty = false;

    /// <summary>The library's last answer for the placement was carried out, and this is where the window is.</summary>
    internal void Placed(ReadBack at)
    {
        (int left, int top, int width, int height) = at.Work;
        log($"probe: placed: the page area {at.AreaWidth} by {at.AreaHeight}, the window {at.Width} by {at.Height} " +
            $"at {at.X}, {at.Y}, on the work area {left}, {top}, {width} by {height}");
    }

    /// <summary>The presenter the window has after <c>Present</c>, said where it changed.</summary>
    internal void Presenting(AppWindowPresenterKind kind)
    {
        if (presenter == kind) return;
        presenter = kind;
        log($"probe: the presenter is {kind}");
    }

    /// <summary>The navigation pane after a chrome record, said where it changed.</summary>
    internal void Pane(bool open, string? label)
    {
        string now = open ? $"open on {label ?? "no panel"}" : "closed";
        if (pane == now) return;
        pane = now;
        log($"probe: the pane is {now}");
    }

    /// <summary>
    /// Whether the window should now leave full screen as Escape leaves it: once, at the first
    /// page drawn while it presents, so Table 147's /NonFullScreenPageMode has an exit to answer.
    /// </summary>
    internal bool LeaveNow(bool fullScreen)
    {
        if (escaped || !fullScreen) return false;
        escaped = true;
        log("probe: a page was drawn full screen, so Escape is pressed as the key presses it");
        return true;
    }
}
