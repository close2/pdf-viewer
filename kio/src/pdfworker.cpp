/*
 * pdfworker.cpp — RFC 0003's KIO face.
 *
 * Read `pdfworker.h` first for what this is and is not. Three decisions live in this file and
 * nowhere else, because they are about KIO rather than about PDF:
 *
 * 1. WHICH KIO ERROR A REFUSAL BECOMES, and it is not what RFC 0003 section 5.3 predicted.
 *    The section says "KIO reports refusals as ERR_UNSUPPORTED_ACTION / ERR_WRITE_ACCESS_DENIED
 *    with the sentence" — and KIO does not work that way: for almost every code, the string a
 *    worker returns is a *parameter* substituted into KIO's own canned message, so
 *    `ERR_WRITE_ACCESS_DENIED` with our sentence renders as "Could not write to <two lines about
 *    why a page's text is not a byte stream>". The sentence is the whole point of section 5.3, so
 *    where it carries the reason we use `ERR_WORKER_DEFINED`, whose string KIO shows verbatim.
 *    Four errnos keep a canned code because KIO's own words for them are true and shorter than
 *    ours: ENOENT, EEXIST, EISDIR and ENOTDIR. See `refused` below.
 *
 * 2. A LISTING DOES NOT STAT. RFC 0003 section 5.5: "[d]irectory listings are cheap — names and
 *    types come from the document's structure — and file managers stat lazily, so browsing stays
 *    fast and the cost lands on the first touch of each file". A `listDir` that filled in
 *    UDS_SIZE would render every page of the document to answer `ls`.
 *
 * 3. A COMMIT'S WARNINGS ARE SHOWN, AND WITH `warning()` RATHER THAN `messageBox()`. Deleting a
 *    page always produces one — §7.5.6 leaves the bytes in the file — so a modal box per `rm`
 *    would be a face nobody keeps installed. `warning()` is KIO's non-modal channel and is what
 *    "the operation proceeded, and here is what the document said" wants. The modal channel is
 *    reserved for the question `CLAUDE.md` principle 3's *ask* level would put, which this face
 *    cannot yet put; ADR 0869 says why and what it would cost.
 */

#include "pdfworker.h"

#include <KIO/WorkerFactory>
#include <KPluginFactory>

#include <QCoreApplication>
#include <QFile>
#include <QMimeDatabase>
#include <QMimeType>
#include <QVarLengthArray>

#include <sys/stat.h>

#include <cstdio>
#include <memory>

namespace
{

/*! How much of a virtual file is handed to KIO at a time. */
constexpr qint64 CHUNK = 512 * 1024;

/*! A refusal's sentence, through the header's two-call idiom. */
QString sentenceOf(const pdfvfs_refusal *why)
{
    size_t needed = 0;
    if (pdfvfs_refusal_message(why, nullptr, 0, &needed) != PDFVFS_BUFFER_TOO_SMALL
        || needed == 0) {
        return QStringLiteral("the library would not say why");
    }
    QVarLengthArray<char, 512> room(static_cast<qsizetype>(needed));
    if (pdfvfs_refusal_message(why, room.data(), needed, &needed) != PDFVFS_OK) {
        return QStringLiteral("the library would not say why");
    }
    return QString::fromUtf8(room.data());
}

/*! The MIME type of a name, by its extension. Every name in this tree carries a true one. */
QString typeOf(const QString &name)
{
    QMimeDatabase types;
    const QMimeType found = types.mimeTypeForFile(name, QMimeDatabase::MatchExtension);
    return found.isValid() ? found.name() : QStringLiteral("application/octet-stream");
}

} // namespace

PdfWorker::PdfWorker(const QByteArray &pool, const QByteArray &app)
    : KIO::WorkerBase("pdf", pool, app)
{
    /*
     * The startup check, which is what a C++ caller has in place of a build failure: the header
     * this was compiled against states two numbers, the library answers with two, and a mismatch
     * is refused here rather than acted on. Every operation below fails with that sentence.
     */
    m_agreed = pdfvfs_abi_check(PDFVFS_ABI_VERSION, PDFVFS_ERRNO_KIND_COUNT) == PDFVFS_OK;
    if (!m_agreed) {
        std::fprintf(stderr,
                     "kio_pdf: this plugin was built against pdf_vfs.h version %u with %u errno "
                     "kind(s), and the library it loaded answers %u and %u\n",
                     PDFVFS_ABI_VERSION, PDFVFS_ERRNO_KIND_COUNT, pdfvfs_abi_version(),
                     pdfvfs_errno_kind_count());
    }
}

