// Build tag registry entries from a preset dictionary.
// Each key in `presets` becomes a tag that maps to `(param: param, key: key)`.

// If `s` is a string of the form `<number>pt` (optional sign; integer or decimal), return that length; else `none`.
#let _parse-pt-length(s) = {
  let m = str(s).match(regex("^(-?(?:\\d+(?:\\.\\d*)?|\\.\\d+))pt$"))
  if m == none {
    return none
  }
  float(m.captures.at(0)) * 1pt
}

#let _make-tags(param, presets) = {
  let result = (:)
  for key in presets.keys() {
    result.insert(key, (param: param, key: key))
  }
  result
}
