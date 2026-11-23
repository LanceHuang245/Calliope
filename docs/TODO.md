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
