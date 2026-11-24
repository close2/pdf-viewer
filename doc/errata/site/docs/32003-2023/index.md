---
isodoc: ISO TS 32003:2023
isotitle: 'Document management — Portable Document Format — Adding support of AES-GCM in PDF 2.0'
title: 'ISO TS 32003: AES-GCM in PDF 2.0'
last_modified_date: 18 November 2026
nav_order: 3
has_toc: false
search_exclude: true
---

<h1>{{ page.isodoc }}: <i>{{ page.isotitle }}</i></h1>

<p>
This document lists all industry-approved corrections to <a href="https://www.pdfa.org/resource/iso-ts-32003/" target="_blank">"{{ page.isodoc }} <i>{{ page.isotitle }}</i>"</a>.
All issues related to {{ page.isodoc }} are recorded in the PDF Association's <a href="https://github.com/pdf-association/pdf-issues" target="_blank">GitHub pdf-issues</a>
with the label '<a href="https://github.com/pdf-association/pdf-issues/issues?q=is%3Aissue+label%3A%22ISO%2FTS+32003%22" target="_blank">ISO/TS 32003</a>'.
</p>

<p>Corrections are organized by the top level clauses in {{ page.isodoc }}:</p>

<nav>
<ul>
    {% for std in site.data.data.toc %}
         {% if std.standard == page.isodoc %}
            {% for pg in std.subfolderitems %}
            <li><a href="{{ pg.url }}">{{ pg.ref }}</a></li>
           {% endfor %}
        {% endif %}
    {% endfor %}
</ul>
</nav>

<p>The following tables have all been corrected:</p>

<nav>
<ul>
</ul>
</nav>
