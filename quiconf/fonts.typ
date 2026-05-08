// Western / CJK font presets.

// Normalize a font slot (string or array) to an array.
#let _font-tuple(x) = {
  if type(x) == str {
    (x,)
  } else if type(x) == array {
    x
  } else {
    assert(
      false,
      message: "quiconf: font slot expects string or array, got " + str(type(x)),
    )
  }
}

// Merge a western preset and a CJK preset into a combined (regular, strong, emph) triple.
#let _merge-presets(w, c) = (
  regular: _font-tuple(w.regular) + _font-tuple(c.regular),
  strong: _font-tuple(w.strong) + _font-tuple(c.strong),
  emph: _font-tuple(w.emph) + _font-tuple(c.emph),
)

// Latin / Western presets: `regular` / `strong` / `emph` + `math`.
#let latin-fonts = (
  lm: (
    regular: "Latin Modern Roman",
    strong: "Latin Modern Roman",
    emph: "Latin Modern Roman",
    math: "Latin Modern Math",
  ),
  // "tex-gyre-pagella": (
  //   regular: "TeX Gyre Pagella",
  //   strong: "TeX Gyre Pagella",
  //   emph: "TeX Gyre Pagella",
  //   math: "TeX Gyre Pagella Math",
  // ),
)

// Chinese (CJK) presets: `regular` / `strong` / `emph` — merged *after* the Western font.
#let cjk-fonts = (
  noto: (
    regular: "Noto Serif CJK SC",
    strong: "Noto Sans CJK SC",
    emph: "Noto Serif CJK SC",
  ),
  lxgw: (
    regular: "LXGW Neo ZhiSong Plus",
    strong: "LXGW Neo XiHei Plus",
    emph: "LXGW WenKai GB",
  ),
)

// Tag registry fragments — keys come directly from the preset dictionaries.
#import "_util.typ": _make-tags

#let _latin-tags = _make-tags("western", latin-fonts)
#let _cjk-tags = _make-tags("chinese", cjk-fonts)

// Back-compat: merged stacks for default Latin + default CJK (legacy `fontsets.default`).
#let _default-merged = _merge-presets(latin-fonts.lm, cjk-fonts.noto)
#let fontsets = (
  default: (
    body-regular: _default-merged.regular,
    body-bold: _default-merged.strong,
    body-emph: _default-merged.emph,
    math: latin-fonts.lm.math,
  ),
)
