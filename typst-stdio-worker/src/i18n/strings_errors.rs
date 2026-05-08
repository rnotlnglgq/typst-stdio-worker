// -- Error message fragments (used as `description` argument to `fmt_error`) --

bilingual!(
    err_read_stdin,
    "failed to read stdin",
    "读取标准输入失败"
);

bilingual!(
    err_write_output,
    "failed to write output",
    "写入输出失败"
);

bilingual!(
    err_write_file,
    "failed to write output file",
    "写入输出文件失败"
);

bilingual!(
    err_read_line,
    "failed to read input",
    "读取输入失败"
);

// -- Pipe / interactive diagnostics (labels only; compiler text stays as-is) --

bilingual!(
    dlg_label_warning,
    "warning",
    "警告"
);

bilingual!(
    dlg_label_error,
    "error",
    "错误"
);

bilingual!(
    dlg_hint_prefix,
    "  hint: ",
    "  提示："
);
