# TODO

## Android 仓库路径写死为 `/data/data/<应用 id>/files`

- **位置**：`app/src/paths.rs` 的 `data_dir()`。手机仓库在 `<data_dir>/<仓库名>`，日志在 `<data_dir>/calliope.log`。
- **问题**：Android 工作资料（work profile）或多用户环境下，应用的实际目录是 `/data/user/<用户编号>/<应用 id>/files`。`/data/data/...` 只指向主用户，在这些环境里会无法读写。
- **原因**：nsh 0.4 的 Android 层通过 Java 的 `getFilesDir()` 拿到应用目录，但只传给了 nsh 引擎（`EngineConfig::data_dir`），没有开放给应用的 Rust 代码。
- **修复方向**：等 nsh 开放应用目录后改用它；或者用 nsh 的原生模块从 Java 侧读取 `getFilesDir()`。
