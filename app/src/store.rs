//! The UI state: its signals stay private, pages read and change it through
//! these functions. A global is per surface, so each test has its own.
//!
//! The vault, the link index and sync live in `calliope-core` and
//! `calliope-sync`; this module keeps what the UI shows of them.

use nsh_core::prelude::*;

const WELCOME: &str = r#"# 欢迎使用 Calliope

Calliope 是一个以本地 Markdown 文件为核心的笔记应用，内置 **WebDAV / S3** 同步。

- [[知识图谱]] 与反向链接
- Markdown 实时渲染：公式 $E = mc^2$、代码、表格、图片
- 编辑、阅读、分栏预览三种模式

$$
\int_0^1 x^2 \, dx = \frac{1}{3}
$$

> [!NOTE]
> 这是项目骨架的占位页面。
"#;

#[derive(Clone)]
struct Session {
    /// The Markdown of the note shown.
    markdown: SharedString,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            markdown: SharedString::from(WELCOME),
        }
    }
}

fn session() -> Signal<Session> {
    global::<Session>()
}

/// The Markdown of the note shown (tracked where it is read).
pub(crate) fn current_markdown() -> SharedString {
    session().with(|session| session.markdown.clone())
}
