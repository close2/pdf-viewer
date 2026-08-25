// The Qt half of the second native host: four classes, and moc runs over this file.
//
// Everything here is a *widget* decision. What a key means, what a row does, which control a
// field is and what value a check box takes are all on the Rust side, in `src/keys.rs`,
// `src/host.rs` and `viewer-host` — so that the two native hosts can be seen to agree about the
// clauses and to differ only in the toolkit. ADR 0246.
#pragma once

#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <vector>

#include <QAbstractItemModel>
#include <QAbstractListModel>
#include <QFrame>
#include <QImage>
#include <QMainWindow>
#include <QPixmap>
#include <QPoint>
#include <QString>
#include <QVector>
#include <QWidget>

#include "viewer-qt/src/bridge.cxx.h"

class QLabel;
class QLineEdit;
class QListView;
class QTabWidget;
class QTimer;
class QToolBar;
class QTreeView;

namespace pdf_viewer_qt {

/// One of `viewer-core`'s three panel answers, as a Qt item model.
///
/// **This is the shape Qt demanded that GTK did not.** `GtkTreeListModel` asks a closure for a
/// row's children the moment a person opens it, so `viewer-gtk` never builds a subtree nobody
/// looked at. `QAbstractItemModel` must answer `index`, `parent` and `rowCount` for any node at
/// any moment and must give every node an identity that outlives a `QModelIndex`, so the whole
/// tree is built up front and every node keeps a stable slot in `nodes_`. The rows arrive
/// flattened for exactly that reason. ADR 0246.
class PanelModel : public QAbstractItemModel
{
    Q_OBJECT

public:
    explicit PanelModel(QObject* parent = nullptr);

    /// Replaces every row. The whole model resets, because a new document is a new tree.
    void setRows(const rust::Vec<QtRow>& rows);

    /// Which flattened row a model index names, or -1.
    int flatRow(const QModelIndex& index) const;

    /// Which model index a flattened row is at, so that §12.3.3's `/Count` can open it.
    QModelIndex indexOfFlatRow(int flat) const;

    QModelIndex index(int row, int column, const QModelIndex& parent) const override;
    QModelIndex parent(const QModelIndex& child) const override;
    int rowCount(const QModelIndex& parent) const override;
    int columnCount(const QModelIndex& parent) const override;
    QVariant data(const QModelIndex& index, int role) const override;
    QVariant headerData(int section, Qt::Orientation orientation, int role) const override;
    bool setData(const QModelIndex& index, const QVariant& value, int role) override;
    Qt::ItemFlags flags(const QModelIndex& index) const override;

Q_SIGNALS:
    /// §8.11.4.3: a person moved a group's switch.
    void switched(int flatRow, bool on);

private:
    /// One node of the tree, holding its place rather than its data.
    struct Node
    {
        /// Which row of `rows_` this is; -1 for the invisible root.
        int flat;
        /// Which node is above it; -1 for the root.
        int parent;
        /// Which nodes are under it, in order.
        std::vector<int> children;
    };

    /// The node a model index names, or nullptr.
    const Node* nodeAt(const QModelIndex& index) const;

    std::vector<Node> nodes_;
    std::vector<QtRow> rows_;
    std::vector<int> nodeOfFlat_;
};

/// ISO 32000-2 §12.3.4's panel as a Qt item model: one row per page, and the miniature it states.
///
/// > An interactive PDF processor may display these images on the screen, allowing the user to
/// > navigate to a page by clicking its thumbnail image
///
/// **The rows are virtual, and that is a requirement rather than a technique.** `CLAUDE.md` section 2
/// forbids thumbnail generation on the launch path by name, and `viewer_core::Query::Thumbnail`
/// answers one page at a time so that a host can obey it. A `QAbstractListModel` is asked for the
/// rows a view is laying out and no others, so the decode happens in `data` and nowhere else:
/// opening this tab on a thousand-page document asks for the dozen rows on the screen.
///
/// What is kept afterwards is bounded by `viewer_host::KEPT_MINIATURES`, asked for across the
/// bridge rather than written down here — a decoded miniature is tens of kilobytes and a reader
/// who scrolls a thousand pages would otherwise leave the window holding all of them. Eviction is
/// by distance from the row last asked for rather than by age, for the reason a panel is scrolled:
/// what a view wants next is next to what it just wanted.
class PageModel : public QAbstractListModel
{
    Q_OBJECT

public:
    /// What one row needs, or nothing at all where the host could not be asked.
    ///
    /// The empty answer is the re-entrancy case and only that: `data` can be called while a call
    /// into the host is running, and a second one would be two borrows of one `rust::Box`. Such a
    /// row is drawn from its number and **not cached**, so the next lay-out asks again.
    using Fetch = std::function<std::optional<QtPage>(int)>;

