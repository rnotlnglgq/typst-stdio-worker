// Tag dispatch layer: assemble per-module tag fragments into a global registry,
// then provide `tag` as syntactic sugar for `setup`.

#import "fonts.typ": _latin-tags, _cjk-tags
#import "pages.typ": _page-tags
#import "documents.typ": _doc-tags
#import "setup.typ": setup
#import "tag-alias.typ": _tag-aliases
#import "_util.typ": _parse-pt-length

// Global registry: tag-name → (param, key).  Assembled from per-module fragments.
#let tag-registry = _latin-tags + _cjk-tags + _page-tags + _doc-tags + _tag-aliases

// `#show: tag.with("pagella", "portrait", margin: 12pt)`
// Positional strings are tag names dispatched to setup params; named args pass through.
#let tag(..args) = {
  let pos = args.pos()
  let named = args.named()
  assert(pos.len() >= 1, message: "quiconf.tag: missing body (use as show rule or pass content)")
  let body = pos.last()
  let tag-names = pos.slice(0, -1)

  let setup-args = (:)
  for t in tag-names {
    if type(t) == length {
      setup-args.insert("text-size", t)
      continue
    }
    assert(type(t) == str, message: "quiconf.tag: expected string, got " + str(type(t)))
    let parsed-length = _parse-pt-length(t)
    if parsed-length != none {
      setup-args.insert("text-size", parsed-length)
      continue
    }
    assert(t in tag-registry, message: "quiconf.tag: unknown tag `" + t + "`")
    let entry = tag-registry.at(t)
    let param = entry.param
    assert(
      param not in setup-args,
      message: "quiconf.tag: duplicate " + param + " — `" + t + "` conflicts",
    )
    setup-args.insert(param, entry.key)
  }

  for (k, v) in named {
    setup-args.insert(k, v)
  }

  setup(body, ..setup-args)
}
