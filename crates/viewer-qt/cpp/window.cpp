// The Qt half of the second native host.
//
// Read this beside `crates/viewer-gtk/src/host.rs`: the two files do the same job for the same
// answers, and what differs between them is ADR 0246's subject.
#include "window.h"

#include <chrono>
#include <cstdint>
#include <vector>

#include <QAction>
#include <QApplication>
#include <QCheckBox>
#include <QComboBox>
#include <QDialog>
#include <QDialogButtonBox>
#include <QElapsedTimer>
#include <QHeaderView>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPalette>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QRadioButton>
#include <QResizeEvent>
#include <QMenuBar>
#include <QSplitter>
#include <QStatusBar>
#include <QTabWidget>
#include <QTimer>
#include <QToolBar>
#include <QTreeView>
#include <QVBoxLayout>
#include <QtGlobal>

namespace pdf_viewer_qt {
namespace {

/// A `rust::String` as Qt spells one. Both are UTF-8, so this is a length and a pointer.
QString text(const rust::String& from)
{
    return QString::fromUtf8(from.data(), static_cast<qsizetype>(from.size()));
}

/// How many rows §12.3.3's `/Count` will open before the tree stops obeying it.
///
/// The same bound `viewer-gtk` sets, and for the same reason: ISO 32000-2's own outline is nine
/// hundred items, so "do what the file asked" has to have an end.
constexpr int kExpansionLimit = 4096;

/// How far one notch of a wheel moves the page, in logical pixels.
///
/// A choice, and the same number `viewer-gtk` chose: the standard says nothing about a wheel, and
/// what a notch is worth is not a fact about a toolkit.
constexpr double kScrollStep = 48.0;

/// Refuses a call into the host that arrives while another one is running.
///
/// Two `&mut Host` at once is undefined behaviour and nothing here would say so, and Qt delivers
/// events from nested event loops. See `MainWindow::busy_`.
struct Busy
{
    explicit Busy(bool& flag) : flag_(flag) { flag_ = true; }
    ~Busy() { flag_ = false; }
    Busy(const Busy&) = delete;
    Busy& operator=(const Busy&) = delete;

private:
    bool& flag_;
};

/// One quadrilateral as a closed path, in the logical pixels Qt paints in.
QPainterPath pathOf(const QtQuad& quad, qreal scale)
{
    QPainterPath path;
    path.moveTo(quad.x0 / scale, quad.y0 / scale);
    path.lineTo(quad.x1 / scale, quad.y1 / scale);
    path.lineTo(quad.x2 / scale, quad.y2 / scale);
    path.lineTo(quad.x3 / scale, quad.y3 / scale);
    path.closeSubpath();
    return path;
}

/// The desktop's accent colour, and the name of the palette role it actually came from.
struct Accented
{
    QColor colour;
    QString role;
};

/// §12.5.1's focus ring, in whichever of two palette roles this Qt has.
///
/// **`QPalette::Accent` is Qt 6.6's, and a long-term distribution ships older.** Asking for the
/// enumerator unconditionally is not a portable request but a version requirement, and this file
/// made it silently: on Ubuntu's LTS — which is what CI's `qt6-base-dev` is — `window.cpp` did
/// not compile at all, and the whole workspace's `clippy` and `test` failed behind it for weeks.
/// So the floor this crate builds against is the Qt 6 that is actually installed, and the accent
/// is asked for where the enumerator exists.
///
/// `Highlight` is what stands in, because it is the same idea one step less specific: it is the
/// selection brush, which on most colour schemes *is* the accent, and it is a colour from the
/// desktop rather than one invented here. What must not happen is the host reporting an accent it
/// did not get — `doc/ui-boundary.md`'s argument for chrome crossing as geometry rests on that
/// sentence being checkable — so the role's own name travels beside the colour and `MainWindow`
/// prints whichever one this build asked for. ADR 0246, ADR 0450.
Accented accentOf(const QPalette& colours)
{
#if QT_VERSION >= QT_VERSION_CHECK(6, 6, 0)
    return {colours.color(QPalette::Accent), QStringLiteral("QPalette::Accent")};
#else
    return {colours.color(QPalette::Highlight),
            QStringLiteral("QPalette::Highlight, this Qt being older than 6.6, which is where "
                           "QPalette::Accent begins")};
#endif
}

} // namespace

// ---------------------------------------------------------------------------------------------
// PanelModel
// ---------------------------------------------------------------------------------------------

PanelModel::PanelModel(QObject* parent) : QAbstractItemModel(parent)
{
    nodes_.push_back(Node{-1, -1, {}});
}

void PanelModel::setRows(const rust::Vec<QtRow>& rows)
{
    beginResetModel();
    rows_.clear();
    nodes_.clear();
    nodeOfFlat_.clear();
    nodes_.push_back(Node{-1, -1, {}});

    // The rows arrive depth first with a depth on each, so one pass and a stack of open parents
    // rebuilds the tree. `open[d]` is the node that a row of depth `d + 1` belongs under.
    std::vector<int> open;
    open.push_back(0);
    for (std::size_t i = 0; i < rows.size(); ++i) {
        const QtRow& row = rows[i];
        rows_.push_back(row);
        const std::size_t depth = static_cast<std::size_t>(row.depth) + 1;
        // A malformed depth — one that skips a level — would index past the stack. Clamping is
        // the only answer that keeps a row visible, and the Rust side cannot produce it.
        while (open.size() > depth) {
            open.pop_back();
        }
        const int parent = open.empty() ? 0 : open.back();
        const int id = static_cast<int>(nodes_.size());
        nodes_.push_back(Node{static_cast<int>(i), parent, {}});
        nodes_[static_cast<std::size_t>(parent)].children.push_back(id);
        nodeOfFlat_.push_back(id);
        open.push_back(id);
    }
    endResetModel();
}

const PanelModel::Node* PanelModel::nodeAt(const QModelIndex& index) const
{
    const quintptr id = index.isValid() ? index.internalId() : 0;
    if (id >= nodes_.size()) {
        return nullptr;
    }
    return &nodes_[static_cast<std::size_t>(id)];
}

int PanelModel::flatRow(const QModelIndex& index) const
{
    const Node* node = nodeAt(index);
    return node == nullptr ? -1 : node->flat;
}

QModelIndex PanelModel::indexOfFlatRow(int flat) const
{
    if (flat < 0 || static_cast<std::size_t>(flat) >= nodeOfFlat_.size()) {
        return {};
    }
    const int id = nodeOfFlat_[static_cast<std::size_t>(flat)];
    const Node& node = nodes_[static_cast<std::size_t>(id)];
    const Node& parent = nodes_[static_cast<std::size_t>(node.parent)];
    for (std::size_t row = 0; row < parent.children.size(); ++row) {
        if (parent.children[row] == id) {
            return createIndex(static_cast<int>(row), 0, static_cast<quintptr>(id));
        }
    }
    return {};
}

QModelIndex PanelModel::index(int row, int column, const QModelIndex& parent) const
{
    const Node* node = nodeAt(parent);
    if (node == nullptr || row < 0 || static_cast<std::size_t>(row) >= node->children.size()) {
        return {};
    }
    return createIndex(row, column, static_cast<quintptr>(node->children[static_cast<std::size_t>(row)]));
}

QModelIndex PanelModel::parent(const QModelIndex& child) const
{
    const Node* node = nodeAt(child);
    if (node == nullptr || node->parent <= 0) {
        return {};
    }
    const Node& parent = nodes_[static_cast<std::size_t>(node->parent)];
    const Node& above = nodes_[static_cast<std::size_t>(parent.parent < 0 ? 0 : parent.parent)];
    for (std::size_t row = 0; row < above.children.size(); ++row) {
        if (above.children[row] == node->parent) {
            return createIndex(static_cast<int>(row), 0, static_cast<quintptr>(node->parent));
        }
    }
    return {};
}

int PanelModel::rowCount(const QModelIndex& parent) const
{
    if (parent.column() > 0) {
        return 0;
    }
    const Node* node = nodeAt(parent);
    return node == nullptr ? 0 : static_cast<int>(node->children.size());
}

int PanelModel::columnCount(const QModelIndex&) const
{
    // Two columns, which is Qt's own way of showing a second line and is where the two hosts
    // diverge in shape rather than in data: `viewer-gtk` stacks the label and the detail in a
    // `GtkBox` inside one column, because a `GtkListView` row is a widget.
    return 2;
}

QVariant PanelModel::data(const QModelIndex& index, int role) const
{
    const int flat = flatRow(index);
    if (flat < 0 || static_cast<std::size_t>(flat) >= rows_.size()) {
        return {};
    }
    const QtRow& row = rows_[static_cast<std::size_t>(flat)];
    if (role == Qt::DisplayRole) {
        return index.column() == 0 ? text(row.label) : text(row.detail);
    }
    // §8.11.4.3's switch is a *role* here and a widget in the other host. Qt puts the check box
    // in the model; GTK4's list view puts a `GtkCheckButton` in the row. ADR 0246.
    if (role == Qt::CheckStateRole && index.column() == 0 && row.action == 2) {
        return row.on ? Qt::Checked : Qt::Unchecked;
    }
    if (role == Qt::ToolTipRole && !row.detail.empty()) {
        return text(row.detail);
    }
    return {};
}

QVariant PanelModel::headerData(int section, Qt::Orientation orientation, int role) const
{
    if (orientation != Qt::Horizontal || role != Qt::DisplayRole) {
        return {};
    }
    return section == 0 ? QStringLiteral("Name") : QStringLiteral("Detail");
}

bool PanelModel::setData(const QModelIndex& index, const QVariant& value, int role)
{
    if (role != Qt::CheckStateRole) {
        return false;
    }
    const int flat = flatRow(index);
    if (flat < 0 || static_cast<std::size_t>(flat) >= rows_.size()) {
        return false;
    }
    QtRow& row = rows_[static_cast<std::size_t>(flat)];
    if (row.action != 2 || row.locked) {
        return false;
    }
    row.on = value.toInt() == Qt::Checked;
    Q_EMIT dataChanged(index, index, {Qt::CheckStateRole});
    Q_EMIT switched(flat, row.on);
    return true;
}

Qt::ItemFlags PanelModel::flags(const QModelIndex& index) const
{
    Qt::ItemFlags flags = Qt::ItemIsEnabled | Qt::ItemIsSelectable;
    const int flat = flatRow(index);
    if (flat < 0 || static_cast<std::size_t>(flat) >= rows_.size()) {
        return flags;
    }
    const QtRow& row = rows_[static_cast<std::size_t>(flat)];
    // Table 99's `/Locked`: "[t]he state of a locked group cannot be changed through the user
    // interface of an interactive PDF processor." The check box stays visible and shows the
    // document's own answer; what it loses is `ItemIsUserCheckable`, which is Qt's own way of
    // saying the state is not a person's to change.
    if (row.action == 2 && index.column() == 0 && !row.locked) {
        flags |= Qt::ItemIsUserCheckable;
    }
    return flags;
}

// ---------------------------------------------------------------------------------------------
// ChromeOverlay
// ---------------------------------------------------------------------------------------------

ChromeOverlay::ChromeOverlay(QWidget* parent) : QWidget(parent)
{
    setAttribute(Qt::WA_TransparentForMouseEvents, true);
    setAttribute(Qt::WA_NoSystemBackground, true);
    setAttribute(Qt::WA_TranslucentBackground, true);
}

void ChromeOverlay::setShapes(QVector<QtQuad> selection, QVector<QtQuad> matches,
                              QVector<QtQuad> highlights, QVector<QtQuad> focus, qreal scale)
{
    selection_ = std::move(selection);
    matches_ = std::move(matches);
    highlights_ = std::move(highlights);
    focus_ = std::move(focus);
    scale_ = scale > 0.0 ? scale : 1.0;
    update();
}

void ChromeOverlay::paintEvent(QPaintEvent*)
{
    if (selection_.isEmpty() && matches_.isEmpty() && highlights_.isEmpty() && focus_.isEmpty()) {
        return;
    }
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing, true);

