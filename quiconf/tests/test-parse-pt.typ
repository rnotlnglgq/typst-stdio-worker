#import "../_util.typ": _parse-pt-length

#assert.eq(_parse-pt-length("12pt"), 12pt)
#assert.eq(_parse-pt-length("0.5pt"), 0.5pt)
#assert.eq(_parse-pt-length("-3pt"), -3pt)
#assert.eq(_parse-pt-length(".25pt"), 0.25pt)
#assert.eq(_parse-pt-length("12"), none)
#assert.eq(_parse-pt-length("12px"), none)
#assert.eq(_parse-pt-length("abc"), none)
#assert.eq(_parse-pt-length("12 pt"), none)
