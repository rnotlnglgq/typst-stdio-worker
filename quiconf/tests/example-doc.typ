#import "../lib.typ": tag, setup, tag-registry


// Basic tag usage: western + layout + override.
// #show: tag.with("tex-gyre-pagella", "portrait", "lxgw", margin: 12pt)
#show: tag.with("lxgw", "a4", "14pt", "indentfirst")
// #show: tag.with("noto")

= Smoke中文 — tag

中文*加粗*与_强调_行距（`hijack-strong-emph` 默认开）。

Block math:
$ integral_0^1 x dif x $

Inline $a + b$。

`tag` 函数自动分派预设名到对应 `setup` 参数。当前支持的标签：

#tag-registry.keys().join(", ")

也支持长度值、pt值字符串设定`text-size`。