PdfWorker::~PdfWorker()
{
    pdfvfs_mount_free(m_mount);
}

bool PdfWorker::locate(const QUrl &url, Located &located, KIO::WorkerResult &why)
{
    if (!m_agreed) {
        why = KIO::WorkerResult::fail(
            KIO::ERR_WORKER_DEFINED,
            QStringLiteral("kio_pdf was built against a different revision of the pdf-vfs ABI "
                           "than the library it loaded; it will not serve anything. Run "
                           "`pdfvfs_abi_check` to see which number moved."));
        return false;
    }

    const QByteArray path = url.path().toUtf8();
    size_t document = 0;
    if (pdfvfs_split(path.constData(), &document) != PDFVFS_OK) {
        why = KIO::WorkerResult::fail(KIO::ERR_DOES_NOT_EXIST, url.toDisplayString());
        return false;
    }
    located.document = path.left(static_cast<int>(document));
    located.inside = path.mid(static_cast<int>(document));
    if (located.inside.isEmpty()) {
        located.inside = "/";
    }

    if (m_mount != nullptr && m_document == located.document) {
        return true;
    }
    pdfvfs_mount_free(m_mount);
    m_mount = nullptr;
    m_document.clear();

    pdfvfs_refusal *refusal = nullptr;
    /*
     * `PDFVFS_RESTRICT_OFF`, and it is a decision rather than a default taken by accident:
     * `CLAUDE.md` principle 3 says a document's restrictions are the reader's to set and that it
     * "shall always be possible to turn them off". There is no user interface in this face to
     * choose another level yet, so the level that is the reader's own is the one it takes; the
     * other three are one argument away on the boundary, which is the shape the principle binds.
     */
    if (pdfvfs_mount_open(located.document.constData(), PDFVFS_RESTRICT_OFF, &m_mount, &refusal)
        != PDFVFS_OK) {
        why = refused(refusal, url);
        return false;
    }
    m_document = located.document;
    return true;
}

KIO::WorkerResult PdfWorker::refused(pdfvfs_refusal *why, const QUrl &url)
{
    if (why == nullptr) {
        return KIO::WorkerResult::fail(KIO::ERR_WORKER_DEFINED,
                                       QStringLiteral("the tree refused and said nothing, which "
                                                      "is a defect in pdf-vfs-ffi"));
    }
    int32_t code = 0;
    if (pdfvfs_refusal_errno(why, &code) != PDFVFS_OK) {
        pdfvfs_refusal_free(why);
        return KIO::WorkerResult::fail(KIO::ERR_WORKER_DEFINED,
                                       QStringLiteral("a refusal that would not say its errno"));
    }
    const QString sentence = sentenceOf(why);
    pdfvfs_refusal_free(why);

    /* See decision 1 at the top of this file. The four below keep a canned code because KIO's own
     * words for them are true and shorter than ours; everything else carries its reason. */
    switch (code) {
    case 2: /* ENOENT */
        return KIO::WorkerResult::fail(KIO::ERR_DOES_NOT_EXIST, url.toDisplayString());
    case 17: /* EEXIST */
        return KIO::WorkerResult::fail(KIO::ERR_FILE_ALREADY_EXIST, url.toDisplayString());
    case 20: /* ENOTDIR */
        return KIO::WorkerResult::fail(KIO::ERR_IS_FILE, url.toDisplayString());
    case 21: /* EISDIR */
        return KIO::WorkerResult::fail(KIO::ERR_IS_DIRECTORY, url.toDisplayString());
    default:
        return KIO::WorkerResult::fail(
            KIO::ERR_WORKER_DEFINED,
            QStringLiteral("%1 (%2)").arg(sentence, QString::fromUtf8(pdfvfs_errno_name(code))));
    }
}

