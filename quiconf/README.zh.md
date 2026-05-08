# Quiconf

[English](README.md) | [中文](README.zh.md)

本Typst项目提供快速设定页面、字体等的函数`setup,tag`。

## 基本用法

引入（如果已在本地仓库路径中）
```typst
#import "@local/quiconf:0.0.1": tag, setup
```

使用`show`和`with`处理文档对象，选项支持用键值对设定：
```typst
#show: setup.with(
  western: "lm",
  chinese: "noto"
)
```
考虑到字体预设名等值经常可以是唯一的，也支持只用值名：
```typst
#show: tag.with("noto", 14pt, "indentfirst", "a4")
```

标签列表：
```
lm, noto, lxgw, wauto, w20, w30, w40, a4, a4h, a5, a5h, zhcn, indentfirst
```
也支持长度值、pt值字符串设定`text-size`。


## 可设定范围
目前主要考虑：
* 页面
* 字体