    // `doc/ui-boundary.md`: "[e]mitting them as quads and points lets a native host draw selection
    // in **macOS's selection colour, KDE's accent, the Windows highlight brush**".
    //
    // **On Qt that sentence is literally satisfiable, and on GTK4 it was not.** ADR 0244 had to
    // record that GTK 4.22 exposes no accent colour to application code at all and settle for the
    // theme's foreground. `QPalette::Accent` has existed since Qt 6.6 and is the desktop's own —
    // KDE writes it from the colour scheme — and `QPalette::Highlight` is the selection brush
    // beside it. Both are asked for here, which is the argument for chrome crossing as geometry
    // paying off rather than being defended. ADR 0246. Which of the two a build older than Qt 6.6
    // gets, and why it says so out loud, is `accentOf` above.
    const QPalette& colours = palette();
    painter.setPen(Qt::NoPen);

    // ISO 32000-2 Annex O's highlighted rectangle, under everything else: the annex leaves its
    // nature to a processor, and what this one has to offer is the platform's own colour at the
    // faintest of the three weights — it is a statement about how the document was opened rather
    // than about what a person is doing in it.
    QColor asked = colours.color(QPalette::Highlight);
    asked.setAlphaF(0.18f);
    painter.setBrush(asked);
    for (const QtQuad& quad : highlights_) {
        painter.drawPath(pathOf(quad, scale_));
    }