    PageModel(Fetch fetch, int kept, QObject* parent = nullptr);

    /// How many pages the document has. Resets the model, because a new document is a new list.
    void setCount(int count);

    int rowCount(const QModelIndex& parent) const override;
    QVariant data(const QModelIndex& index, int role) const override;

private:
    /// One row, once it has been asked for.
    struct Held
    {
        /// §12.4.2's label for the page.
        QString label;
        /// The miniature, or a null pixmap for a page stating no `/Thumb` — most pages of most
        /// documents, and not a defect. The row is drawn either way.
        QPixmap picture;
    };

    /// This row, asking for it if it is not held and dropping the furthest if that goes over the
    /// bound. `nullptr` where the host refused, which is the re-entrancy case above.
    const Held* held(int row) const;

    Fetch fetch_;
    int kept_;
    int count_ = 0;
    /// Ordered, so that the two ends are the two candidates for "furthest from here"; `mutable`
    /// because `data` is const and a demand-driven model is the reason this class exists.
    mutable std::map<int, Held> held_;
};

/// The layer the interactive chrome is drawn on: over the page, over the controls, under nothing.
///
/// `Qt::WA_TransparentForMouseEvents` is what keeps a click on a `QLineEdit` underneath from
/// landing here instead — the same job `gtk_widget_set_can_target(FALSE)` does in the other host,
/// and both toolkits have an answer for it because both need one.
class ChromeOverlay : public QWidget
{
    Q_OBJECT

public:
    explicit ChromeOverlay(QWidget* parent);

    /// What to draw, in device pixels of the viewport.
    ///
    /// `matches` is every occurrence of the find bar's string on this page and `selection` is the
    /// one a person is on — two lists rather than one, because they say different things and are
    /// drawn in different weights of the same platform colour. `highlights` is Annex O's, which
    /// says how the document was opened rather than what is being done to it now.
    void setShapes(QVector<QtQuad> selection, QVector<QtQuad> matches, QVector<QtQuad> highlights,
                   QVector<QtQuad> focus, qreal scale);

protected:
    void paintEvent(QPaintEvent* event) override;

private:
    QVector<QtQuad> selection_;
    QVector<QtQuad> matches_;
    QVector<QtQuad> highlights_;
    QVector<QtQuad> focus_;
    qreal scale_ = 1.0;
};

/// One of ISO 32000-2 §12.5.6.14's popup windows, as real Qt window furniture.
///
/// The clause gives a popup "no appearance stream or associated actions of its own", so there is
/// nothing of it in the page's pixels: what a window *looks* like is the platform's, which is why
/// `viewer-core` answers a rectangle and three strings and why this is a `QFrame` rather than
/// something painted on the chrome overlay. The three texts and the box are
/// `viewer_host::popup`'s, shared with the other two hosts; the frame, the fonts and the palette
/// are Qt's.
class PopupWindow : public QFrame
{
    Q_OBJECT

public:
    /// Builds one window from what the host answered. Placed by the caller with `setGeometry`.
    PopupWindow(const QtPopup& window, QWidget* parent);
};

/// The page's pixels, the form's controls, and the chrome over both.
///
/// A plain `QWidget` with no layout is Qt's `GtkFixed`: children are placed by `setGeometry` at
/// the rectangles §12.7's widgets state, and the page is painted underneath them.
class PageArea : public QWidget
{
    Q_OBJECT

public:
    explicit PageArea(QWidget* parent = nullptr);

