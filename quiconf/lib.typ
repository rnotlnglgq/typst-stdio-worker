// Package entry: re-export presets and `setup` (implementation in sibling modules).

#import "fonts.typ": latin-fonts, cjk-fonts, fontsets
#import "pages.typ": pages
#import "documents.typ": documents
#import "setup.typ": setup
#import "tags.typ": tag, tag-registry

// Public preset tables (same keys as `setup` arguments `western` / `chinese`).
#let western = latin-fonts
#let chinese = cjk-fonts
