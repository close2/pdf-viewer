// Owned copies of the dialogue records: their strings cannot outlive quorra_owed on the C side.
using System;
using System.Collections.Generic;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>A script's alert or response, copied before its owed batch is freed.</summary>
internal sealed record ScriptQuestion(uint Kind, uint Icon, uint Buttons, uint Dismissed,
    bool Password, string Message, string? Title, string Default, string? Label);

/// <summary>The archive chooser's starting indices and the library's choices and explanations.</summary>
internal sealed record ArchiveChoice(int Target, int Profile, IReadOnlyList<string> Targets,
    IReadOnlyList<string> Profiles, IReadOnlyList<string> Sentences);

/// <summary>The print preview the library admitted, including its quality restriction.</summary>
internal sealed record PrintGrant(nuint Pages, bool Degraded);

internal sealed unsafe partial class QuorraWindow
{
    private static ScriptQuestion ReadScript(nint owed, nuint at)
    {
        uint kind = 0, icon = 0, buttons = 0, dismissed = 0;
        byte password = 0;
        Library.Check("quorra_owed_script_question",
            Native.quorra_owed_script_question(owed, at, &kind, &icon, &buttons, &dismissed, &password));
        string? Text(uint part) => Library.ReadTextOrNone("quorra_owed_script_text",
            (into, cap, needed) => Native.quorra_owed_script_text(owed, at, part, into, cap, needed));
        return new ScriptQuestion(kind, icon, buttons, dismissed, password != 0,
            Text(Native.QUORRA_SCRIPT_TEXT_MESSAGE) ?? string.Empty,
            Text(Native.QUORRA_SCRIPT_TEXT_TITLE),
            kind == Native.QUORRA_SCRIPT_QUESTION_RESPONSE
                ? Text(Native.QUORRA_SCRIPT_TEXT_DEFAULT) ?? string.Empty : string.Empty,
            kind == Native.QUORRA_SCRIPT_QUESTION_RESPONSE ? Text(Native.QUORRA_SCRIPT_TEXT_LABEL) : null);
    }

    private static ArchiveChoice ReadArchive(nint owed, nuint at)
    {
        nuint target = 0, profile = 0, targets = 0, profiles = 0;
        Library.Check("quorra_owed_archive", Native.quorra_owed_archive(owed, at, &target, &profile));
        Library.Check("quorra_archive_targets", Native.quorra_archive_targets(&targets));
        Library.Check("quorra_archive_profiles", Native.quorra_archive_profiles(&profiles));
        var targetNames = new List<string>();
        var profileNames = new List<string>();
        var sentences = new List<string>();
        for (nuint index = 0; index < targets; index++)
        {
            nuint row = index;
            targetNames.Add(Library.ReadText("quorra_archive_target",
                (into, cap, needed) => Native.quorra_archive_target(row, into, cap, needed)));
        }
        for (nuint index = 0; index < profiles; index++)
        {
            nuint row = index;
            profileNames.Add(Library.ReadText("quorra_archive_profile",
                (into, cap, needed) => Native.quorra_archive_profile(row, into, cap, needed)));
            sentences.Add(Library.ReadText("quorra_archive_profile_sentence",
                (into, cap, needed) => Native.quorra_archive_profile_sentence(row, into, cap, needed)));
        }
        return new ArchiveChoice(checked((int)target), checked((int)profile), targetNames, profileNames, sentences);
    }

    private static PrintGrant ReadPrint(nint owed, nuint at)
    {
        nuint pages = 0;
        byte degraded = 0;
        Library.Check("quorra_owed_print", Native.quorra_owed_print(owed, at, &pages, &degraded));
        return new PrintGrant(pages, degraded != 0);
    }

    /// <summary>A script answer carrying the actual pressed button or typed response.</summary>
    internal void AnswerScript(ulong document, uint how, uint button, string? text) =>
        Library.Check("quorra_window_answer_script",
            Native.quorra_window_answer_script(handle, document, how, button, text));
}
