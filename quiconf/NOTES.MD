# quiconf — 项目简记

Typst 包（`typst.toml` → `lib.typ`）。目标：用 **包 + 预设 API** 替代外部注入的 prelude；面向 **中西混排**，西文字体列表始终在 CJK 列表之前拼接。

## 公共 API（`lib.typ` 导出）

| 名称 | 类型 | 说明 |
|------|------|------|
| `setup` | 函数 | 主配置入口，`#show: setup.with(...)` |
| `tag` | 函数 | `setup` 的语法糖，按 tag 名自动分派到对应参数 |
| `tag-registry` | 字典 | 全局 tag 注册表（tag 名 → `(param, key)`） |
| `western` | 字典 | 西文字体预设表（= `latin-fonts`） |
| `chinese` | 字典 | 中文字体预设表（= `cjk-fonts`） |
| `pages` | 字典 | 页面几何预设 |
| `documents` | 字典 | 文档流/locale 预设 |
| `fontsets` | 字典 | 旧兼容：`default` 合并栈；**新代码应使用 `western` + `chinese`** |

## 核心设计原则

### 预设独立、组合动态

- **西文**（`latin-fonts`）与**中文**（`cjk-fonts`）是两套独立预设字典。
- 各预设含 `regular` / `strong` / `emph` 三字段（字符串或数组）；西文另含 `math`。
- **不存在硬编码的中西文组合预设**。组合始终由用户通过 `setup(western: "..", chinese: "..")` 动态选择，`_merge-presets` 按槽合并为 `(西…, 中…)` 三元组。

### Tag = 预设键名

- 每个预设字典的键本身就是 tag 名，**全局唯一**，不需要额外别名层。
- `_make-tags(param, presets)` 直接遍历字典键生成注册表条目。
- 添加新预设时只需在对应文件加一个键值对，tag 自动生成。

### 参数命名避免遮蔽

- 公式字体参数名 **`equation-font`**（禁 `math`，会遮蔽 Typst 的 `math` 模块）。
- 页面几何参数名 **`layout`**（禁 `page`，会遮蔽 Typst 的 `page` 函数）。

### Typst 作用域规则：`set`/`show` 必须覆盖 `body`

Typst 中 `set`/`show` 规则仅对其所在词法作用域内**后续**的 content 生效。函数内直接写 `set ...; body` 无效——因为 `set` 在代码块作用域内，而 `body` 是独立表达式。

正确做法：
- **`set page`**：因为 `page` 只有 `set` 有效果，用 `let body = if ... { set page(...); body }` 重新绑定 body。
- **`set text` / `set par` / `show math.equation`**：写在函数体顶层，通过最终表达式（`if hijack { show ...; body } else { body }`）返回 body，此时所有前置 set/show 规则生效。
- **`show strong` / `show emph`**：写在 `if hijack` 分支内，`body` 从同一分支返回，规则生效。

### 注册表模块化

各预设文件底部就地维护 tag 映射片段（调用 `_make-tags`），`tags.typ` 只负责汇总 + 分派。修改预设时在同一文件内顺手完成，无需跨文件维护。

## 预设一览

| 类别 | `setup` 参数 | 预设键（= tag 名） |
|------|-------------|-------------------|
| 西文 | `western` | `lm`（Latin Modern） |
| 中文 | `chinese` | `noto`（Noto Serif/Sans CJK SC）, `lxgw`（LXGW 系列） |
| 页面 | `layout` | `oneline`, `portrait`, `landscape` |
| 文档 | `document` | `zhcn` |

用法：`#show: tag.with("lm", "lxgw", "portrait", margin: 12pt)`

等价于：`#show: setup.with(western: "lm", chinese: "lxgw", layout: "portrait", margin: 12pt)`

## 文件结构

| 文件 | 职责 |
|------|------|
| `lib.typ` | 包入口，re-export |
| `typst.toml` | 包清单 |
| `setup.typ` | `setup` 函数实现 |
| `tags.typ` | Tag 汇总 + `tag` 函数 |
| `_util.typ` | 内部工具（`_make-tags`） |
| `fonts.typ` | 西文/中文字体预设 + `_merge-presets` + tag 片段 |
| `pages.typ` | 页面几何预设 + tag 片段 |
| `documents.typ` | 文档流预设 + tag 片段 |
| `tests/` | 测试文件 |

## 验证

```bash
./test.sh
```

缺字环境可能对 Noto Sans/Serif CJK 等报警告，属环境字体问题。

## 后续可做（未承诺）

- README / 发布到 `@preview` 与版本号策略。
- 与外部 Rust 应用联动：去掉注入 prelude，改为提示用户 `#import` + `#show: setup.with(...)`。