    /// The frames to paint, each with where its top-left corner sits in the viewport.
    ///
    /// A list since Table 29's `/PageLayout` was obeyed: `OneColumn` puts several pages in one
    /// window, and each of them is a raster of its own placed by the viewer.
    void setFrames(QList<QPair<QImage, QPointF>> frames);

    /// The layer the chrome is drawn on, kept last in the child order.
    ChromeOverlay* chrome() const { return chrome_; }

Q_SIGNALS:
    /// The viewport changed size, in device pixels.
    void resizedTo(unsigned int width, unsigned int height, float scale);
    /// The pointer moved or a button changed: 0 moved, 1 pressed, 2 dragged, 3 released.
    void pointerAt(float x, float y, unsigned char action);
    /// The wheel turned, in device pixels of the viewport.
    void scrolledBy(float dx, float dy);

protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;
    void wheelEvent(QWheelEvent* event) override;

private:
    /// One pointer event, in the device pixels the boundary speaks.
    void report(const QPointF& at, unsigned char action);

    QList<QPair<QImage, QPointF>> frames_;
    ChromeOverlay* chrome_;
    bool pressed_ = false;
};

/// The window: a splitter, `viewer_host::Tab`'s panels, a page, a status bar, and the host.
class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    /// Takes the host. C++ owns it for the whole life of the event loop.
    explicit MainWindow(rust::Box<Host> host);

private:
    /// Asks the host what changed and does it.
    void applyUpdates();
    /// Puts the frame on the screen and reports what the copy cost.
    void showFrame();
    /// Rebuilds §12.7's controls, which happens only when the set of them changes.
    void rebuildControls();
    /// Moves the controls the page already has and writes their values back into them.
    void placeControls();
    /// Rebuilds the three trees.
    void rebuildPanels();
    /// Rebuilds ISO 32000-2 §12.5.6.14's popup windows, which happens when the answer changes.
    ///
    /// Rebuilt rather than moved, which is the opposite of `placeControls`' rule and right for the
    /// opposite reason: a control holds the keyboard and a person's half-typed value, and a popup
    /// window holds neither — the clause gives it no actions of its own and
    /// `Qt::WA_TransparentForMouseEvents` means a press goes through it to the page.
    void rebuildPopups();
    /// §7.6.4.1's prompt, in a window of the platform's own.
    void askForAPassword();
    /// The third-party notices this binary is obliged to carry, in a window of their own.
    ///
    /// A licence obligation with a surface: `pdf-font` compiles the standard 14 font programs
    /// (ISO 32000-2 §9.6.2.2) into every binary in this tree and both of their licences require a
    /// *binary* distribution to reproduce their notices. `?` is the key, in all three hosts since
    /// ADR 0526; the text is `viewer_host::NOTICE`, so that two binaries of one program do not
    /// make two claims about one obligation.
    void showNotices();
    /// Builds the find bar: a real `QLineEdit` and two buttons in a `QToolBar`.
    void buildFindBar();
    /// Posts one step of the search on a zero-delay timer, for as long as pages remain.
    ///
    /// Annex O's `search` and a find bar's *next* read one page per step, because `viewer-core`
    /// has no thread to read a thousand of them on. Pumping through the event loop rather than in
    /// a loop here is what keeps the window painting while a 1023-page document is searched.
    void pumpSearch();
    /// Runs ISO 32000-2 §12.4.4.1's clock at whatever interval the host asks for, or stops it.
    ///
    /// The interval is `viewer_host::Clock`'s and not this window's: a still page wants a tick
    /// every tenth of a second and a transition wants a frame every sixtieth, and which is which
    /// is a clause's question rather than a toolkit's. A `-1` stops the timer outright, so a
    /// window that is not presenting has nothing armed at all.
    void pumpPresentation();
    /// Drains what an assistive technology has asked for, at whatever interval the host asks for.
    ///
    /// `pumpPresentation`'s shape and for a related reason: the interval is a *pull*, so this side
    /// owns the timer and Rust owns the decision. `accesskit_unix` calls the host back from its own
    /// D-Bus thread and there is no path from a foreign thread into `QApplication::exec` that would
    /// not cost `viewer-qt` a second hand-written `unsafe` token — so a request waits on a timer
    /// that does not exist until a client has attached. `-1` means nobody is listening, which is
    /// the overwhelmingly common case and arms nothing at all. ADR 0623.
    void pumpAccessibility();
    /// Tells the host where this window is on the screen, which AT-SPI needs to place a node.
    ///
    /// `QWidget::frameGeometry` and `QWidget::geometry` are both in screen coordinates, which is
    /// the thing GTK4 exposes nowhere — so this is the one place the two native hosts genuinely
    /// differ about ISO 32000-2 §14.7, and it is a difference between two platforms rather than
    /// between two programs.
    void reportPlacement();
    /// One tree, built once and filled thereafter.
    QTreeView* buildTree(unsigned char which);
    /// ISO 32000-2 §12.3.4's panel: a `QListView` of miniatures, built once and filled on demand.
    QListView* buildPages();
    /// Puts the window in the state Table 29 and ISO 32000-2 §12.2 ask for.
    ///
    /// Four sentences, four widgets: `/HideMenubar` is `QMainWindow::menuBar`, `/HideToolbar` the
    /// navigate and find tool bars, `/HideWindowUI` the status bar, and Table 29's "any other
    /// window visible" the panel of three trees. Full screen is `showFullScreen`, which is a
    /// fifth thing rather than a fourth: the sentence names the window controls too. ADR 0470.
    void applyChrome();

protected:
    /// What a key means is `src/keys.rs`'s, so this carries the `Qt::Key` number and no meaning.
    void keyPressEvent(QKeyEvent* event) override;
    /// The window moved, so every node's screen coordinates moved with it.
    void moveEvent(QMoveEvent* event) override;
    /// The window's frame changed size, which moves the contents' origin inside it.
    void resizeEvent(QResizeEvent* event) override;

private:

    rust::Box<Host> host_;
    QTabWidget* tabs_;
    /// One entry per `viewer_host::Tab`, in that list's own order, and **grown from the bridge
    /// rather than sized here**: the window asks `panel_label` until it runs out, so a panel added
    /// on the Rust side appears in this notebook without a line of C++ changing. §12.3.4's slot
    /// holds `nullptr` in both vectors, because that panel is a `QListView` and not a tree.
    std::vector<QTreeView*> trees_;
    std::vector<PanelModel*> models_;
    /// §12.3.4's panel, which is the one that is not a tree.
    QListView* pageView_ = nullptr;
    PageModel* pageModel_ = nullptr;
    PageArea* page_;
    QLabel* status_;
    /// The find bar, hidden until Ctrl+F or `/`.
    QToolBar* find_;
    /// The navigate bar, which with the find bar is what §12.2's `/HideToolbar` names.
    QToolBar* navigate_ = nullptr;
    /// The string in it.
    QLineEdit* needle_;
    std::vector<QWidget*> controls_;
    /// ISO 32000-2 §12.5.6.14's open windows, in the order the host answered them.
    std::vector<QWidget*> popups_;
    /// §12.4.4.1's clock, as one repeating timer that is stopped whenever nothing is presenting.
    ///
    /// One timer rather than a chain of `singleShot`s, because `applyUpdates` runs after every
    /// key press as well as after every tick: a one-shot armed from there would multiply.
    QTimer* clock_ = nullptr;
    /// The timer draining an assistive technology's requests, stopped whenever nobody is listening.
    QTimer* access_ = nullptr;
    /// Set while this window is writing values into its own controls, so that the write is not
    /// mistaken for a person typing. The same flag `viewer-gtk` calls `suppress`.
    bool writing_ = false;
    /// Set while a call into the host is running, and refused rather than nested.
    ///
    /// **This is the one thing the C++ side has to promise that Rust's compiler used to check.**
    /// `viewer-gtk` holds its host as `Rc<RefCell<Host>>` and a callback arriving mid-write gets a
    /// failed `try_borrow_mut` and a printed note; here the host is a `rust::Box` and a nested
    /// call would be two `&mut Host` at once, which is undefined behaviour with nothing to say
    /// so. Qt delivers events from a nested event loop — `QDialog::exec` is one — so the case is
    /// real rather than theoretical, and `Busy` in `window.cpp` is what makes the promise
    /// mechanical. ADR 0246.
    bool busy_ = false;
};

} // namespace pdf_viewer_qt