void PdfWorker::speak(pdfvfs_commit *commit)
{
    size_t count = 0;
    if (commit == nullptr || pdfvfs_commit_warning_count(commit, &count) != PDFVFS_OK) {
        pdfvfs_commit_free(commit);
        return;
    }
    for (size_t at = 0; at < count; ++at) {
        size_t needed = 0;
        if (pdfvfs_commit_warning(commit, at, nullptr, 0, &needed) != PDFVFS_BUFFER_TOO_SMALL) {
            continue;
        }
        QVarLengthArray<char, 512> room(static_cast<qsizetype>(needed));
        if (pdfvfs_commit_warning(commit, at, room.data(), needed, &needed) == PDFVFS_OK) {
            /* Decision 3 at the top of this file: non-modal, because §7.5.6's note fires on
             * every deletion and a dialogue per `rm` is a face nobody keeps. */
            warning(QString::fromUtf8(room.data()));
        }
    }
    pdfvfs_commit_free(commit);
}

KIO::WorkerResult PdfWorker::listDir(const QUrl &url)
{
    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }

    pdfvfs_listing *listing = nullptr;
    pdfvfs_refusal *refusal = nullptr;
    if (pdfvfs_list(m_mount, located.inside.constData(), &listing, &refusal) != PDFVFS_OK) {
        return refused(refusal, url);
    }
    size_t count = 0;
    if (pdfvfs_listing_len(listing, &count) != PDFVFS_OK) {
        pdfvfs_listing_free(listing);
        return KIO::WorkerResult::fail(KIO::ERR_WORKER_DEFINED,
                                       QStringLiteral("a listing that would not say its length"));
    }
    for (size_t at = 0; at < count; ++at) {
        size_t needed = 0;
        uint32_t kind = PDFVFS_KIND_FILE;
        if (pdfvfs_listing_name(listing, at, nullptr, 0, &needed) != PDFVFS_BUFFER_TOO_SMALL) {
            continue;
        }
        QVarLengthArray<char, 256> room(static_cast<qsizetype>(needed));
        if (pdfvfs_listing_name(listing, at, room.data(), needed, &needed) != PDFVFS_OK
            || pdfvfs_listing_kind(listing, at, &kind) != PDFVFS_OK) {
            continue;
        }
        const QString name = QString::fromUtf8(room.data());
        const bool directory = kind == PDFVFS_KIND_DIRECTORY;

        KIO::UDSEntry entry;
        entry.fastInsert(KIO::UDSEntry::UDS_NAME, name);
        entry.fastInsert(KIO::UDSEntry::UDS_FILE_TYPE, directory ? S_IFDIR : S_IFREG);
        /* Decision 2: no UDS_SIZE here. A listing that stated one would generate every file. */
        entry.fastInsert(KIO::UDSEntry::UDS_ACCESS, directory ? 0555 : 0444);
        if (!directory) {
            entry.fastInsert(KIO::UDSEntry::UDS_MIME_TYPE, typeOf(name));
        }
        listEntry(entry);
    }
    pdfvfs_listing_free(listing);
    return KIO::WorkerResult::pass();
}

KIO::WorkerResult PdfWorker::stat(const QUrl &url)
{
    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }

    pdfvfs_attributes attributes;
    pdfvfs_refusal *refusal = nullptr;
    if (pdfvfs_stat(m_mount, located.inside.constData(), &attributes, &refusal) != PDFVFS_OK) {
        return refused(refusal, url);
    }
    const bool directory = attributes.kind == PDFVFS_KIND_DIRECTORY;

    /*
     * The access bits are the CORE's answer rather than a list this file keeps: `pdfvfs_write_meaning`
     * is the layout table speaking, so what a file manager greys out is the document's own shape.
     */
    uint32_t onWrite = PDFVFS_MEANS_NOTHING;
    uint32_t onDelete = PDFVFS_MEANS_NOTHING;
    const bool writable =
        pdfvfs_write_meaning(m_mount, located.inside.constData(), &onWrite, &onDelete) == PDFVFS_OK
        && (onWrite != PDFVFS_MEANS_NOTHING || onDelete != PDFVFS_MEANS_NOTHING);

    const QString name = QString::fromUtf8(located.inside).section(QLatin1Char('/'), -1);
    KIO::UDSEntry entry;
    entry.fastInsert(KIO::UDSEntry::UDS_NAME, name.isEmpty() ? QStringLiteral(".") : name);
    entry.fastInsert(KIO::UDSEntry::UDS_FILE_TYPE, directory ? S_IFDIR : S_IFREG);
    entry.fastInsert(KIO::UDSEntry::UDS_ACCESS,
                     directory ? (writable ? 0755 : 0555) : (writable ? 0644 : 0444));
    if (attributes.has_size != 0) {
        entry.fastInsert(KIO::UDSEntry::UDS_SIZE, static_cast<long long>(attributes.size));
    }
    if (!directory) {
        entry.fastInsert(KIO::UDSEntry::UDS_MIME_TYPE, typeOf(name));
    }
    statEntry(entry);
    return KIO::WorkerResult::pass();
}

