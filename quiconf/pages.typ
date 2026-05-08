// Page geometry presets: `oneline` (auto-sized canvas), fixed `paper` + `flipped`.

#let pages = (
  wauto: (
    width: auto,
    height: auto,
    margin: 1em,
  ),
  w20: (
    width: 20em,
    height: auto,
    margin: 1em,
  ),
  w30: (
    width: 20em,
    height: auto,
    margin: 1em,
  ),
  w40: (
    width: 40em,
    height: auto,
    margin: 1em,
  ),
  a4: (
    paper: "a4",
    flipped: false,
  ),
  a4h: (
    paper: "a4",
    flipped: true,
  ),
  a5: (
    paper: "a5",
    flipped: false,
  ),
  a5h: (
    paper: "a5",
    flipped: true,
  ),
)

// Tag registry fragment.
#import "_util.typ": _make-tags

#let _page-tags = _make-tags("layout", pages)
