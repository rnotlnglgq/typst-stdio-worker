// -- Program-level descriptions --

bilingual!(
    about,
    "Typst compilation worker — renders Typst source to PNG via pipe, interactive REPL, or long-running NDJSON protocol.",
    "Typst 编译工作进程 — 通过管道、交互式 REPL 或长连接 NDJSON 协议将 Typst 源码渲染为 PNG。"
);

// -- Subcommand descriptions --

bilingual!(
    cmd_worker_about,
    "Long-running NDJSON worker (stdin/stdout protocol)",
    "长连接 NDJSON 工作模式（stdin/stdout 协议）"
);

bilingual!(
    cmd_interactive_about,
    "Interactive REPL: type source, press Enter to compile and preview",
    "交互式 REPL：输入源码，按回车编译预览"
);

bilingual!(
    cmd_pipe_about,
    "One-shot pipe mode: read stdin, compile, write PNG to stdout",
    "单次管道模式：读取 stdin，编译后输出 PNG 到 stdout"
);
