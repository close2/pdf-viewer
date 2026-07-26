//! Document and page-tree model built over parsed PDF objects.
//!
//! Gives meaning to the object graph from `pdf-syntax`: the catalogue, the page
//! tree with attribute inheritance, resource dictionaries, outlines, and named
//! destinations. Validation of object shapes is delegated to `pdf-spec`.
//!
//! Page access is lazy by design. Time-to-first-page is the metric a user
//! perceives, and eagerly walking a page tree of several thousand nodes to open a
//! document is the single most common reason viewers feel slow to start. Nothing
//! here may require a full-document traversal to render page one.
//!
//! Implemented after Phase 5.

#![forbid(unsafe_code)]
