# Typst Stdio Worker

语言: [English](README.md) · **简体中文**

本项目主要基于 Rust `typst` crate，在 STDIO 以 NDJSON 协议接受排版任务请求，并以 BASE64 in JSON 回应任务结果。主要目的是隔离本进程以让调用者使用操作系统的配额管理。

## Worker NDJSON 协议

通信为 **按行 NDJSON**（每行一个 JSON）。字体加载完成后，worker 先在 stdout 输出一行 **ready**：

```json
{"ready":true,"protocol_version":1,"version":"0.1.0","fonts_loaded":2464}
```

父进程向 stdin **逐行写请求**。必填 `source`；可选 `id`、`template`（`raw`/`default`）、`scale`、`max_pages`、`format`（`png`）、`stitch` 等，完整字段表见 [docs/NOTES.md](docs/NOTES.md)。

请求示例：

```json
{"id":"req-1","source":"Hello","template":"default","scale":2.0,"max_pages":50,"format":"png","stitch":true}
```

每条 **响应** 占 stdout 一行。成功（可有 `id`、`warnings`）：

```json
{"id":"req-1","ok":true,"format":"png","data":"<base64 PNG>","pages":3}
```

失败（`errors[]` 含 `kind`、可选 `span`、`hints`）：

```json
{"id":"req-1","ok":false,"errors":[{"kind":"compile","message":"…","span":{"line":3,"column":5},"hints":[]}]}
```

日志仅在 stderr。

## 主要细节
主要实现了 `typst::World` trait，具备 typst 要求的基本的缓存功能，并提供一定带有 i18n 的日志记录功能。模板应用仍在开发，目前主要依赖于拼接字面量，更多逻辑下放于本地 typst 包中完成。默认不支持网络下载 typst 包，可通过选项 `--allow-download` 启用下载，推荐用法为服务提供者提前开启 `--allow-download` 选项缓存好 `@preview/` 命名空间的包，然后提供给用户服务时关闭该选项。
字体读取使用 `unsafe` 的 `memmap2` 以实现并发实例的内存共享，最好拷贝副本并使用 Linux ACL 避免字体文件被意外修改。

## 环境变量

| 变量 | 作用 |
| --- | --- |
| `TSW_LANG` | 强制 CLI/帮助语言：`zh`、`zh_cn`、`zh_tw`、`chinese` 等为中文，其余为英文；优先于 `LANG`。 |
| `LANG` | 未设置 `TSW_LANG` 时，若 `LANG` 以 `zh` 开头则使用中文帮助与文案，否则为英文。 |
| `TSW_RESOURCES` | 内置文本资源（`resources/` 下 prelude 等）的根目录；可为绝对路径，或相对于进程当前工作目录。 |
| `TYPST_PACKAGE_PATH` | `--package-path` 的默认值（本地 `@local/` 包搜索根）。命令行与之一同指定时以命令行为准。 |
| `TYPST_PACKAGE_CACHE_PATH` | `--package-cache-path` 的默认值（注册表包缓存）。命令行优先。 |
| `XDG_CACHE_HOME` | 未设置包缓存路径时，回退到 `$XDG_CACHE_HOME/typst/packages`。 |
| `HOME` | 若 `XDG_CACHE_HOME` 也未设置，则回退到 `$HOME/.cache/typst/packages`。 |
| `RUST_LOG` | 若设为合法的 `tracing-subscriber` 过滤表达式，则覆盖 `--log-level` 的默认过滤（与 tracing 生态中的 `EnvFilter` 行为一致）。 |

交互模式下的预览 PNG 等临时文件路径遵循系统临时目录规则（Unix 上常见为 `TMPDIR` 等）。

## 命令行参数一览
```
Typst 编译工作进程 — 通过管道、交互式 REPL 或长连接 NDJSON 协议将 Typst 源码渲染为 PNG。

Usage: typst-stdio-worker [OPTIONS] <COMMAND>

Commands:
  worker       长连接 NDJSON 工作模式（stdin/stdout 协议）
  interactive  交互式 REPL：输入源码，按回车编译预览
  pipe         单次管道模式：读取 stdin，编译后输出 PNG 到 stdout
  help         Print this message or the help of the given subcommand(s)

Options:
  -f, --font-path <FONT_PATH>
          额外字体目录
  -l, --package-path <DIR>
          本地包搜索根目录（TYPST_PACKAGE_PATH；与 typst compile --package-path 相同）。命令行优先于环境变量。先于包缓存根目录解析。 [env: TYPST_PACKAGE_PATH=]
  -p, --package-cache-path <DIR>
          包缓存目录（TYPST_PACKAGE_CACHE_PATH；与 typst compile --package-cache-path 相同）。未设置时使用 $XDG_CACHE_HOME/typst/packages 或 ~/.cache/typst/packages。 [env: TYPST_PACKAGE_CACHE_PATH=]
      --allow-download
          允许从 Typst 包注册表下载缺失的包
  -s, --scale <SCALE>
          PNG 像素缩放因子 [default: 4.0]
      --max-pages <MAX_PAGES>
          最大渲染页数 [default: 1]
      --max-pixels <MAX_PIXELS>
          最大总像素数（所有页面宽×高之和） [default: 100000000]
      --max-input-size <MAX_INPUT_SIZE>
          最大输入大小（字节） [default: 1048576]
      --log-level <LOG_LEVEL>
          日志级别 [default: info]
      --meter
          记录去重后的字体文件总字节数，以及每次渲染 PNG 的尺寸与编码体积
  -h, --help
          Print help
  -V, --version
          Print version
```