    // Every other occurrence of the find bar's string, under the selection and fainter. The
    // standard says nothing whatever about what a match looks like, so this is a choice: the same
    // platform colour at a third of the alpha, so that "where else the word is" and "which one you
    // are on" are the same hue and never the same weight.
    QColor other = colours.color(QPalette::Highlight);
    other.setAlphaF(0.12f);
    painter.setBrush(other);
    for (const QtQuad& quad : matches_) {
        painter.drawPath(pathOf(quad, scale_));
    }

    QColor selection = colours.color(QPalette::Highlight);
    selection.setAlphaF(0.35f);
    painter.setBrush(selection);
    for (const QtQuad& quad : selection_) {
        painter.drawPath(pathOf(quad, scale_));
    }

    // §12.5.1: an annotation with the input focus. What a focus ring looks like is the platform's,
    // which is why this crosses as one quadrilateral and not as pixels.
    painter.setBrush(Qt::NoBrush);
    painter.setPen(QPen(accentOf(colours).colour, 2.0));
    for (const QtQuad& quad : focus_) {
        painter.drawPath(pathOf(quad, scale_));
    }
}

// ---------------------------------------------------------------------------------------------
// PageArea
// ---------------------------------------------------------------------------------------------

PageArea::PageArea(QWidget* parent) : QWidget(parent), chrome_(new ChromeOverlay(this))
{
    setMouseTracking(true);
    setAutoFillBackground(true);
    setFocusPolicy(Qt::StrongFocus);
}

void PageArea::setFrames(QList<QPair<QImage, QPointF>> frames)
{
    frames_ = std::move(frames);
    update();
}

void PageArea::paintEvent(QPaintEvent*)
{
    if (frames_.isEmpty()) {
        return;
    }
    QPainter painter(this);
    for (const QPair<QImage, QPointF>& frame : frames_) {
        if (!frame.first.isNull()) {
            painter.drawImage(frame.second, frame.first);
        }
    }
}

void PageArea::resizeEvent(QResizeEvent* event)
{
    QWidget::resizeEvent(event);
    chrome_->setGeometry(rect());
    chrome_->raise();
    const qreal scale = devicePixelRatioF();
    const int width = static_cast<int>(qRound(event->size().width() * scale));
    const int height = static_cast<int>(qRound(event->size().height() * scale));
    if (width <= 0 || height <= 0) {
        return;
    }
    Q_EMIT resizedTo(static_cast<unsigned int>(width), static_cast<unsigned int>(height),
                     static_cast<float>(scale));
}

void PageArea::wheelEvent(QWheelEvent* event)
{
    // **The wheel, which this host had no binding for until Table 29's `/PageLayout` was
    // obeyed.** Qt reports a high-resolution device in pixels and a notched one in eighths of a
    // degree, fifteen degrees to a notch; both are turned into the device pixels the boundary
    // speaks. The distance a notch moves is this host's choice — see kScrollStep — and it is the
    // same number `viewer-gtk` chose, because a wheel is not a fact about a toolkit.
    const QPoint pixels = event->pixelDelta();
    const qreal scale = devicePixelRatioF();
    qreal dx = 0.0;
    qreal dy = 0.0;
    if (!pixels.isNull()) {
        dx = -static_cast<qreal>(pixels.x()) * scale;
        dy = -static_cast<qreal>(pixels.y()) * scale;
    } else {
        const QPoint degrees = event->angleDelta();
        dx = -static_cast<qreal>(degrees.x()) / 120.0 * kScrollStep * scale;
        dy = -static_cast<qreal>(degrees.y()) / 120.0 * kScrollStep * scale;
    }
    if (dx == 0.0 && dy == 0.0) {
        QWidget::wheelEvent(event);
        return;
    }
    event->accept();
    Q_EMIT scrolledBy(static_cast<float>(dx), static_cast<float>(dy));
}

void PageArea::report(const QPointF& at, unsigned char action)
{
    const qreal scale = devicePixelRatioF();
    Q_EMIT pointerAt(static_cast<float>(at.x() * scale), static_cast<float>(at.y() * scale), action);
}

void PageArea::mousePressEvent(QMouseEvent* event)
{
    pressed_ = true;
    report(event->position(), 1);
}

void PageArea::mouseMoveEvent(QMouseEvent* event)
{
    // §12.5.5's three appearances follow the pointer, and §12.5.6.19's `/H` with them — which is
    // why a move with no button down is a command and not only a cursor question.
    report(event->position(), pressed_ ? 2 : 0);
}

