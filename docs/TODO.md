# TODO

## nsh `Editor`：键入已存在的右括号后，光标画错位置

- **位置**：nsh 的 `nsh-base/src/editor/controller.rs` 的 `after_typing`（截至 2026-11-24，nightly `589a60f` 仍未修复）。在 Linux 上复现，其他平台未验证。
- **问题**：输入 `[`，编辑器自动补成 `[]`、光标在中间；再手动输入 `]`，真实光标已在 `]` 后面（接着输入 `X` 得到 `[]X`），但界面上闪烁的光标仍画在 `[` 和 `]` 之间，直到下一次移动光标才恢复。所有自动配对的符号（`()`、`{}`、`""`、`''`）都一样。
- **原因**：右侧已有同样的右括号时，编辑器不插入字符，只把光标右移一格（`commands.rs` 的 `typed_char`），结果是 `Outcome::Moved`。`after_typing` 只处理 `Outcome::Edited`，光标移动被忽略，界面没被通知重画光标。方向键走的 `after` 处理了 `Outcome::Moved`，所以那条路径正常。
- **修复方向**：向 nsh 上游报告。修法是在 `after_typing` 里也处理 `Outcome::Moved`，照 `after` 的做法调用 `bump_selection()`、`report_selections()`、`show_caret()`、`restart_blink()`（未编译验证）。Calliope 这边没有绕过办法（关掉括号自动补全代价更大）。

## nsh `Editor`：手机上点补全列表的项不会补全

- **位置**：nsh 的 `nsh-base/src/editor/completion.rs` 的 `list()` 里那个 `scroll()`（截至 2026-11-24，nightly `589a60f` 仍未修复）。2026-11-20 在 Android 模拟器上复现，iOS 未验证。
- **问题**：手机上输入 `[[`（或按键盘工具栏的 `[[`）弹出笔记标题，点其中一项后，键盘收起、列表关闭，但什么都没插入。
- **原因**：滚动视图的 `keyboard_taps` 默认是 `KeyboardTaps::Never`：键盘弹起时，在里面点一下只收起键盘，里面的内容收不到这次点击。补全列表的滚动视图没有改这个默认值；键盘一收、编辑器失去焦点，补全列表随之关闭（`completion_blurred`）。nsh 自己的命令面板（`command.rs`）、列表框（`listbox.rs`）和菜单（`menu.rs`）都设了 `.keyboard_taps(KeyboardTaps::Handled)`，只有补全列表漏了。
- **修复方向**：向 nsh 上游报告。修法是给补全列表的 `scroll()` 加上 `.keyboard_taps(KeyboardTaps::Handled)`，和 `command.rs` 一样（未编译验证）。Calliope 这边没有绕过办法：这个滚动视图在编辑器内部，应用设置不到。修好后 `make update` 即可。

## Android 的同步密码没有存进 Keystore

- **位置**：`app/src/secrets.rs`。MVP 文档 8.6 要求 Android 的密码存进 Keystore。
- **问题**：Android 上的同步密码现在存在应用私有目录里的 `secrets.json`（`<data_dir>/secrets.json`）。其他应用读不到这个文件，但它没有经过 Keystore 加密；手机被 root 或者被做了完整备份时能读出来。
- **原因**：现成的 [android-native-keyring-store](https://crates.io/crates/android-native-keyring-store) 要求先用 JavaVM 和应用的 Context 初始化 `ndk-context`。nsh 0.4 在 `JNI_OnLoad` 里拿到了 JavaVM，但只在 `nsh-platform-android` 内部使用，没有开放给应用（锁定版本 `c5214aa`）；`JNI_GetCreatedJavaVMs` 要到 API 31 才是公开的 NDK 接口，而 nsh 的 minSdk 是 26；Context 也只能通过隐藏 API 拿到。
- **修复方向**：等 nsh 开放 JavaVM 和 Context（或者初始化 `ndk-context`）后，在 `secrets.rs` 的 Android 分支改用 `android-native-keyring-store`，并把 `secrets.json` 里已有的密码迁移过去。
