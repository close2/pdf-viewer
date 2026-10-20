// The application object: one window, opened on what the command line names (RFC 0010 step N3a,
// ADR 2136).

using System;
using System.Linq;
using Microsoft.UI.Xaml;

namespace Quorra.App;

/// <summary>The WinUI application, whose one job is to open the window.</summary>
public partial class App : Application
{
    // Held so that the window outlives OnLaunched, as WinUI's own templates hold it.
    private MainWindow? window;

    /// <summary>Loads App.xaml's resources.</summary>
    public App()
    {
        InitializeComponent();
    }

    /// <summary>Opens the window on the command line's words and documents.</summary>
    /// <remarks>
    /// An unpackaged application's command line is the process's, and the launch arguments WinUI
    /// passes carry nothing for it, so the words are read from <see cref="Environment"/>; the
    /// first is the executable and is skipped.
    /// </remarks>
    protected override void OnLaunched(LaunchActivatedEventArgs args)
    {
        window = new MainWindow(Environment.GetCommandLineArgs().Skip(1).ToList());
        window.Activate();
    }
}