void PageArea::mouseReleaseEvent(QMouseEvent* event)
{
    pressed_ = false;
    report(event->position(), 3);
}

// ---------------------------------------------------------------------------------------------
// MainWindow
// ---------------------------------------------------------------------------------------------

MainWindow::MainWindow(rust::Box<Host> host)
    : host_(std::move(host)), tabs_(new QTabWidget), page_(new PageArea), status_(new QLabel)
{
    const rust::Vec<std::int32_t> size = host_->window_size();
    resize(size.size() >= 2 ? size[0] : 1000, size.size() >= 2 ? size[1] : 1100);
    setWindowTitle(text(host_->title()));

    // The ground the pages are laid on. **Not ISO 32000-2 §11.4.7's 𝑊**, which is the page's own
    // colour and is composited by the rasteriser inside §14.11.2.1's crop box; what lies outside
    // every page is no clause's subject and is this program's documented choice. Read from the
    // Rust side (`pdf_render::SURROUND`, through `Host::surround`) rather than written here, so
    // that the three hosts and the three rasterisers state one fact once — and taken at all
    // because the toolkit's own window background is within a few levels of paper white, which
    // made the gap between two pages of a column as good as invisible.
    const rust::Vec<std::uint8_t> ground = host_->surround();
    if (ground.size() >= 3) {
        QPalette laid = page_->palette();
        laid.setColor(QPalette::Window, QColor(ground[0], ground[1], ground[2]));
        page_->setPalette(laid);
    }

    static const char* const kTabs[3] = {"Outline", "Layers", "Files"};
    for (unsigned char which = 0; which < 3; ++which) {
        trees_[which] = buildTree(which);
        tabs_->addTab(trees_[which], QString::fromLatin1(kTabs[which]));
    }

    auto* split = new QSplitter(Qt::Horizontal);
    split->addWidget(tabs_);
    split->addWidget(page_);
    split->setStretchFactor(1, 1);
    split->setSizes({300, 700});
    setCentralWidget(split);

    status_->setTextInteractionFlags(Qt::TextSelectableByMouse);
    statusBar()->addWidget(status_, 1);

    auto* bar = addToolBar(QStringLiteral("Navigate"));
    bar->setMovable(false);
    navigate_ = bar;
    static const struct
    {
        const char* label;
        unsigned char command;
    } kButtons[] = {{"‹", 0}, {"›", 1}, {"−", 2}, {"+", 3}, {"Fit", 4}};
    for (const auto& button : kButtons) {
        QAction* action = bar->addAction(QString::fromUtf8(button.label));
        const unsigned char what = button.command;
        connect(action, &QAction::triggered, this, [this, what] {
            if (busy_) {
                return;
            }
            Busy guard(busy_);
            host_->command(what);
            applyUpdates();
        });
    }

    // Said out loud rather than assumed, because it is the one place `doc/ui-boundary.md`'s
    // argument for chrome crossing as geometry can be *checked*: the two colours below are the
    // desktop's own, and ADR 0244 had to record that GTK4 would part with neither.
    {
        const QPalette& colours = palette();
        const Accented accent = accentOf(colours);
        const QString said = QStringLiteral("chrome in the platform's colours: selection %1 "
                                            "(QPalette::Highlight), focus ring %2 (%3)")
                                 .arg(colours.color(QPalette::Highlight).name(),
                                      accent.colour.name(), accent.role);
        const QByteArray utf8 = said.toUtf8();
        host_->note(rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
    }

    buildFindBar();

    connect(page_, &PageArea::resizedTo, this, [this](unsigned int width, unsigned int height, float scale) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        host_->resized(width, height, scale);
        applyUpdates();
    });
    connect(page_, &PageArea::pointerAt, this, [this](float x, float y, unsigned char action) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        host_->pointer(x, y, action);
        applyUpdates();
    });
    connect(page_, &PageArea::scrolledBy, this, [this](float dx, float dy) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        host_->scrolled(dx, dy);
        applyUpdates();
    });
}

QTreeView* MainWindow::buildTree(unsigned char which)
{
    auto* view = new QTreeView;
    auto* model = new PanelModel(view);
    models_[which] = model;
    view->setModel(model);
    view->setUniformRowHeights(true);
    view->header()->setStretchLastSection(true);

    // §12.3.3: "[c]licking the text of any visible item activates the item". `clicked` rather than
    // `activated`, because `activated` is a double click under most styles and the clause says a
    // click.
    connect(view, &QTreeView::clicked, this, [this, which, model](const QModelIndex& index) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        const int flat = model->flatRow(index);
        if (flat >= 0) {
            host_->activate_row(which, static_cast<std::size_t>(flat));
        }
        applyUpdates();
    });
    connect(model, &PanelModel::switched, this, [this, which](int flat, bool on) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        host_->toggle_row(which, static_cast<std::size_t>(flat), on);
        applyUpdates();
    });
    return view;
}

