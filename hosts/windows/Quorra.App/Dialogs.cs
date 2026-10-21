// One modal dialogue at a time; the library's timer continues while ShowAsync yields.
using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Quorra.Interop;

namespace Quorra.App;

/// <summary>The cards a window owes, queued without blocking its timer or retaining native data.</summary>
internal sealed class WindowDialogs
{
    private sealed class Pending
    {
        internal readonly Owed Record;
        internal bool Withdrawn;
        internal Pending(Owed record) => Record = record;
    }

    private readonly Func<XamlRoot?> root;
    private readonly Action<string, Action<QuorraWindow>> commit;
    private readonly Action<string> say;
    private readonly Queue<Pending> queue = new();
    private Pending? showing;
    private ContentDialog? dialog;
    private bool running;
    private bool closed;

    internal WindowDialogs(Func<XamlRoot?> root, Action<string, Action<QuorraWindow>> commit, Action<string> say)
    {
        this.root = root;
        this.commit = commit;
        this.say = say;
    }

    internal void Enqueue(Owed record)
    {
        if (closed) return;
        // Each native card kind has one outstanding answer slot. A newer record replaces
        // that slot, so an older visible card must never answer the replacement.
        foreach (Pending pending in queue)
            if (pending.Record.Kind == record.Kind) pending.Withdrawn = true;
        if (showing is Pending active && active.Record.Kind == record.Kind)
        {
            active.Withdrawn = true;
            dialog?.Hide();
        }
        queue.Enqueue(new Pending(record));
        Start();
    }

    /// <summary>A script can withdraw while its dialogue is queued or visible.</summary>
    internal void Withdraw(ulong document)
    {
        foreach (Pending pending in queue)
        {
            if (pending.Record.Kind == Native.QUORRA_OWED_SCRIPT && pending.Record.Document == document)
                pending.Withdrawn = true;
        }
        if (showing is Pending active && active.Record.Kind == Native.QUORRA_OWED_SCRIPT
            && active.Record.Document == document)
        {
            active.Withdrawn = true;
            dialog?.Hide();
        }
    }

    internal void Close()
    {
        closed = true;
        queue.Clear();
        dialog?.Hide();
    }

    // The event-facing async void contains its exceptions. A failed card is declined explicitly,
    // so an SDK refusal does not strand a question whose window never appeared.
    internal async void Start()
    {
        if (closed || running || root() is null) return;
        running = true;
        try
        {
            while (!closed && queue.TryDequeue(out Pending? pending))
            {
                if (pending.Withdrawn) continue;
                showing = pending;
                try
                {
                    await Show(pending);
                }
                catch (Exception error)
                {
                    say($"the dialogue could not be shown: {error.Message}");
                    if (!closed && !pending.Withdrawn)
                        commit("a dialogue declined after a window error", open => Decline(open, pending.Record));
                }
                finally
                {
                    dialog = null;
                    showing = null;
                }
            }
        }
        finally
        {
            running = false;
        }
    }

