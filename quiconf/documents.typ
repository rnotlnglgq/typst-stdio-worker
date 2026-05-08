// Document flow / locale / markup defaults (not font families, not page shape).

#let documents = (
  zhcn: (
    text-size: 10.5pt,
    lang: "zh",
    region: "cn",
    par-leading: 0.65em,
    par-spacing: 0.65em,
    first-line-indent: (amount: 2em, all: true),
    hijack-strong-emph: true,
  ),
)

// Tag registry fragment.
#import "_util.typ": _make-tags

#let _doc-tags = _make-tags("document", documents)