// The find bar, and every widget in it is Qt's: a `QLineEdit` with a clear button, two actions
// with the platform's own shortcuts, and a `QToolBar` that hides. Nothing here is drawn by this
// program — `doc/ui-boundary.md`'s rule applied to a find bar, which is that the geometry of the
// matches crosses and the *bar* belongs to the desktop.
void MainWindow::buildFindBar()
{
    find_ = addToolBar(QStringLiteral("Find"));
    find_->setMovable(false);
    find_->hide();

    needle_ = new QLineEdit(find_);
    needle_->setClearButtonEnabled(true);
    needle_->setPlaceholderText(QStringLiteral("Find in document"));
    needle_->setMaximumWidth(360);
    find_->addWidget(needle_);

    auto step = [this](bool backward) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        host_->find(backward);
        applyUpdates();
    };
    QAction* previous = find_->addAction(QStringLiteral("Previous"));
    previous->setShortcut(QKeySequence::FindPrevious);
    connect(previous, &QAction::triggered, this, [step] { step(true); });
    QAction* next = find_->addAction(QStringLiteral("Next"));
    next->setShortcut(QKeySequence::FindNext);
    connect(next, &QAction::triggered, this, [step] { step(false); });

    // Typing highlights this page and searches nothing, which is the split `Query::Find` and
    // `Command::Find` make: one is free and one interprets pages.
    connect(needle_, &QLineEdit::textEdited, this, [this](const QString& typed) {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        const QByteArray utf8 = typed.toUtf8();
        host_->retype(rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
        applyUpdates();
    });
    connect(needle_, &QLineEdit::returnPressed, this, [step] { step(false); });

    auto* open = new QAction(this);
    open->setShortcuts({QKeySequence::Find, QKeySequence(Qt::Key_Slash)});
    connect(open, &QAction::triggered, this, [this] {
        find_->show();
        needle_->setFocus();
        needle_->selectAll();
    });
    addAction(open);

    // **Escape means two things and the order is the Rust side's.** A `QAction` shortcut consumes
    // the key before `keyPressEvent` ever sees it, so this is the only place Escape arrives in
    // this window — and since ADR 0470 the first thing it may mean is *leave full screen*, which
    // no clause states and which `src/keys.rs` documents as a choice. Forwarding the key rather
    // than deciding here is what keeps the three hosts agreeing about what a key means.
    auto* close = new QAction(this);
    close->setShortcut(QKeySequence(Qt::Key_Escape));
    connect(close, &QAction::triggered, this, [this] {
        if (busy_) {
            return;
        }
        if (host_->chrome().full_screen) {
            Busy guard(busy_);
            host_->key(static_cast<unsigned int>(Qt::Key_Escape));
            applyUpdates();
            return;
        }
        if (!find_->isVisible()) {
            return;
        }
        find_->hide();
        needle_->clear();
        Busy guard(busy_);
        host_->find_stop();
        applyUpdates();
    });
    addAction(close);
}

void MainWindow::pumpSearch()
{
    if (!host_->searching()) {
        return;
    }
    QTimer::singleShot(0, this, [this] {
        if (busy_) {
            return;
        }
        Busy guard(busy_);
        host_->find_continue();
        applyUpdates();
    });
}

void MainWindow::keyPressEvent(QKeyEvent* event)
{
    if (busy_) {
        QMainWindow::keyPressEvent(event);
        return;
    }
    Busy guard(busy_);
    host_->key(static_cast<unsigned int>(event->key()));
    applyUpdates();
}

void MainWindow::applyUpdates()
{
    const QtUpdate update = host_->take_update();
    if (update.panels) {
        rebuildPanels();
    }
    if (update.controls) {
        rebuildControls();
    }
    if (update.frame) {
        showFrame();
        placeControls();
    }
    if (update.chrome) {
        QVector<QtQuad> selection;
        for (const QtQuad& quad : host_->selection()) {
            selection.push_back(quad);
        }
        QVector<QtQuad> matches;
        for (const QtQuad& quad : host_->matches()) {
            matches.push_back(quad);
        }
        QVector<QtQuad> highlights;
        for (const QtQuad& quad : host_->highlights()) {
            highlights.push_back(quad);
        }
        QVector<QtQuad> focus;
        for (const QtQuad& quad : host_->focus()) {
            focus.push_back(quad);
        }
        page_->chrome()->setShapes(std::move(selection), std::move(matches), std::move(highlights),
                                   std::move(focus), page_->devicePixelRatioF());
    }
    if (update.title) {
        setWindowTitle(text(host_->title()));
    }
    if (update.status) {
        status_->setText(text(host_->status()));
    }
    pumpSearch();
    if (update.window) {
        applyChrome();
    }
    if (update.password) {
        // Queued rather than called: `QDialog::exec` runs a nested event loop, and starting one
        // from inside a handler that is holding the host would be exactly the nesting `busy_`
        // exists to refuse. This lets the current handler unwind first.
        QTimer::singleShot(0, this, [this] { askForAPassword(); });
    }
}

// Table 29's `FullScreen` and ISO 32000-2 §12.2's three hide flags, in Qt widgets.
//
// Which sentence is in force is decided once on the Rust side and shared with the other two hosts
// (`viewer_host::Presenting`); what is here is the mapping from four booleans onto four widgets,
// which is the half that genuinely is a toolkit's. ADR 0470.
void MainWindow::applyChrome()
{
    const QtChrome chrome = host_->chrome();
    // §12.2: "whether to hide the interactive PDF processor's tool bars when the document is
    // active" — both of them, and the find bar goes with its own string.
    if (navigate_ != nullptr) {
        navigate_->setVisible(chrome.tool_bar);
    }
    if (!chrome.tool_bar && find_->isVisible()) {
        find_->hide();
        needle_->clear();
        host_->find_stop();
    }
    // §12.2: "user interface elements in the document's window (such as scroll bars and
    // navigation controls), leaving only the document's contents displayed".
    statusBar()->setVisible(chrome.window_ui);
    // Table 29: "or any other window visible".
    tabs_->setVisible(chrome.other_windows);
    // §12.2's `/HideMenubar` names a widget none of the three hosts draws. `findChild` rather than
    // `menuBar()` deliberately: the latter *creates* an empty bar, so obeying the flag that way
    // would put a strip on the screen that hiding it then took away again. A window that grows a
    // menu is obeyed here without a line changing.
    if (QMenuBar* menus = findChild<QMenuBar*>(); menus != nullptr) {
        menus->setVisible(chrome.menu_bar);
    }
    if (chrome.full_screen) {
        showFullScreen();
        return;
    }
    if (isFullScreen()) {
        showNormal();
    }
    // Table 29's `/PageMode` when the document opens and §12.2's `/NonFullScreenPageMode` when
    // full screen ends — one question, because the Rust side answers with whichever of the two
    // clauses applies and this window has no business knowing which.
    const int wanted = host_->panel_wanted();
    if (wanted >= 0 && wanted < tabs_->count()) {
        tabs_->setCurrentIndex(wanted);
    }
}