KIO::WorkerResult PdfWorker::mimetype(const QUrl &url)
{
    /*
     * Answered from the name rather than by reading the file. The alternative KIO offers is worse
     * than it looks: a worker that does not implement this has a whole `get` issued instead, so
     * hovering over a 300 dpi render would rasterise it.
     */
    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }
    const QString name = QString::fromUtf8(located.inside).section(QLatin1Char('/'), -1);
    mimeType(name.isEmpty() ? QStringLiteral("inode/directory") : typeOf(name));
    return KIO::WorkerResult::pass();
}

KIO::WorkerResult PdfWorker::get(const QUrl &url)
{
    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }

    pdfvfs_file *file = nullptr;
    pdfvfs_refusal *refusal = nullptr;
    if (pdfvfs_open(m_mount, located.inside.constData(), &file, &refusal) != PDFVFS_OK) {
        return refused(refusal, url);
    }
    uint64_t size = 0;
    if (pdfvfs_file_size(file, &size) != PDFVFS_OK) {
        pdfvfs_file_free(file);
        return KIO::WorkerResult::fail(KIO::ERR_WORKER_DEFINED,
                                       QStringLiteral("an open file that would not say its size"));
    }
    const QString name = QString::fromUtf8(located.inside).section(QLatin1Char('/'), -1);
    mimeType(typeOf(name));
    totalSize(size);

    QByteArray chunk;
    uint64_t at = 0;
    while (at < size) {
        const qint64 wanted = qMin<qint64>(CHUNK, static_cast<qint64>(size - at));
        chunk.resize(wanted);
        size_t filled = 0;
        if (pdfvfs_file_read(file, at, reinterpret_cast<uint8_t *>(chunk.data()),
                             static_cast<size_t>(wanted), &filled)
                != PDFVFS_OK
            || filled == 0) {
            pdfvfs_file_free(file);
            return KIO::WorkerResult::fail(KIO::ERR_CANNOT_READ, url.toDisplayString());
        }
        chunk.resize(static_cast<qsizetype>(filled));
        data(chunk);
        at += filled;
        processedSize(at);
    }
    pdfvfs_file_free(file);
    /* An empty block is how KIO is told the data has ended. */
    data(QByteArray());
    return KIO::WorkerResult::pass();
}

KIO::WorkerResult PdfWorker::put(const QUrl &url, int permissions, KIO::JobFlags flags)
{
    Q_UNUSED(permissions)
    Q_UNUSED(flags)

    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }

    /*
     * The whole file is collected before anything is written, which is RFC 0003 section 5.4's
     * commit point for this face: "a KIO `put` commits when the worker's `put` completes (KIO's
     * verb is already transactional)". The staged four a kernel needs are not on the boundary at
     * all, which is why this is the only place a transaction is assembled.
     */
    QByteArray staged;
    QByteArray chunk;
    int read = 0;
    do {
        dataReq();
        read = readData(chunk);
        if (read > 0) {
            staged.append(chunk);
        }
    } while (read > 0);
    if (read < 0) {
        return KIO::WorkerResult::fail(KIO::ERR_CANNOT_READ, url.toDisplayString());
    }

    pdfvfs_commit *commit = nullptr;
    pdfvfs_refusal *refusal = nullptr;
    if (pdfvfs_write(m_mount, located.inside.constData(),
                     reinterpret_cast<const uint8_t *>(staged.constData()),
                     static_cast<size_t>(staged.size()), &commit, &refusal)
        != PDFVFS_OK) {
        return refused(refusal, url);
    }
    speak(commit);
    return KIO::WorkerResult::pass();
}

