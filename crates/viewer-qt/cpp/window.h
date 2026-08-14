// The Qt half of the second native host: four classes, and moc runs over this file.
//
// Everything here is a *widget* decision. What a key means, what a row does, which control a
// field is and what value a check box takes are all on the Rust side, in `src/keys.rs`,
// `src/host.rs` and `viewer-host` — so that the two native hosts can be seen to agree about the
// clauses and to differ only in the toolkit. ADR 0246.
#pragma once

#include <cstdint>
#include <vector>

#include <QAbstractItemModel>
#include <QImage>
#include <QMainWindow>
#include <QPoint>
#include <QVector>
#include <QWidget>

#include "viewer-qt/src/bridge.cxx.h"

class QLabel;
class QLineEdit;
class QTabWidget;
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

/// The page's pixels, the form's controls, and the chrome over both.
///
/// A plain `QWidget` with no layout is Qt's `GtkFixed`: children are placed by `setGeometry` at
/// the rectangles §12.7's widgets state, and the page is painted underneath them.
class PageArea : public QWidget
{
    Q_OBJECT

public:
    explicit PageArea(QWidget* parent = nullptr);

    /// The frame to paint, and where its top-left corner sits in the viewport.
    void setFrame(QImage image, QPointF origin);

    /// The layer the chrome is drawn on, kept last in the child order.
    ChromeOverlay* chrome() const { return chrome_; }

Q_SIGNALS:
    /// The viewport changed size, in device pixels.
    void resizedTo(unsigned int width, unsigned int height, float scale);
    /// The pointer moved or a button changed: 0 moved, 1 pressed, 2 dragged, 3 released.
    void pointerAt(float x, float y, unsigned char action);

protected:
    void paintEvent(QPaintEvent* event) override;
    void resizeEvent(QResizeEvent* event) override;
    void mousePressEvent(QMouseEvent* event) override;
    void mouseMoveEvent(QMouseEvent* event) override;
    void mouseReleaseEvent(QMouseEvent* event) override;

private:
    /// One pointer event, in the device pixels the boundary speaks.
    void report(const QPointF& at, unsigned char action);

    QImage image_;
    QPointF origin_;
    ChromeOverlay* chrome_;
    bool pressed_ = false;
};

/// The window: a splitter, three trees, a page, a status bar, and the host behind all of it.
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
    /// §7.6.4.1's prompt, in a window of the platform's own.
    void askForAPassword();
    /// Builds the find bar: a real `QLineEdit` and two buttons in a `QToolBar`.
    void buildFindBar();
    /// Posts one step of the search on a zero-delay timer, for as long as pages remain.
    ///
    /// Annex O's `search` and a find bar's *next* read one page per step, because `viewer-core`
    /// has no thread to read a thousand of them on. Pumping through the event loop rather than in
    /// a loop here is what keeps the window painting while a 1023-page document is searched.
    void pumpSearch();
    /// One tree, built once and filled thereafter.
    QTreeView* buildTree(unsigned char which);

protected:
    /// What a key means is `src/keys.rs`'s, so this carries the `Qt::Key` number and no meaning.
    void keyPressEvent(QKeyEvent* event) override;

private:

    rust::Box<Host> host_;
    QTabWidget* tabs_;
    QTreeView* trees_[3];
    PanelModel* models_[3];
    PageArea* page_;
    QLabel* status_;
    /// The find bar, hidden until Ctrl+F or `/`.
    QToolBar* find_;
    /// The string in it.
    QLineEdit* needle_;
    std::vector<QWidget*> controls_;
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