void MainWindow::showFrame()
{
    const std::size_t count = host_->frame_count();
    if (count == 0) {
        return;
    }
    // Tier 1's whole cost, in the one place it is paid. `Raster` is row-major RGBA with straight
    // alpha and no padding, which is `QImage::Format_RGBA8888` exactly — so this is a `memcpy`
    // and no conversion, the same crossing `viewer-gtk` makes into a `gdk::MemoryTexture`.
    // `QImage::copy` is what performs it: the constructor below only wraps the borrowed slice.
    // One copy per page of Table 29's arrangement, which under `SinglePage` is the one it
    // always was.
    QElapsedTimer clock;
    clock.start();
    QList<QPair<QImage, QPointF>> frames;
    std::size_t bytes = 0;
    const qreal scale = page_->devicePixelRatioF();
    for (std::size_t index = 0; index < count; ++index) {
        const QtFrame frame = host_->frame(index);
        if (!frame.present || frame.width == 0 || frame.height == 0) {
            continue;
        }
        const rust::Slice<const std::uint8_t> pixels = host_->frame_pixels(index);
        const qsizetype stride = static_cast<qsizetype>(frame.width) * 4;
        if (static_cast<qsizetype>(pixels.size()) < stride * static_cast<qsizetype>(frame.height)) {
            // The Rust side checks this too, in `page::describe`. Checked twice deliberately: this
            // is the one number that decides how many bytes are read out of somebody else's
            // allocation.
            continue;
        }
        const QImage borrowed(pixels.data(), static_cast<int>(frame.width),
                              static_cast<int>(frame.height), static_cast<int>(stride),
                              QImage::Format_RGBA8888);
        QImage owned = borrowed.copy();
        owned.setDevicePixelRatio(scale);
        frames.append(qMakePair(std::move(owned),
                                QPointF(frame.origin_x / scale, frame.origin_y / scale)));
        bytes += static_cast<std::size_t>(stride) * static_cast<std::size_t>(frame.height);
    }
    if (frames.isEmpty()) {
        return;
    }
    const qint64 nanos = clock.nsecsElapsed();
    page_->setFrames(std::move(frames));
    host_->painted(bytes, static_cast<std::uint64_t>(nanos));
}

