// -- Tracing log messages --

bilingual!(log_signal_register_failed, "failed to register signal handler", "注册信号处理器失败");
bilingual!(log_using_package_cache, "using package cache", "使用包缓存");
bilingual!(log_using_package_path, "using package path", "使用本地包路径");
bilingual!(log_loading_fonts, "loading fonts", "正在加载字体");
bilingual!(log_fonts_loaded, "fonts loaded", "字体加载完成");
bilingual!(log_no_package_cache, "no package cache directory found", "未找到包缓存目录");
bilingual!(log_no_package_path, "no package path set", "未设置本地包路径");
bilingual!(log_write_ready_failed, "failed to write ready message", "写入就绪消息失败");
bilingual!(log_shutdown_signal, "received shutdown signal, exiting", "收到关闭信号，退出");
bilingual!(log_stdin_closed, "stdin closed, shutting down", "标准输入关闭，正在停止");
bilingual!(log_protocol_error, "protocol error", "协议错误");
bilingual!(log_write_response_failed, "failed to write response", "写入响应失败");
bilingual!(log_max_compilations, "reached max compilations, exiting", "已达最大编译次数，退出");
bilingual!(log_cache_create_failed, "failed to create package cache directory; package imports disabled", "创建包缓存目录失败；已禁用包导入");
bilingual!(log_cache_canonicalize_failed, "package cache directory cannot be canonicalized; package imports disabled", "包缓存目录无法规范化；已禁用包导入");
bilingual!(log_path_canonicalize_failed, "package path cannot be canonicalized, skipping", "本地包路径无法规范化，跳过");
bilingual!(log_path_not_exist, "package path does not exist, skipping", "本地包路径不存在，跳过");
bilingual!(log_downloading_package, "downloading package", "正在下载包");
bilingual!(log_package_downloaded, "package downloaded and extracted", "包下载并解压完成");
bilingual!(log_package_already_exists, "package already placed by another worker, discarding our copy", "另一工作进程已安装此包，丢弃本次下载");
bilingual!(log_render_complete, "render complete", "渲染完成");
bilingual!(log_source_access, "source", "源文件访问");
bilingual!(log_file_access, "file", "文件访问");
bilingual!(log_template_internal_error, "template internal error", "模板内部错误");