    private async Task Show(Pending pending)
    {
        Owed record = pending.Record;
        var card = new ContentDialog
        {
            XamlRoot = root(), Title = record.Text, CloseButtonText = "Cancel",
            DefaultButton = ContentDialogButton.Close
        };
        Action<QuorraWindow, ContentDialogResult> answer;
        switch (record.Kind)
        {
            case Native.QUORRA_OWED_PASSWORD:
                var password = new PasswordBox { Header = "Password" };
                card.Content = password;
                card.PrimaryButtonText = "Open";
                answer = (open, result) => open.Password(record.Document,
                    result == ContentDialogResult.Primary ? password.Password : null);
                break;
            case Native.QUORRA_OWED_QUESTION:
                card.Title = "Permission";
                card.Content = Text(record.Text);
                card.PrimaryButtonText = "Allow";
                card.CloseButtonText = "Decline";
                answer = (open, result) => open.Answer(result == ContentDialogResult.Primary);
                break;
            case Native.QUORRA_OWED_ARCHIVE:
                ArchiveChoice choice = record.Archive ?? throw new QuorraException("an archive card has no choices");
                var target = new ComboBox { Header = "Archive format", ItemsSource = choice.Targets,
                    SelectedIndex = choice.Target, HorizontalAlignment = HorizontalAlignment.Stretch };
                var profile = new ComboBox { Header = "Conversion policy", ItemsSource = choice.Profiles,
                    SelectedIndex = choice.Profile, HorizontalAlignment = HorizontalAlignment.Stretch };
                var sentence = Text(string.Empty);
                void Selected()
                {
                    int at = profile.SelectedIndex;
                    sentence.Text = at >= 0 && at < choice.Sentences.Count ? choice.Sentences[at] : string.Empty;
                    card.IsPrimaryButtonEnabled = target.SelectedIndex >= 0 && at >= 0;
                }
                target.SelectionChanged += (_, _) => Selected();
                profile.SelectionChanged += (_, _) => Selected();
                Selected();
                card.Content = Stack(target, profile, sentence);
                card.PrimaryButtonText = "Convert";
                answer = (open, result) => open.Archive(
                    result == ContentDialogResult.Primary ? target.SelectedIndex : -1,
                    result == ContentDialogResult.Primary ? profile.SelectedIndex : -1);
                break;
            case Native.QUORRA_OWED_SCRIPT:
                ScriptQuestion script = record.Script ?? throw new QuorraException("a script card has no question");
                card.Title = script.Title ?? "Document script";
                if (script.Kind == Native.QUORRA_SCRIPT_QUESTION_RESPONSE)
                {
                    var input = new TextBox { Header = script.Label ?? "Response", Text = script.Default };
                    var secret = new PasswordBox { Header = script.Label ?? "Response", Password = script.Default };
                    card.Content = Stack(Text(script.Message), script.Password ? secret : input);
                    card.PrimaryButtonText = "OK";
                    answer = (open, result) => open.AnswerScript(record.Document, Native.QUORRA_SCRIPT_ANSWER_TYPED,
                        0, result == ContentDialogResult.Primary ? (script.Password ? secret.Password : input.Text) : null);
                }
                else if (script.Kind == Native.QUORRA_SCRIPT_QUESTION_ALERT)
                {
                    string severity = script.Icon switch
                    {
                        Native.QUORRA_ALERT_ICON_ERROR => "Error",
                        Native.QUORRA_ALERT_ICON_WARNING => "Warning",
                        Native.QUORRA_ALERT_ICON_QUESTION => "Question",
                        _ => "Information"
                    };
                    card.Content = Stack(Text(severity), Text(script.Message));
                    bool yesNo = script.Buttons is Native.QUORRA_ALERT_BUTTONS_YES_NO
                        or Native.QUORRA_ALERT_BUTTONS_YES_NO_CANCEL;
                    card.PrimaryButtonText = yesNo ? "Yes" : "OK";
                    card.DefaultButton = ContentDialogButton.Primary;
                    card.SecondaryButtonText = yesNo ? "No" : string.Empty;
                    card.CloseButtonText = script.Buttons is Native.QUORRA_ALERT_BUTTONS_OK_CANCEL
                        or Native.QUORRA_ALERT_BUTTONS_YES_NO_CANCEL ? "Cancel" : string.Empty;
                    answer = (open, result) => open.AnswerScript(record.Document, Native.QUORRA_SCRIPT_ANSWER_PRESSED,
                        result == ContentDialogResult.Primary ? (yesNo ? Native.QUORRA_ALERT_BUTTON_YES : Native.QUORRA_ALERT_BUTTON_OK)
                        : result == ContentDialogResult.Secondary ? Native.QUORRA_ALERT_BUTTON_NO : script.Dismissed, null);
                }
                else throw new QuorraException($"unknown script question {script.Kind}");
                break;
            case Native.QUORRA_OWED_PRINT:
                PrintGrant print = record.Print ?? throw new QuorraException("a print card has no preview");
                card.Title = "Print preview";
                card.Content = Text($"{print.Pages} page(s). " +
                    (print.Degraded ? "The document restricts printing quality. " : string.Empty) +
                    "The page area shows the print preview. Printer selection and spooling are not available in this window yet.");
                card.CloseButtonText = "Finish preview";
                answer = (open, _) => open.PrintFinished();
                break;
            default:
                throw new QuorraException($"no dialogue for {Library.OwedName(record.Kind)}");
        }
        dialog = card;
        ContentDialogResult result = await card.ShowAsync();
        if (!closed && !pending.Withdrawn)
            commit("a dialogue answered", open => answer(open, result));
    }

    private static void Decline(QuorraWindow open, Owed record)
    {
        switch (record.Kind)
        {
            case Native.QUORRA_OWED_PASSWORD: open.Password(record.Document, null); break;
            case Native.QUORRA_OWED_QUESTION: open.Answer(false); break;
            case Native.QUORRA_OWED_ARCHIVE: open.Archive(-1, -1); break;
            case Native.QUORRA_OWED_SCRIPT:
                open.AnswerScript(record.Document, Native.QUORRA_SCRIPT_ANSWER_UNANSWERABLE); break;
            case Native.QUORRA_OWED_PRINT: open.PrintFinished(); break;
        }
    }

    private static TextBlock Text(string text) => new() { Text = text, TextWrapping = TextWrapping.Wrap };

    private static StackPanel Stack(params UIElement[] children)
    {
        var stack = new StackPanel { Spacing = 12 };
        foreach (UIElement child in children) stack.Children.Add(child);
        return stack;
    }
}