KIO::WorkerResult PdfWorker::del(const QUrl &url, bool isfile)
{
    Q_UNUSED(isfile)

    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }
    pdfvfs_commit *commit = nullptr;
    pdfvfs_refusal *refusal = nullptr;
    if (pdfvfs_remove(m_mount, located.inside.constData(), &commit, &refusal) != PDFVFS_OK) {
        return refused(refusal, url);
    }
    speak(commit);
    return KIO::WorkerResult::pass();
}

KIO::WorkerResult PdfWorker::mkdir(const QUrl &url, int permissions)
{
    Q_UNUSED(permissions)

    Located located;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(url, located, why)) {
        return why;
    }
    pdfvfs_refusal *refusal = nullptr;
    pdfvfs_create_directory(m_mount, located.inside.constData(), &refusal);
    return refused(refusal, url);
}

KIO::WorkerResult PdfWorker::rename(const QUrl &src, const QUrl &dest, KIO::JobFlags flags)
{
    Q_UNUSED(flags)

    Located from;
    KIO::WorkerResult why = KIO::WorkerResult::pass();
    if (!locate(src, from, why)) {
        return why;
    }
    /* A rename out of this tree is a copy and a delete, which KIO does for us; a rename *within*
     * it is what section 5.3 refuses, and the core is asked either way so the sentence is one.
     * Where the destination is inside a document too, its own inside path is what the sentence
     * names — a `mv 0007 0002` should read as the two ordinals it is. */
    QByteArray target = dest.path().toUtf8();
    size_t document = 0;
    if (pdfvfs_split(target.constData(), &document) == PDFVFS_OK) {
        target = target.mid(static_cast<int>(document));
        if (target.isEmpty()) {
            target = "/";
        }
    }
    pdfvfs_refusal *refusal = nullptr;
    pdfvfs_rename(m_mount, from.inside.constData(), target.constData(), &refusal);
    return refused(refusal, src);
}

KIO::WorkerResult PdfWorker::symlink(const QString &target, const QUrl &dest, KIO::JobFlags flags)
{
    Q_UNUSED(target)
    Q_UNUSED(flags)
    Q_UNUSED(dest)
    /* The layout has no symbolic links and inventing one would be a name that is not the
     * document's. Stated rather than inherited: `WorkerBase`'s own default is ERR_UNSUPPORTED_ACTION
     * with no reason at all. */
    return KIO::WorkerResult::fail(
        KIO::ERR_WORKER_DEFINED,
        QStringLiteral("this tree has no symbolic links: every name in it is a position in the "
                       "document, and a link would name something the file does not say"));
}

KIO::WorkerResult PdfWorker::chmod(const QUrl &url, int permissions)
{
    Q_UNUSED(url)
    Q_UNUSED(permissions)
    return KIO::WorkerResult::fail(
        KIO::ERR_WORKER_DEFINED,
        QStringLiteral("the access bits in this tree are the document's own shape — what a write "
                       "to a name would mean — and are not a property anyone can set"));
}

/*
 * The two entry points KIO uses. `kioworker` loads this module with QPluginLoader and resolves
 * `kdemain`; the plugin metadata beside it is what `KProtocolInfo` reads to know that `pdf:` is
 * served here at all, and it is embedded by the factory macro below.
 */
class PdfWorkerFactory : public KIO::WorkerFactory
{
    Q_OBJECT
public:
    explicit PdfWorkerFactory(QObject *parent = nullptr)
        : KIO::WorkerFactory(parent)
    {
    }

    std::unique_ptr<KIO::WorkerBase> createWorker(const QByteArray &pool,
                                                  const QByteArray &app) override
    {
        return std::make_unique<PdfWorker>(pool, app);
    }
};

K_PLUGIN_CLASS_WITH_JSON(PdfWorkerFactory, "pdf.json")

extern "C" int Q_DECL_EXPORT kdemain(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("kio_pdf"));

    if (argc != 4) {
        std::fprintf(stderr, "Usage: kio_pdf protocol domain-socket1 domain-socket2\n");
        return -1;
    }

    PdfWorker worker(argv[2], argv[3]);
    worker.dispatchLoop();
    return 0;
}

#include "pdfworker.moc"
