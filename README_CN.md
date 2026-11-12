[English](README.md) | 简体中文

<p align="center">
  <img src="images/Calliope_Logo.png" alt="Calliope logo" width="200">
</p>

# Calliope

[![CI](https://github.com/LanceHuang245/Calliope/actions/workflows/ci.yml/badge.svg)](https://github.com/LanceHuang245/Calliope/actions/workflows/ci.yml)

Calliope 是一款本地优先、跨平台的 Markdown 知识库，使用 Rust 编写，通过 [Native Shell (nsh)](https://nsh.dev) 以各平台的原生 UI 渲染，支持 Windows、macOS、Linux、Android 和 iOS。\
笔记始终以普通 Markdown 文件的形式保存在你自己的文件夹中，通过双链、反向链接和全库图谱相互关联。同步直接连接你自己的 WebDAV 或 S3 存储，不经过任何中间服务器。
> **注意**
>
> Calliope 仍处于早期开发阶段，功能和界面还会变化。用它打开和同步的笔记请自行备份。
---
## 功能
- 普通文件：任何一个装着 Markdown 笔记的文件夹都是一个仓库；Calliope 只会添加一个隐藏的 `.calliope/` 目录
- 三种模式：编辑、阅读，以及左右分栏；源码支持查找替换
- Markdown 渲染：GitHub 风格的 Markdown（表格、任务列表、脚注）、代码高亮、LaTeX 公式、图片，以及 GitHub 的 5 种 Callout
- 双链：`[[笔记]]`、`[[笔记|别名]]`、`[[笔记#标题]]` 和 `[[文件夹/笔记]]`，按名称在整个仓库中解析；链接到不存在的笔记时可以直接新建
- 反向链接：列出所有链接到当前笔记的笔记，以及链接所在的那一行
- 链接补全：输入 `[[` 弹出笔记标题
- 快速切换：按标题查找笔记，或新建一篇
- Mermaid 图表：流程图、时序图、类图、状态图、甘特图和饼图，由 [merman](https://crates.io/crates/merman) 用 Rust 原生绘制（不使用 WebView）
- 图谱：展示所有笔记和链接，支持平移、缩放、悬停高亮，可选显示孤立笔记和尚未创建的笔记
- WebDAV 和 S3 同步：与上次同步的状态做三方比较，冲突时保留副本而不是覆盖，删除大量文件前先询问，被删除的文件在仓库的回收站里保留 15 天；密码保存在系统的密钥环里（Android 上暂时保存在应用的私有存储中）
- 编辑安全：一秒内自动保存、原子写入、删除的文件进入回收站；其他应用修改的内容会被读入，但不会覆盖你未保存的修改
- 主题：浅色、深色或跟随系统
- 五个平台原生运行：Windows、macOS、Linux、Android 和 iOS

## 路线图
Calliope 分阶段开发，双链、图谱和同步已经完成。接下来：
- 移动端：系统 TabBar 与导航，Android 上的 Material 3 和 iOS 上的 Liquid Glass
- 端到端加密：同步的笔记内容、文件名和目录名都加密
- 打磨：多标签页、全文搜索、命令面板、局部图谱和大纲

## 使用
1. 打开一个装着 Markdown 笔记的文件夹，或新建一个仓库。在手机上，Calliope 会在应用自己的存储中创建仓库
2. 用 `Ctrl+N` 新建笔记，笔记的标题就是文件名。新建文件夹、重命名和删除在文件树的右键菜单里
3. 用 `[[笔记]]` 链接笔记，然后在阅读模式下点击跳转，并在反向链接面板里查看
4. 用 `Ctrl+G` 或文件树下方的按钮打开图谱；拖动来移动，`Ctrl` + 滚轮（或双指捏合）缩放
5. 在“设置 → Sync”里配置同步（WebDAV 或 S3）。打开仓库时、停止编辑 30 秒后、以及每隔几分钟会自动同步，也可以点同步按钮

| 快捷键 | 功能 |
|---|---|
| `Ctrl+N` | 新建笔记 |
| `Ctrl+E` | 在编辑和阅读之间切换 |
| `Ctrl+F` | 在笔记内查找 |
| `Ctrl+O` | 快速切换 |
| `Ctrl+G` | 图谱 |
| `Ctrl+\` | 隐藏或显示侧边栏 |

在 macOS 上用 `Cmd` 代替 `Ctrl`。

## 参与贡献
欢迎社区用户参与贡献！你可以 Fork 本仓库、提交 Pull Request，或通过 [Issues](https://github.com/LanceHuang245/Calliope/issues) 提出建议和报告问题。

### 构建
需要：
- Rust stable（`rust-toolchain.toml` 会自动安装所需的 target）
- Windows：Visual Studio Build Tools（使用 C++ 的桌面开发、Windows SDK）
- Linux：GTK 4.14 或更高版本，以及 GStreamer，包括它们的开发包
- 移动端构建与打包：[nsh CLI](https://nsh.dev/docs/installation)；Android 需要 SDK 和 NDK，iOS 需要 Xcode

```sh
cargo run -p calliope            # 在本机桌面上运行
cargo test --workspace           # 运行全部测试，包括无头 UI 测试
nsh run app --target android     # 在 Android 模拟器或设备上运行
nsh package app --target windows # 构建安装包
```

UI 位于 `app/`；仓库、链接、索引和图谱位于 `crates/calliope-core`，同步位于 `crates/calliope-sync`。这两个 crate 都不依赖 UI，可以单独测试。

### 翻译
界面目前只有英文。所有界面文案都集中在 [`app/src/strings.rs`](app/src/strings.rs) 中，以后可以加入其他语言。

## 下载
目前还没有正式版本。每次推送都会为五个平台构建安装包：[Calliope Action](https://github.com/LanceHuang245/Calliope/actions/workflows/ci.yml)（需要登录 GitHub 才能下载）。

| 平台 | 最低版本 | 说明 |
|---|---|---|
| Windows | Windows 10 1809 | |
| macOS | macOS 14 | 仅在 CI 中构建和测试 |
| Linux | GTK 4.14 | |
| Android | Android 9 | APK：64 位（arm64-v8a）或 32 位（armeabi-v7a）手机各一个，以及通用包（模拟器也能用） |
| iOS | iOS 15 | 未签名的 IPA，需要自行签名后安装 |

## 许可证
[Apache License 2.0](LICENSE) © Linxing Huang
