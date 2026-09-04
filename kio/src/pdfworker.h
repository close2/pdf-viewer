/*
 * pdfworker.h — RFC 0003's KIO face, declared.
 *
 * A `KIO::WorkerBase` subclass and nothing else. Every question it is asked is forwarded over
 * `pdf_vfs.h`'s C ABI into `pdf-vfs`, and every answer is that library's turned into Qt's types.
 * There is no PDF logic here, no layout knowledge, and not one `errno` this file chooses: RFC
 * 0003 section 7 puts all of that in the core so that "adding `fonts/` one day is a core change
 * that both faces grow simultaneously".
 *
 * The division the RFC draws is kept literally in the other direction too: **this file owns the
 * Qt types and the core never sees them.** `QUrl`, `UDSEntry` and `KIO::WorkerResult` stop here.
 */

#pragma once

#include <KIO/WorkerBase>

#include <QByteArray>
#include <QString>
#include <QUrl>

extern "C" {
#include "pdf_vfs.h"
}

/*!
 * One PDF, served as a folder.
 *
 * A worker outlives one operation — KIO keeps it in a pool — so the mount is kept between calls:
 * that is what keeps the confined generator alive and the core's cache warm across a `cp -r`.
 * The core validates the document's generation key before every answer, so a cached mount cannot
 * serve a stale one.
 */
class PdfWorker : public KIO::WorkerBase
{
public:
    PdfWorker(const QByteArray &pool, const QByteArray &app);
    ~PdfWorker() override;

    KIO::WorkerResult listDir(const QUrl &url) override;
    KIO::WorkerResult stat(const QUrl &url) override;
    KIO::WorkerResult get(const QUrl &url) override;
    KIO::WorkerResult mimetype(const QUrl &url) override;

    /* RFC 0003 section 5.2's write verbs, as KIO spells them. */
    KIO::WorkerResult put(const QUrl &url, int permissions, KIO::JobFlags flags) override;
    KIO::WorkerResult del(const QUrl &url, bool isfile) override;

    /* RFC 0003 section 5.3's refusals. Each one asks the core, so the sentence a person reads is
     * the core's rather than this file's — a face that worded its own would be a second copy of
     * a decision, and the two would drift. */
    KIO::WorkerResult mkdir(const QUrl &url, int permissions) override;
    KIO::WorkerResult rename(const QUrl &src, const QUrl &dest, KIO::JobFlags flags) override;
    KIO::WorkerResult symlink(const QString &target, const QUrl &dest, KIO::JobFlags flags) override;
    KIO::WorkerResult chmod(const QUrl &url, int permissions) override;

private:
    /*! A URL split into the document to open and the path inside it. */
    struct Located {
        QByteArray document;
        QByteArray inside;
    };

    /*!
     * Splits the URL and opens (or reuses) the mount behind it.
     *
     * Answers the path inside on success. On failure `why` holds the result to return; KIO's
     * `WorkerResult` has no empty state, which is why it is an out-parameter rather than a
     * return value.
     */
    bool locate(const QUrl &url, Located &located, KIO::WorkerResult &why);

    /*! Turns a refusal into KIO's vocabulary, and frees it. See the definition for the choice. */
    KIO::WorkerResult refused(pdfvfs_refusal *why, const QUrl &url);

    /*! Shows a commit's warnings, and frees the commit. */
    void speak(pdfvfs_commit *commit);

    /*! The document currently open, and the mount over it. */
    QByteArray m_document;
    pdfvfs_mount *m_mount = nullptr;
    /*! Whether the header this was compiled against and the library agree. Checked once. */
    bool m_agreed = false;
};
