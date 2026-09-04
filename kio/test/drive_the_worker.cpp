/*
 * drive_the_worker.cpp — the plugin, driven through real KIO, with no KDE session.
 *
 * WHY THIS EXISTS. Every other instrument in this round tests something *beside* the worker: the
 * Rust tests test the core, the C driver tests the ABI, and the header check tests the two
 * against each other. None of them loads `pdf.so`, and none of them makes KIO fork `kioworker`,
 * read the plugin's metadata, decide that `pdf:` is served here, and send it a command over a
 * socket. This program does exactly that — it is a KIO *client*, running the same jobs Dolphin
 * runs — so what it exercises is the real plugin through the real protocol.
 *
 * WHAT IT IS NOT. It is not Dolphin. It says nothing about how a file manager renders a listing,
 * about the `archiveMimetype` association that makes a click on a PDF enter it as a folder, or
 * about anything a person sees. Those need a session; this needs a QCoreApplication.
 *
 * Usage: drive_the_worker <document.pdf> <scratch-copy.pdf>
 * with QT_PLUGIN_PATH naming a directory whose kf6/kio/ holds pdf.so.
 */

#include <KIO/DeleteJob>
#include <KIO/Job>
#include <KIO/ListJob>
#include <KIO/MkdirJob>
#include <KIO/SimpleJob>
#include <KIO/StatJob>
#include <KIO/StoredTransferJob>

#include <QCoreApplication>
#include <QStringList>
#include <QTextStream>
#include <QUrl>

namespace
{

QTextStream &out()
{
    static QTextStream stream(stdout);
    return stream;
}

/*! `pdf:` plus a document's path plus a path inside it. */
QUrl at(const QString &document, const QString &inside)
{
    QUrl url;
    url.setScheme(QStringLiteral("pdf"));
    url.setPath(document + inside);
    return url;
}

/*!
 * Every name a `listDir` produced, in the order the worker sent them.
 *
 * The "." KIO puts at the end of every listing is dropped here. It is **KIO's own** and not the
 * worker's — `pdfvfs_list` never answers it, and the worker sends exactly what the core says —
 * so counting it would make every number here one larger than the document's.
 */
QStringList listing(const QUrl &url, QString &why)
{
    QStringList names;
    KIO::ListJob *job = KIO::listDir(url, KIO::HideProgressInfo);
    QObject::connect(job, &KIO::ListJob::entries, job,
                     [&names](KIO::Job *, const KIO::UDSEntryList &entries) {
                         for (const KIO::UDSEntry &entry : entries) {
                             const QString name = entry.stringValue(KIO::UDSEntry::UDS_NAME);
                             if (name != QLatin1String(".")) {
                                 names << name;
                             }
                         }
                     });
    if (!job->exec()) {
        why = job->errorString();
    }
    return names;
}

} // namespace

int main(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    if (argc != 3) {
        out() << "usage: drive_the_worker <document.pdf> <scratch-copy.pdf>\n";
        return 2;
    }
    const QString document = QString::fromLocal8Bit(argv[1]);
    const QString scratch = QString::fromLocal8Bit(argv[2]);

    /* 1. The root of the tree, which RFC 0003 section 4 says has six directories in it. */
    QString why;
    QStringList root = listing(at(document, QString()), why);
    if (!why.isEmpty()) {
        out() << "root refused: " << why << "\n";
        out().flush();
        return 1;
    }
    out() << "root: " << root.join(QLatin1Char(' ')) << "\n";

    /* 2. One page per page. */
    QStringList pages = listing(at(document, QStringLiteral("/pages")), why);
    if (!why.isEmpty()) {
        out() << "pages refused: " << why << "\n";
        out().flush();
        return 1;
    }
    out() << "pages: " << pages.join(QLatin1Char(' ')) << "\n";

    /* 3. A stat, whose size RFC 0003 section 5.5 requires to be the file's own. */
    const QUrl first = at(document, QStringLiteral("/pages/0001.pdf"));
    KIO::StatJob *stat = KIO::stat(first, KIO::HideProgressInfo);
    if (!stat->exec()) {
        out() << "stat refused: " << stat->errorString() << "\n";
        out().flush();
        return 1;
    }
    const KIO::UDSEntry found = stat->statResult();
    const qint64 stated = found.numberValue(KIO::UDSEntry::UDS_SIZE, -1);
    out() << "stat: " << found.stringValue(KIO::UDSEntry::UDS_NAME) << ", directory "
          << (found.isDir() ? 1 : 0) << ", " << stated << " byte(s), type "
          << found.stringValue(KIO::UDSEntry::UDS_MIME_TYPE) << "\n";

    /* 4. And a get, whose bytes are the same length the stat stated. `cp` IS page extraction. */
    KIO::StoredTransferJob *got = KIO::storedGet(first, KIO::NoReload, KIO::HideProgressInfo);
    if (!got->exec()) {
        out() << "get refused: " << got->errorString() << "\n";
        out().flush();
        return 1;
    }
    const QByteArray page = got->data();
    out() << "get: " << page.size() << " byte(s), beginning " << QString::fromLatin1(page.left(5))
          << ", agreeing with the stat " << (page.size() == stated ? 1 : 0) << "\n";

    /* 5-7. Section 5.3's refusals, each reaching a person as its own sentence. */
    KIO::SimpleJob *made = KIO::mkdir(at(document, QStringLiteral("/fonts")));
    made->exec();
    out() << "mkdir: " << made->errorString() << "\n";

    KIO::SimpleJob *moved = KIO::rename(first, at(document, QStringLiteral("/pages/0002.pdf")),
                                        KIO::Overwrite);
    moved->exec();
    out() << "rename: " << moved->errorString() << "\n";

    KIO::StoredTransferJob *typed =
        KIO::storedPut(QByteArray("nothing"), at(document, QStringLiteral("/text/0001.txt")), -1,
                       KIO::Overwrite | KIO::HideProgressInfo);
    typed->exec();
    out() << "put into text/: " << typed->errorString() << "\n";

    /* 8-11. Section 5.2's verbs, over the scratch copy, through KIO's own put and del. */
    KIO::SimpleJob *deleted =
        KIO::file_delete(at(scratch, QStringLiteral("/pages/0005.pdf")), KIO::HideProgressInfo);
    /*
     * `CLAUDE.md` principle 3's *warn* channel, end to end. A deletion always produces one —
     * §7.5.6 leaves the deleted object's bytes in the file — and the worker sends it with KIO's
     * non-modal `warning()`, which arrives here as `KJob::warning`. Printing it is what makes
     * this a demonstration that the channel exists rather than a claim that it does.
     */
    QObject::connect(deleted, &KJob::warning, deleted, [](KJob *, const QString &said) {
        out() << "the document said: " << said.left(60) << "\n";
    });
    if (!deleted->exec()) {
        out() << "delete refused: " << deleted->errorString() << "\n";
        out().flush();
        return 1;
    }
    QStringList after = listing(at(scratch, QStringLiteral("/pages")), why);
    out() << "after the delete: " << after.size() << " page(s)\n";

    KIO::StoredTransferJob *inserted =
        KIO::storedPut(page, at(scratch, QStringLiteral("/pages/0001.pdf")), -1,
                       KIO::Overwrite | KIO::HideProgressInfo);
    if (!inserted->exec()) {
        out() << "insert refused: " << inserted->errorString() << "\n";
        out().flush();
        return 1;
    }
    after = listing(at(scratch, QStringLiteral("/pages")), why);
    out() << "after the insert: " << after.size() << " page(s)\n";

    out() << "ok\n";
    out().flush();
    return 0;
}
