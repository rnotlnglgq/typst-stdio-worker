#import "fonts.typ": latin-fonts, cjk-fonts, _merge-presets
#import "pages.typ": pages
#import "documents.typ": documents

// `#show: setup.with(layout: "oneline", document: "zhcn")`.
// Fonts: `western` and `chinese` select presets from `latin-fonts` / `cjk-fonts` independently.
// `equation-font: none` → use the active Western preset's `math` (do not name this `math`,
// or it shadows Typst's `math` module and breaks `math.equation` show rules).
// `layout` names an entry in `pages` (avoid shadowing Typst's `page` function).
// Optional overrides (none = use resolved preset): margin, text-size, leading,
// par-spacing, first-line-indent, hijack-strong-emph.
#let setup(
  body,
  western: "lm",
  chinese: "noto",
  equation-font: none,
  layout: "wauto",
  document: "zhcn",
  margin: 2em,
  text-size: 14pt,
  first-line-indent: false,
  leading: none,
  par-spacing: none,
  hijack-strong-emph: true,
) = {
  assert(
    western in latin-fonts,
    message: "quiconf.setup: unknown western preset `" + western + "`",
  )
  assert(
    chinese in cjk-fonts,
    message: "quiconf.setup: unknown chinese preset `" + chinese + "`",
  )
  assert(layout in pages, message: "quiconf.setup: unknown layout `" + layout + "`")
  assert(
    document in documents,
    message: "quiconf.setup: unknown document preset `" + document + "`",
  )

  let w = latin-fonts.at(western)
  let c = cjk-fonts.at(chinese)
  let merged = _merge-presets(w, c)
  let math-face = if equation-font != none { equation-font } else { w.math }

  let pg = pages.at(layout)
  let doc = documents.at(document)

  let m = if margin != none { margin } else { pg.margin }
  let sz = if text-size != none { text-size } else { doc.text-size }
  let ld = if leading != none { leading } else { doc.par-leading }
  let ps = if par-spacing != none { par-spacing } else { doc.par-spacing }
  let fli = if first-line-indent {
    doc.first-line-indent
  } else {
    (amount: 0em, all: true)
  }
  let hijack = if hijack-strong-emph != none {
    hijack-strong-emph
  } else {
    doc.hijack-strong-emph
  }

  let body = if "width" in pg {
    set page(width: pg.width, height: pg.height, margin: m)
    body
  } else {
    set page(paper: pg.paper, flipped: pg.flipped, margin: m)
    body
  }

  set text(
    font: merged.regular,
    size: sz,
    lang: doc.lang,
    region: doc.region,
  )
  show math.equation: set text(font: math-face)
  show math.equation.where(block: true): set block(above: 1em, below: 1em)
  set par(
    justify: true,
    first-line-indent: fli,
    leading: ld,
    spacing: ps,
  )
  if hijack {
    show strong: set text(font: merged.strong)
    show emph: set text(font: merged.emph)
    body
  } else {
    body
  }
}