void MainWindow::rebuildPanels()
{
    // Timed, because it is on the launch path and it is the one thing Qt's model made *eager*:
    // `GtkTreeListModel` pulls a subtree when a person opens it, `QAbstractItemModel` must answer
    // for every node at any moment, so a document whose outline is a thousand items builds a
    // thousand nodes before the first frame. `CLAUDE.md` makes that a number to keep rather than a
    // decision to defend. ADR 0246.
    QElapsedTimer clock;
    clock.start();
    int built = 0;
    for (unsigned char which = 0; which < 3; ++which) {
        const rust::Vec<QtRow> rows = host_->rows(which);
        models_[which]->setRows(rows);
        // §12.3.3 gives an outline item's `/Count` a sign for it — "[i]f the outline item is open,
        // Count is the sum of the number of visible descendent outline items" — so a tree that
        // opened everything, or nothing, would be discarding a statement the file made.
        const int limit = static_cast<int>(rows.size()) < kExpansionLimit
                              ? static_cast<int>(rows.size())
                              : kExpansionLimit;
        for (int flat = 0; flat < limit; ++flat) {
            if (rows[static_cast<std::size_t>(flat)].expanded) {
                trees_[which]->setExpanded(models_[which]->indexOfFlatRow(flat), true);
            }
        }
        trees_[which]->resizeColumnToContents(0);
        built += static_cast<int>(rows.size());
    }
    const QString said = QStringLiteral("%1 tree row(s) into %2 model(s) in %3 µs")
                             .arg(built)
                             .arg(3)
                             .arg(clock.nsecsElapsed() / 1000);
    const QByteArray utf8 = said.toUtf8();
    host_->note(rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
}

void MainWindow::rebuildControls()
{
    for (QWidget* control : controls_) {
        control->deleteLater();
    }
    controls_.clear();

    const rust::Vec<QtControl> wanted = host_->controls();
    for (std::size_t index = 0; index < wanted.size(); ++index) {
        const QtControl& control = wanted[index];
        QWidget* widget = nullptr;
        switch (control.kind) {
        case 0: { // §12.7.5.3's single-line text field
            auto* entry = new QLineEdit(page_);
            if (control.max_len >= 0) {
                entry->setMaxLength(control.max_len);
            }
            // `textEdited` rather than `textChanged`: Qt emits the first only for what a person
            // typed, so writing the field's own value back cannot look like a second keystroke.
            // That is one flag `viewer-gtk` needs and this host does not — GTK4's `Entry` has no
            // signal that distinguishes the two.
            connect(entry, &QLineEdit::textEdited, this, [this, index](const QString& typed) {
                if (busy_) {
                    return;
                }
                Busy guard(busy_);
                const QByteArray utf8 = typed.toUtf8();
                host_->set_control(index, rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
                applyUpdates();
            });
            widget = entry;
            break;
        }
        case 1: { // Table 231 bit 13: "the field may contain multiple lines of text"
            auto* entry = new QPlainTextEdit(page_);
            connect(entry, &QPlainTextEdit::textChanged, this, [this, index, entry] {
                if (busy_ || writing_) {
                    return;
                }
                Busy guard(busy_);
                const QByteArray utf8 = entry->toPlainText().toUtf8();
                host_->set_control(index, rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
                applyUpdates();
            });
            widget = entry;
            break;
        }
        case 2: { // Table 231 bit 14: "a secure password that should not be echoed visibly"
            // Qt's answer is an echo mode on the ordinary entry where GTK4 has a control of its
            // own. Either way it is the platform's own secure entry, which is what the flag asks
            // for — and it is the one control whose value this host never writes back, because
            // `Answer::Field` answers a password field with bullets (ADR 0244 finding 3).
            auto* entry = new QLineEdit(page_);
            entry->setEchoMode(QLineEdit::Password);
            if (control.max_len >= 0) {
                entry->setMaxLength(control.max_len);
            }
            connect(entry, &QLineEdit::textEdited, this, [this, index](const QString& typed) {
                if (busy_) {
                    return;
                }
                Busy guard(busy_);
                const QByteArray utf8 = typed.toUtf8();
                host_->set_control(index, rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
                applyUpdates();
            });
            widget = entry;
            break;
        }
        case 3:   // §12.7.5.2.3's check box
        case 4: { // §12.7.5.2.4's radio button
            QAbstractButton* button = control.kind == 3 ? static_cast<QAbstractButton*>(new QCheckBox(page_))
                                                        : static_cast<QAbstractButton*>(new QRadioButton(page_));
            button->setParent(page_);
            // Qt would put every radio button of one parent into one exclusive group, which is not
            // §12.7.5.2.4's grouping: a PDF's set is the widgets of one *field*, and two fields'
            // buttons on one page are two sets. So exclusivity is off and the clause's own rule —
            // `/V` names the on state, Table 229 bit 15 decides what a second click does — is
            // enforced where it belongs, on the Rust side.
            button->setAutoExclusive(false);
            connect(button, &QAbstractButton::toggled, this, [this, index](bool on) {
                if (busy_ || writing_) {
                    return;
                }
                Busy guard(busy_);
                host_->toggle_control(index, on);
                applyUpdates();
            });
            widget = button;
            break;
        }
        case 5: { // §12.7.5.2.2's push button, "without retaining a permanent value"
            auto* button = new QPushButton(page_);
            button->setText(text(control.tooltip));
            connect(button, &QPushButton::clicked, this, [this, index] {
                if (busy_) {
                    return;
                }
                Busy guard(busy_);
                host_->activate_control(index);
                applyUpdates();
            });
            widget = button;
            break;
        }
        case 6: { // Table 233 bit 18 set: a combo box
            auto* combo = new QComboBox(page_);
            for (const rust::String& option : host_->control_options(index)) {
                combo->addItem(text(option));
            }
            // Bit 19: "the combo box shall include an editable text box as well as a drop-down
            // list". **Qt can obey this and GTK4 could not** — `GtkDropDown` is not editable, so
            // `viewer-gtk` carries the flag and reports it. That is the one place the two hosts
            // differ in what they can do rather than in how they spell it.
            combo->setEditable(control.editable);
            connect(combo, &QComboBox::currentTextChanged, this, [this, index, combo](const QString& chosen) {
                if (busy_ || writing_) {
                    return;
                }
                Busy guard(busy_);
                // An editable combo box's text need not be one of Table 234's options at all —
                // bit 19 lets "the user … type a value other than the predefined choices" — so
                // that one sends characters and a plain drop-down sends the position it picked.
                if (combo->isEditable() || combo->currentIndex() < 0) {
                    const QByteArray utf8 = chosen.toUtf8();
                    host_->set_control(index, rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
                } else {
                    const std::uint32_t one = static_cast<std::uint32_t>(combo->currentIndex());
                    host_->choose_control(index, rust::Slice<const std::uint32_t>(&one, 1));
                }
                applyUpdates();
            });
            widget = combo;
            break;
        }
        case 7: { // Table 233 bit 18 clear: a list box
            auto* list = new QListWidget(page_);
            for (const rust::String& option : host_->control_options(index)) {
                list->addItem(text(option));
            }
            // Table 233 bit 22: "(PDF 1.4) If set, more than one of the field's option items may
            // be selected simultaneously; if clear, at most one item shall be selected." This host
            // asked for SingleSelection either way until the four-hundred-and-twelfth session,
            // because `Edit::SetField` carried one value; the vocabulary carries a set now, so the
            // flag decides the selection mode and the clause is obeyed rather than reported
            // (ADR 0248).
            list->setSelectionMode(control.multi ? QAbstractItemView::ExtendedSelection
                                                 : QAbstractItemView::SingleSelection);
            // Table 234's `/TI`, "the index in the Opt array of the first option visible in the
            // list": where a scrollable list *starts*, which is not where the selection is. The
            // page's own appearance has obeyed it since ADR 0407, and a control placed over that
            // picture showing a different first row is the same disagreement a mark would be.
            if (const int top = static_cast<int>(control.top); top > 0 && top < list->count()) {
                list->scrollToItem(list->item(top), QAbstractItemView::PositionAtTop);
            }
            connect(list, &QListWidget::itemSelectionChanged, this, [this, index, list] {
                if (busy_ || writing_) {
                    return;
                }
                Busy guard(busy_);
                // The rows themselves, ascending, which is the order Table 234's `/I` wants and
                // the order `QListWidget::selectedIndexes` does not promise.
                std::vector<std::uint32_t> chosen;
                for (int row = 0; row < list->count(); ++row) {
                    if (list->item(row)->isSelected()) {
                        chosen.push_back(static_cast<std::uint32_t>(row));
                    }
                }
                host_->choose_control(index, rust::Slice<const std::uint32_t>(chosen.data(), chosen.size()));
                applyUpdates();
            });
            widget = list;
            break;
        }
        default:
            // §12.7.5.5's signature and Table 226's absent `/FT` never reach here — the Rust side
            // places no control for either — and a number this host does not know is said rather
            // than drawn as something else.
            host_->note("a control kind this window has no widget for was skipped");
            break;
        }
        if (widget == nullptr) {
            continue;
        }
        // Table 227 bit 1: "the field shall not be modified by the user". The platform's own way
        // of saying so, rather than a refusal after the fact.
        widget->setEnabled(!control.read_only);
        if (!control.tooltip.empty()) {
            widget->setToolTip(text(control.tooltip));
        }
        widget->show();
        controls_.push_back(widget);
    }
    host_->note("controls rebuilt");
}

void MainWindow::placeControls()
{
    const rust::Vec<QtControl> wanted = host_->controls();
    if (wanted.size() != controls_.size()) {
        return;
    }
    const qreal scale = page_->devicePixelRatioF() > 0.0 ? page_->devicePixelRatioF() : 1.0;
    // ADR 0244's second finding, measured again on a second toolkit. A `/Rect` is whatever the
    // document says; a platform control has a *minimum* size its style decides, so a control whose
    // minimum exceeds its rectangle covers the page around it.
    //
    // What is taken here is only the pair of sizes: `minimumSizeHint` is a Qt style's opinion and
    // nothing else can ask it, and everything done *with* the pairs is `viewer_host::ControlFit`'s
    // on the Rust side, where `viewer-gtk`'s numbers go too (ADR 0346). This side used to count and
    // report by itself, which meant two hosts computing one finding twice and only one of them able
    // to offer the magnification that fixes it.
    std::vector<QtMeasure> measured;
    measured.reserve(wanted.size());
    writing_ = true;
    for (std::size_t index = 0; index < wanted.size(); ++index) {
        const QtControl& control = wanted[index];
        QWidget* widget = controls_[index];
        const int askedWidth = qRound(control.width / scale);
        const int askedHeight = qRound(control.height / scale);
        const QSize least = widget->minimumSizeHint();
        measured.push_back(QtMeasure{askedWidth, askedHeight, least.width(), least.height()});
        widget->setGeometry(QRect(qRound(control.x / scale), qRound(control.y / scale),
                                  askedWidth, askedHeight));
        // ADR 0201: a host keeps the *point* it clicked and never the text, because §12.7.5.3's
        // truncation means the field can take less than was typed — so the control shows what the
        // field took. `obscured` is the exception, and since ADR 0247 it is the *answer* that says
        // so rather than this switch leaving the password entry out: Table 231 bit 14's field
        // answers with bullets, and writing those back would send them as the next value.
        if (control.obscured) {
            continue;
        }
        const QString value = text(control.value);
        switch (control.kind) {
        case 0:
            if (auto* entry = qobject_cast<QLineEdit*>(widget); entry != nullptr && entry->text() != value) {
                entry->setText(value);
            }
            break;
        case 1:
            if (auto* entry = qobject_cast<QPlainTextEdit*>(widget);
                entry != nullptr && entry->toPlainText() != value) {
                entry->setPlainText(value);
            }
            break;
        case 3:
        case 4:
            if (auto* button = qobject_cast<QAbstractButton*>(widget); button != nullptr) {
                button->setChecked(control.on);
            }
            break;
        case 6:
            if (auto* combo = qobject_cast<QComboBox*>(widget); combo != nullptr) {
                const rust::Vec<std::uint32_t> chosen = host_->control_selection(index);
                if (!chosen.empty()) {
                    combo->setCurrentIndex(static_cast<int>(chosen[0]));
                }
            }
            break;
        case 7:
            if (auto* list = qobject_cast<QListWidget*>(widget); list != nullptr) {
                const rust::Vec<std::uint32_t> chosen = host_->control_selection(index);
                // Every selected row and not just the first: Table 233 bit 22's field may hold
                // several, so writing back one would silently drop what a person had chosen the
                // moment anything else redrew the controls.
                std::vector<bool> wantSelected(static_cast<std::size_t>(list->count()), false);
                for (const std::uint32_t row : chosen) {
                    if (static_cast<int>(row) < list->count()) {
                        wantSelected[static_cast<std::size_t>(row)] = true;
                    }
                }
                for (int row = 0; row < list->count(); ++row) {
                    const bool want = wantSelected[static_cast<std::size_t>(row)];
                    if (list->item(row)->isSelected() != want) {
                        list->item(row)->setSelected(want);
                    }
                }
            }
            break;
        default:
            break;
        }
    }
    writing_ = false;
    // One call rather than one per control: the answer is the worst ratio over the whole page, and
    // a bridge crossing per widget would be seventy-six of them on `160F-2019.pdf` to compute one
    // number. An empty page is still sent, because "nothing is placed" is what clears the previous
    // page's magnification.
    host_->measured(rust::Slice<const QtMeasure>(measured.data(), measured.size()));
}

void MainWindow::askForAPassword()
{
    if (busy_) {
        return;
    }
    QDialog dialog(this);
    dialog.setWindowTitle(QStringLiteral("Password"));
    dialog.setModal(true);
    auto* column = new QVBoxLayout(&dialog);
    column->addWidget(new QLabel(QStringLiteral("This document is encrypted (§7.6.4.1)."), &dialog));
    auto* entry = new QLineEdit(&dialog);
    entry->setEchoMode(QLineEdit::Password);
    column->addWidget(entry);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, &dialog);
    column->addWidget(buttons);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    connect(entry, &QLineEdit::returnPressed, &dialog, &QDialog::accept);
    entry->setFocus();

    if (dialog.exec() != QDialog::Accepted) {
        return;
    }
    const QByteArray utf8 = entry->text().toUtf8();
    Busy guard(busy_);
    host_->supply_password(rust::Str(utf8.constData(), static_cast<std::size_t>(utf8.size())));
    applyUpdates();
}

// ---------------------------------------------------------------------------------------------
// The entry point
// ---------------------------------------------------------------------------------------------

std::int32_t run_qt_host(rust::Box<Host> host, std::int32_t quit_after)
{
    // Qt wants an argument vector it can keep for the life of the application, and this host's own
    // arguments were read in Rust — so it is handed a program name and nothing else, which is the
    // same answer `pdf-viewer-gtk` gives GTK (`run_with_args::<&str>(&[])`).
    static char program[] = "pdf-viewer-qt";
    static char* argv[] = {program, nullptr};
    static int argc = 1;
    QApplication app(argc, argv);

    MainWindow window(std::move(host));
    window.show();
    if (quit_after > 0) {
        QTimer::singleShot(quit_after, &app, &QCoreApplication::quit);
    }
    return QApplication::exec();
}

} // namespace pdf_viewer_qt
