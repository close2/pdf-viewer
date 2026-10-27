// A Windows virtual key as one of quorra.h's QUORRA_KEY_* (RFC 0010 section 5, ADR 2072). Only the
// translation is this window's: what a key does is viewer_host's table, reached through
// quorra_window_press, so this window stays level with the other three by construction.

using Quorra.Interop;
using Windows.System;

namespace Quorra.App;

/// <summary>The keys the shared table binds, as Windows names them.</summary>
internal static class Keys
{
    // The three punctuation keys VirtualKey has no member for, by their Win32 numbers: VK_OEM_PLUS
    // (`=` and, shifted, `+`), VK_OEM_MINUS and VK_OEM_2 (`/` and, shifted, `?`). Their places are
    // the US layout's, a documented choice of this step (ADR 2136); a layout that puts `+` or `/`
    // elsewhere reaches the same keys through the number pad's Add and Divide.
    private const VirtualKey OemPlus = (VirtualKey)0xBB;
    private const VirtualKey OemMinus = (VirtualKey)0xBD;
    private const VirtualKey OemQuestion = (VirtualKey)0xBF;

    /// <summary>The QUORRA_KEY_* <paramref name="key"/> is, or null for one the table does not bind.</summary>
    /// <remarks>
    /// "A letter is the letter, not the keystroke" (quorra.h): a virtual key is already the letter
    /// whatever Shift says, which is the folding the header asks for. "PLUS is whichever key
    /// produces `+`", which is the key VK_OEM_PLUS with Shift held and the number pad's Add.
    /// </remarks>
    internal static uint? Of(VirtualKey key, bool shift) => key switch
    {
        VirtualKey.A => Native.QUORRA_KEY_A,
        VirtualKey.B => Native.QUORRA_KEY_B,
        VirtualKey.C => Native.QUORRA_KEY_C,
        VirtualKey.F => Native.QUORRA_KEY_F,
        VirtualKey.H => Native.QUORRA_KEY_H,
        VirtualKey.K => Native.QUORRA_KEY_K,
        VirtualKey.L => Native.QUORRA_KEY_L,
        VirtualKey.M => Native.QUORRA_KEY_M,
        VirtualKey.O => Native.QUORRA_KEY_O,
        VirtualKey.P => Native.QUORRA_KEY_P,
        VirtualKey.R => Native.QUORRA_KEY_R,
        VirtualKey.S => Native.QUORRA_KEY_S,
        VirtualKey.T => Native.QUORRA_KEY_T,
        VirtualKey.W => Native.QUORRA_KEY_W,
        VirtualKey.Y => Native.QUORRA_KEY_Y,
        VirtualKey.Z => Native.QUORRA_KEY_Z,
        VirtualKey.Number0 or VirtualKey.NumberPad0 => Native.QUORRA_KEY_ZERO,
        VirtualKey.Add => Native.QUORRA_KEY_PLUS,
        OemPlus => shift ? Native.QUORRA_KEY_PLUS : Native.QUORRA_KEY_EQUALS,
        VirtualKey.Subtract or OemMinus => Native.QUORRA_KEY_MINUS,
        VirtualKey.Divide => Native.QUORRA_KEY_SLASH,
        OemQuestion => shift ? Native.QUORRA_KEY_QUESTION : Native.QUORRA_KEY_SLASH,
        VirtualKey.Escape => Native.QUORRA_KEY_ESCAPE,
        VirtualKey.Tab => Native.QUORRA_KEY_TAB,
        VirtualKey.Space => Native.QUORRA_KEY_SPACE,
        VirtualKey.Enter => Native.QUORRA_KEY_ENTER,
        VirtualKey.Home => Native.QUORRA_KEY_HOME,
        VirtualKey.End => Native.QUORRA_KEY_END,
        VirtualKey.Left => Native.QUORRA_KEY_LEFT,
        VirtualKey.Right => Native.QUORRA_KEY_RIGHT,
        VirtualKey.Up => Native.QUORRA_KEY_UP,
        VirtualKey.Down => Native.QUORRA_KEY_DOWN,
        VirtualKey.PageUp => Native.QUORRA_KEY_PAGE_UP,
        VirtualKey.PageDown => Native.QUORRA_KEY_PAGE_DOWN,
        _ => null,
    };
}
