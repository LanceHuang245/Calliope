# Calliope

一个以本地 Markdown 文件为核心、内置 WebDAV / S3 同步的笔记应用，基于 [Native Shell (nsh)](https://nsh.dev) 使用 Rust 编写，在每个平台上使用系统原生 UI 渲染。

## 目标平台

| 平台 | nsh 后端 | 运行方式 |
|---|---|---|
| Windows | Win32 / Direct2D / DirectWrite | `cargo run -p calliope` 或 `nsh run app --target windows` |
| macOS | AppKit / Core Animation | `cargo run -p calliope` 或 `nsh run app --target macos` |
| Linux | GTK 4 / Pango | `cargo run -p calliope` 或 `nsh run app --target linux` |
| Android | Android Views（Material 3 导航栏） | `nsh run app --target android` |
| iOS（暂不上架） | UIKit（iOS 26 Liquid Glass） | `nsh run app --target ios`（需要 macOS + Xcode） |

## 目录结构

```text
Calliope/
├── Cargo.toml              Cargo workspace：成员与共享依赖（nsh 0.4）
├── rust-toolchain.toml     stable 工具链与移动端 target
├── app/                    nsh 应用（UI 层）
│   ├── nsh.json            应用名、id、版本、根组件、主题、外观（跟随系统）
│   ├── theme.json          设计令牌（shadcn/ui cssVars，浅色/深色）
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs          根组件 App 与 #[native_module]
│   │   ├── main.rs         cargo run：桌面后端入口
│   │   ├── shell.rs        应用外壳：桌面（侧边栏 + 工作区）/ 移动（系统 TabBar + 导航栈）
│   │   ├── store.rs        UI 状态
│   │   ├── strings.rs      界面文案（集中管理）
│   │   ├── paths.rs        应用数据目录、手机沙盒仓库；logging.rs：日志文件
│   │   ├── pages/          页面（笔记、图谱、设置……）
│   │   └── components/     Calliope 自己的样式组件层（基于 nsh-base）
│   └── tests/              无头 UI 测试（cargo test）
├── crates/
│   ├── calliope-core/      知识库：Vault、笔记、链接解析、反向链接索引、图谱（无 UI）
│   └── calliope-sync/      同步：存储后端（WebDAV、S3）、同步引擎、状态、冲突处理（无 UI）
├── docs/                   产品与设计文档
└── .github/workflows/      CI：三个桌面平台上的 fmt、clippy、test，5 个平台的安装包
```

UI 层只依赖 `calliope-core` / `calliope-sync` 的公开接口；两个核心 crate 不依赖 nsh，可以单独测试。

## 开发

需要：

- Rust stable（`rust-toolchain.toml` 会自动安装所需 target）
- Windows：Visual Studio Build Tools（C++ 桌面开发 + Windows SDK）
- Linux：GTK 4.14+ 开发包
- 移动端与打包：[nsh CLI](https://nsh.dev/docs/installation)；Android 需要 SDK + NDK，iOS 需要 Xcode

```sh
cargo run -p calliope          # 在本机桌面后端运行
cargo test --workspace         # 运行全部测试（含无头 UI 测试）
nsh run app --target android   # 在 Android 模拟器/设备运行
nsh package app --target windows
```
