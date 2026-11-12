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

## 图谱不能用滚轮缩放、不能用触控板双指滑动平移

- **位置**：`app/src/pages/graph.rs` 的 `diagram()`；MVP 文档 F6 的“交互”要求“平移、缩放（滚轮、触控板、双指捏合）”。
- **问题**：图谱里单独滚动鼠标滚轮没有反应，触控板双指滑动也没有反应。现在只能拖动平移，用 `Ctrl` + 滚轮、触控板或手机的双指捏合缩放。
- **原因**：nsh 只把滚轮和触控板滚动交给滚动视图（`scroll()` 的 `on_scroll`），其他元素收不到滚轮事件；`Ctrl` + 滚轮被当作捏合手势，交给 `pinch_value`（`nsh-platform-linux/src/input.rs`，锁定版本 `c5214aa`）。
- **修复方向**：等 nsh 给元素开放滚轮事件后，滚轮缩放、双指滑动平移。

## Android 的同步密码没有存进 Keystore

- **位置**：`app/src/secrets.rs`。MVP 文档 8.6 要求 Android 的密码存进 Keystore。
- **问题**：Android 上的同步密码现在存在应用私有目录里的 `secrets.json`（`<data_dir>/secrets.json`）。其他应用读不到这个文件，但它没有经过 Keystore 加密；手机被 root 或者被做了完整备份时能读出来。
- **原因**：现成的 [android-native-keyring-store](https://crates.io/crates/android-native-keyring-store) 要求先用 JavaVM 和应用的 Context 初始化 `ndk-context`。nsh 0.4 在 `JNI_OnLoad` 里拿到了 JavaVM，但只在 `nsh-platform-android` 内部使用，没有开放给应用（锁定版本 `c5214aa`）；`JNI_GetCreatedJavaVMs` 要到 API 31 才是公开的 NDK 接口，而 nsh 的 minSdk 是 26；Context 也只能通过隐藏 API 拿到。
- **修复方向**：等 nsh 开放 JavaVM 和 Context（或者初始化 `ndk-context`）后，在 `secrets.rs` 的 Android 分支改用 `android-native-keyring-store`，并把 `secrets.json` 里已有的密码迁移过去。同一个接口也能解决上面“Android 仓库路径写死”那一条。
