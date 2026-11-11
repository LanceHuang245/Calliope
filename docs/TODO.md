# TODO

## Android 仓库路径写死为 `/data/data/<应用 id>/files`

- **位置**：`app/src/paths.rs` 的 `data_dir()`。手机仓库在 `<data_dir>/<仓库名>`，日志在 `<data_dir>/calliope.log`。
- **问题**：Android 工作资料（work profile）或多用户环境下，应用的实际目录是 `/data/user/<用户编号>/<应用 id>/files`。`/data/data/...` 只指向主用户，在这些环境里会无法读写。
- **原因**：nsh 0.4 的 Android 层通过 Java 的 `getFilesDir()` 拿到应用目录，但只传给了 nsh 引擎（`EngineConfig::data_dir`），没有开放给应用的 Rust 代码。
- **修复方向**：等 nsh 开放应用目录后改用它；或者用 nsh 的原生模块从 Java 侧读取 `getFilesDir()`。

## nsh `Editor`：键入已存在的右括号后，光标画错位置

- **位置**：nsh 的 `nsh-base/src/editor/controller.rs` 的 `after_typing`（锁定版本 `c5214aa`；截至 2026-11-11，上游 `main` 新 136 个提交仍未修复）。在 Linux 上复现，其他平台未验证。
- **问题**：输入 `[`，编辑器自动补成 `[]`、光标在中间；再手动输入 `]`，真实光标已在 `]` 后面（接着输入 `X` 得到 `[]X`），但界面上闪烁的光标仍画在 `[` 和 `]` 之间，直到下一次移动光标才恢复。所有自动配对的符号（`()`、`{}`、`""`、`''`）都一样。
- **原因**：右侧已有同样的右括号时，编辑器不插入字符，只把光标右移一格（`commands.rs` 的 `typed_char`），结果是 `Outcome::Moved`。`after_typing` 只处理 `Outcome::Edited`，光标移动被忽略，界面没被通知重画光标。方向键走的 `after` 处理了 `Outcome::Moved`，所以那条路径正常。
- **修复方向**：向 nsh 上游报告。修法是在 `after_typing` 里也处理 `Outcome::Moved`，照 `after` 的做法调用 `bump_selection()`、`report_selections()`、`show_caret()`、`restart_blink()`（未编译验证）。Calliope 这边没有绕过办法（关掉括号自动补全代价更大）。
