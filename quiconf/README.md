# Quiconf

[English](README.md) | [中文](README.zh.md)

This Typst package provides functions `setup, tag` for quickly configuring pages, fonts, and more.

## Basic Usage

Import (if already in the local repository path):
```typst
#import "@local/quiconf:0.0.1": tag, setup
```

Use `show` and `with` to process the document object. Options can be set via key-value pairs:
```typst
#show: setup.with(
  western: "lm",
  chinese: "noto"
)
```
Since values like font preset names are often unique, you can also pass values directly:
```typst
#show: tag.with("noto", 14pt, "indentfirst", "a4")
```

Tag list:
```
lm, noto, lxgw, wauto, w20, w30, w40, a4, a4h, a5, a5h, zhcn, indentfirst
```
Length values and pt-value strings are also supported for setting `text-size`.


## Configurable Scope
Currently focused on:
* Pages
* Fonts
