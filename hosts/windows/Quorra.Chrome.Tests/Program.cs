// State transitions only: no WinUI substitute or assertion about XAML runtime behavior.
using Quorra.App;

static void Equal(ChromeLayout expected, ChromeLayout actual)
{
    if (actual != expected) throw new InvalidOperationException($"Expected {expected}; got {actual}");
}

var shown = new WindowChrome(true, true, true, true);
var hidden = new WindowChrome(false, false, false, false);
var chrome = new ChromeState();
Equal(new(true, true, true, true, true), chrome.Apply(shown, true));
// HideWindowUI must hide the toggle/status, not a document outline already showing.
Equal(new(true, false, false, true, true), chrome.Apply(shown with { WindowUI = false }, true));
// A toolbar preference cannot hide this host's menu button; there is no toolbar.
Equal(new(true, true, true, true, true), chrome.Apply(shown with { ToolBar = false }, true));
// A document cannot remove the reader's permission controls (ADR 1145).
Equal(new(true, true, true, true, true), chrome.Apply(shown with { MenuBar = false }, true));
Equal(new(false, false, false, false, false), chrome.Apply(hidden, true));
// Repeated hidden updates cannot replace the remembered open preference with forced closure.
Equal(new(false, false, false, false, false), chrome.Apply(hidden, false));
Equal(new(true, true, true, true, true), chrome.Apply(shown, false));
Equal(new(false, false, false, false, false), chrome.Apply(hidden, false));
Equal(new(true, true, true, true, false), chrome.Apply(shown, false));
Console.WriteLine("Chrome mapping: independent preferences and open/closed pane restoration passed.");

// Explicit exit/document requests override saved choice once; later chrome updates preserve the user.
Equal(new(true, true, true, true, true), chrome.Apply(shown, false, 1));
Equal(new(true, true, true, true, false), chrome.Apply(shown, false));
Equal(new(true, true, true, true, false), chrome.Apply(shown, true, -1));
Equal(new(true, true, true, true, true), chrome.Apply(shown, true));
