// What every call into viewer_ffi.dll shares: a refusal as an exception with the library's own
// sentence, C's two-call idiom for a string, and the check a caller makes once at startup that the
// library is the one Quorra.Interop was declared against (RFC 0010 section 5, ADR 2136).

using System;
using System.Runtime.InteropServices;
using System.Text;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>An entry point refused: its name, and the library's sentence for the status.</summary>
/// <remarks>
/// Every status the library returns is checked, and a refusal becomes this exception. It is caught
/// in one place, MainWindow's <c>Guard</c>, which says it on the status line, so no status is
/// dropped and no exception leaves an event handler.
/// </remarks>
internal sealed class QuorraException : Exception
{
    /// <summary>A refusal in the library's words.</summary>
    internal QuorraException(string entryPoint, int status)
        : base($"{entryPoint}: {Library.StatusSentence(status)} ({status})")
    {
    }

    /// <summary>A refusal this window makes of what the library handed it.</summary>
    internal QuorraException(string sentence)
        : base(sentence)
    {
    }
}

/// <summary>One string-valued entry point, partly applied: the buffer, its size, the size needed.</summary>
internal unsafe delegate int TwoCall(byte* into, nuint cap, nuint* needed);

/// <summary>The calls every part of this window makes the same way.</summary>
internal static unsafe class Library
{
    /// <summary>Throws the refusal where <paramref name="status"/> is not QUORRA_OK.</summary>
    internal static void Check(string entryPoint, int status)
    {
        if (status != Native.QUORRA_OK)
        {
            throw new QuorraException(entryPoint, status);
        }
    }

    /// <summary>The library's sentence for a status; static on its side, so never freed here.</summary>
    internal static string StatusSentence(int status) =>
        Marshal.PtrToStringUTF8((nint)Native.quorra_status_message(status)) ?? "no sentence";

    /// <summary>The library's name for a QUORRA_OWED_* kind, "unknown" for one it does not define.</summary>
    internal static string OwedName(uint kind) => Name(Native.quorra_owed_kind_name(kind));

    /// <summary>The library's name for a QUORRA_ACT_*, "unknown" for one it does not define.</summary>
    internal static string ActName(uint act) => Name(Native.quorra_act_name(act));

    // A static string the library answers; never freed here.
    private static string Name(byte* name) => Marshal.PtrToStringUTF8((nint)name) ?? "unknown";

    /// <summary>
    /// C's two-call idiom: the size first, then the text into a buffer of exactly that size.
    /// </summary>
    /// <remarks>
    /// <c>needed</c> counts the terminating NUL, and nothing is written unless the whole string
    /// fits (quorra.h; <c>copy_out</c> in abi.rs), so the second call cannot truncate.
    /// </remarks>
    internal static string ReadText(string entryPoint, TwoCall call)
    {
        nuint needed = 0;
        int sized = call(null, 0, &needed);
        if (sized != Native.QUORRA_BUFFER_TOO_SMALL)
        {
            throw new QuorraException(entryPoint, sized);
        }
        if (needed > int.MaxValue)
        {
            throw new QuorraException($"{entryPoint}: a string of {needed} bytes is past what .NET holds");
        }
        byte[] text = new byte[(int)needed];
        nuint written = 0;
        fixed (byte* into = text)
        {
            Check(entryPoint, call(into, needed, &written));
        }
        return Encoding.UTF8.GetString(text, 0, text.Length - 1);
    }

    /// <summary>
    /// Why this library cannot serve this window, or null where it can: quorra_abi_check, with the
    /// ABI's revision and the event kinds the header states, made once before anything else.
    /// </summary>
    /// <remarks>
    /// A missing viewer_ffi.dll, or one older than these declarations, is the runtime's exception
    /// at the first call; both are caught here, the one place they can first arise, and become a
    /// sentence, because a window that opens and says why it is empty is the loud failure and a
    /// process that ends at launch is not.
    /// </remarks>
    internal static string? Refusal()
    {
        try
        {
            int agreed = Native.quorra_abi_check(Native.QUORRA_ABI_VERSION, Native.QUORRA_EVENT_KIND_COUNT);
            if (agreed != Native.QUORRA_OK)
            {
                return "viewer_ffi.dll is not the library this window was built against: "
                    + $"its ABI is revision {Native.quorra_abi_version()} with "
                    + $"{Native.quorra_event_kind_count()} event kinds, and this window's is revision "
                    + $"{Native.QUORRA_ABI_VERSION} with {Native.QUORRA_EVENT_KIND_COUNT} "
                    + $"({StatusSentence(agreed)})";
            }
            return null;
        }
        catch (DllNotFoundException)
        {
            return "viewer_ffi.dll is not beside Quorra.App.exe: build it with "
                + "`cargo build --release -p viewer-ffi` and build this window again";
        }
        catch (EntryPointNotFoundException missing)
        {
            return $"viewer_ffi.dll is older than this window's declarations: {missing.Message}";
        }
    }

    /// <summary>
    /// What the library counts differently from this window's declarations, or null where the
    /// counts agree. Said, not refused: none of the three is part of quorra_abi_check.
    /// </summary>
    /// <remarks>
    /// The header asks a caller to compare the owed kinds beside the ABI check and to name a kind
    /// with no arm rather than drop it, which MainWindow does; a key this window does not
    /// translate is one it never passes, and an act it has no arm for is said by name.
    /// </remarks>
    internal static string? Differences()
    {
        if (Native.quorra_owed_kind_count() == Native.QUORRA_OWED_KIND_COUNT
            && Native.quorra_key_count() == Native.QUORRA_KEY_COUNT
            && Native.quorra_act_count() == Native.QUORRA_ACT_COUNT)
        {
            return null;
        }
        return $"viewer_ffi.dll owes {Native.quorra_owed_kind_count()} kinds of record, binds "
            + $"{Native.quorra_key_count()} keys and hands back {Native.quorra_act_count()} acts; "
            + $"this window knows {Native.QUORRA_OWED_KIND_COUNT}, {Native.QUORRA_KEY_COUNT} and "
            + $"{Native.QUORRA_ACT_COUNT}, and names what it has no arm for";
    }
}
