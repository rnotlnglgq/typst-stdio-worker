// -- Argument help --

bilingual!(
    help_font_path,
    "Additional font directories to load (can be repeatedly specified)",
    "额外字体目录（可多次指定）"
);

bilingual!(
    help_package_path,
    "Custom path to local packages (TYPST_PACKAGE_PATH; same as typst compile --package-path). CLI overrides env. Resolved before the package cache root.",
    "本地包搜索根目录（TYPST_PACKAGE_PATH；与 typst compile --package-path 相同）。命令行优先于环境变量。先于包缓存根目录解析。"
);

bilingual!(
    help_package_cache_path,
    "Custom path to package cache (TYPST_PACKAGE_CACHE_PATH; same as typst compile --package-cache-path). Falls back to $XDG_CACHE_HOME/typst/packages or ~/.cache/typst/packages when unset.",
    "包缓存目录（TYPST_PACKAGE_CACHE_PATH；与 typst compile --package-cache-path 相同）。未设置时使用 $XDG_CACHE_HOME/typst/packages 或 ~/.cache/typst/packages。"
);

bilingual!(
    help_allow_download,
    "Allow downloading missing packages from the Typst package registry",
    "允许从 Typst 包注册表下载缺失的包"
);

bilingual!(
    help_scale,
    "PNG pixel scale factor",
    "PNG 像素缩放因子"
);

bilingual!(
    help_max_pages,
    "Maximum number of pages to render",
    "最大渲染页数"
);

bilingual!(
    help_max_pixels,
    "Maximum total pixels (width*height across all pages)",
    "最大总像素数（所有页面宽×高之和）"
);

bilingual!(
    help_max_input_size,
    "Maximum input size in bytes",
    "最大输入大小（字节）"
);

bilingual!(
    help_log_level,
    "Log level filter",
    "日志级别"
);

bilingual!(
    help_meter,
    "Log aggregate font file bytes (deduplicated) and each rendered PNG size / dimensions",
    "记录去重后的字体文件总字节数，以及每次渲染 PNG 的尺寸与编码体积"
);

// -- Subcommand-specific argument help --

bilingual!(
    help_max_compilations,
    "Exit after N compilations (0 = unlimited) [default: 0]",
    "编译 N 次后退出（0 = 无限制）[默认: 0]"
);

bilingual!(
    help_open,
    "Open the rendered PNG with xdg-open after each compilation",
    "每次编译后使用 xdg-open 打开渲染的 PNG"
);

bilingual!(
    help_template,
    "Template to apply [default: default]",
    "应用的模板 [默认: default]"
);
