# OfficeCLI

> **OfficeCLI 是全球首个、也是最好的专为 AI 智能体设计的 Office 套件。**

**让任何 AI 智能体完全掌控 Word、Excel、PowerPoint 和 PDF——只需一行代码。**

开源免费。单一二进制文件。无需安装 Office。零依赖。全平台运行。

**内置 agent 友好渲染引擎** —— 智能体可以"看见"自己创建的内容，无需 Office。把 `.docx` / `.xlsx` / `.pptx` / `.pdf` 渲染为 HTML 或 SVG，"渲染 → 看 → 改" 循环在二进制能跑的任何地方都成立。

[![GitHub Release](https://img.shields.io/github/v/release/iOfficeAI/OfficeCLI)](https://github.com/iOfficeAI/OfficeCLI/releases)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

[English](README.md) | **中文** | [日本語](README_ja.md) | [한국어](README_ko.md)

<p align="center">
  <strong>💬 社区:</strong> <a href="https://discord.gg/2QAwJn7Egx" target="_blank">Discord</a>
</p>

<p align="center">
  <img src="assets/ppt-process.webp" alt="OfficeCLI 创建 PowerPoint 演示文稿" width="100%">
</p>

<p align="center"><em>在 <a href="https://github.com/iOfficeAI/AionUi">AionUi</a> 上使用 OfficeCLI 的 PPT 制作过程</em></p>

## 支持的格式

| 格式 | 读取 | 修改 | 创建 | 文本/偏移映射 |
|------|------|------|------|---------------|
| Word (.docx) | ✅ | ✅ | ✅ | ✅ |
| Excel (.xlsx) | ✅ | ✅ | ✅ | ✅ |
| PowerPoint (.pptx) | ✅ | ✅ | ✅ | ✅ |
| PDF (.pdf) | ✅ | ✅ (文本替换、删除页面) | — | ✅ |

## AI 智能体 — 文本/偏移 → 路径映射

每个文档都可以输出 **TextOffsetMap** —— 完整文本加上字符偏移→路径 ID 的映射。AI 智能体读取映射，找到需要修改的文本位置，获取精确的文档路径（如 `/body/p[3]/r[1]`），然后使用 `set` 进行精确修改。无需猜测，无需正则解析。

```bash
officecli extract-text report.docx --with-offsets --json
```

```json
{
  "full_text": "你好世界\n第二段",
  "spans": [
    {"start": 0, "end": 2, "path": "/body/p[1]/r[1]", "text": "你好", "element_type": "run"},
    {"start": 2, "end": 4, "path": "/body/p[1]/r[2]", "text": "世界", "element_type": "run"},
    {"start": 4, "end": 10, "path": "/body/p[2]/r[1]", "text": "第二段", "element_type": "run"}
  ],
  "meta": {"format": "docx", "total_chars": 10, "total_spans": 3}
}
```

四种格式全部支持 — docx、xlsx、pptx 和 pdf。

## 开发者 — 30 秒亲眼看到效果

```bash
# 1. 安装（macOS / Linux）
curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
# Windows：从 GitHub Releases 下载

# 2. 创建一个空白 PowerPoint
officecli create deck.pptx

# 3. 启动实时预览 — 浏览器自动打开 http://localhost:26315
officecli watch deck.pptx

# 4. 打开另一个终端，添加一页幻灯片 — 浏览器即时刷新
officecli add deck.pptx / --type slide --prop title="Hello, World!"
```

## 快速开始

```bash
# 创建演示文稿并添加内容
officecli create deck.pptx
officecli add deck.pptx / --type slide --prop title="Q4 报告"

# 查看大纲
officecli view deck.pptx outline

# 查看 HTML — 在浏览器中打开渲染预览
officecli view deck.pptx html

# 获取任意元素的结构化数据
officecli get deck.pptx '/slide[1]' --json

# 查看 PDF 文档
officecli view report.pdf --mode text
officecli get report.pdf '/page[1]' --json

# 提取文本与偏移映射（用于 AI 智能体定位）
officecli extract-text report.docx --with-offsets --json
```

## 为什么选择 OfficeCLI？

**OfficeCLI 能做什么：**

- **创建** 文档 -- 空白文档或带内容的文档
- **读取** 文本、结构、样式 -- 纯文本或结构化 JSON
- **修改** 任意元素 -- 文本、样式、布局
- **重组** 内容 -- 添加、删除、移动、复制元素
- **验证** 文档结构，检测问题
- **提取** 文本与偏移→路径映射，用于 AI 智能体定位
- **渲染** 文档为 HTML/SVG，用于可视化预览
- **PDF 支持** — 读取、查看、修改文本、删除页面、提取图片

## 安装

以单一原生二进制文件分发。纯 Rust 实现，跨平台，无运行时依赖。

**一键安装：**

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
```

**或手动下载** [GitHub Releases](https://github.com/iOfficeAI/OfficeCLI/releases)：

| 平台 | 二进制文件 |
|------|-----------|
| macOS Apple Silicon | `officecli-mac-arm64` |
| macOS Intel | `officecli-mac-x64` |
| Linux x64 | `officecli-linux-x64` |
| Linux ARM64 | `officecli-linux-arm64` |
| Windows x64 | `officecli-win-x64.exe` |
| Windows ARM64 | `officecli-win-arm64.exe` |

验证安装：`officecli --version`

## 核心功能

### 三层架构

从简单开始，仅在需要时深入。

| 层 | 用途 | 命令 |
|----|------|------|
| **L1：读取** | 内容的语义视图 | `view`（text、annotated、outline、stats、issues、html、svg） |
| **L2：DOM** | 结构化元素操作 | `get`、`query`、`set`、`add`、`remove`、`move`、`copy` |
| **L3：原始 XML** | XPath 直接访问 — 通用兜底 | `raw`、`raw-set`、`add-part`、`validate` |

```bash
# L1 — 高级视图
officecli view report.docx annotated
officecli view budget.xlsx stats
officecli view report.pdf text

# L2 — 元素级操作
officecli query report.docx "paragraph"
officecli add budget.xlsx / --type sheet --prop name="Q2 报告"
officecli remove report.pptx '/slide[3]'

# L3 — L2 不够时用原始 XML
officecli raw deck.pptx 'ppt/slides/slide1.xml'
officecli raw-set report.docx document --xpath "//w:p[1]" --action append --xml '<w:r><w:t>注入文本</w:t></w:r>'
```

### 驻留模式与批量执行

驻留模式将文档保持在内存中，批量模式在一次打开/保存周期内执行多条命令。

```bash
# 驻留模式 — 通过 Unix Domain Socket 通信，延迟接近零
officecli open report.docx
officecli set report.docx /body/p[1]/r[1] --prop text="已更新"
officecli close report.docx

# 批量模式 — 原子化多命令执行
echo '[{"command":"set","path":"/slide[1]/shape[1]","props":{"text":"你好"}}]' \
  | officecli batch deck.pptx --json
```

### PDF 支持

读取、查看和修改 PDF 文档：

```bash
# 读取 PDF 文本
officecli view report.pdf text
officecli view report.pdf outline

# 获取页面内容
officecli get report.pdf '/page[1]'

# 提取文本与偏移映射
officecli extract-text report.pdf --with-offsets --json

# 修改 PDF — 替换页面文本
officecli set report.pdf '/page[1]' --prop text="新内容"
officecli save report.pdf

# 删除页面
officecli remove report.pdf '/page[3]'
officecli save report.pdf

# 渲染为 SVG 预览
officecli view report.pdf svg
```

### 文本/偏移 → 路径映射

每种格式都输出偏移→路径映射，让 AI 智能体精确定位和修改文本：

```bash
# Docx：字符偏移映射到段落/文本片段路径
officecli extract-text report.docx --with-offsets --json

# Xlsx：单元格偏移映射到工作表/单元格路径
officecli extract-text budget.xlsx --with-offsets --json

# Pptx：文本偏移映射到幻灯片/形状/段落路径
officecli extract-text deck.pptx --with-offsets --json

# Pdf：字符偏移映射到页面/文本块路径
officecli extract-text report.pdf --with-offsets --json
```

## AI 集成

### MCP 服务器

内置 [MCP](https://modelcontextprotocol.io) 服务器：

```bash
officecli mcp         # 启动 MCP stdio 服务器
```

通过 JSON-RPC 暴露所有文档操作 — 无需 shell 访问。

### 内置帮助

```bash
officecli --help                     # 完整命令概览
officecli view --help                # view 命令详情
officecli get --help                 # get 命令详情
```

## 命令参考

| 命令 | 说明 |
|------|------|
| `create` | 创建空白 .docx、.xlsx 或 .pptx |
| `view` | 查看内容（模式：text、annotated、outline、stats、issues、html、svg） |
| `get` | 获取元素及子元素（`--depth N`、`--json`） |
| `query` | CSS 风格查询 |
| `set` | 修改元素属性 |
| `add` | 添加元素 |
| `remove` | 删除元素 |
| `move` | 移动元素 |
| `copy` | 从源复制元素到目标 |
| `validate` | 验证文档结构 |
| `extract-text` | 提取文本与偏移→路径映射（`--with-offsets`、`--json`） |
| `batch` | 单次周期内执行多条操作 |
| `dump` | 序列化文档为可重放的 JSON |
| `raw` | 查看文档部件的原始 XML |
| `raw-set` | 通过 XPath 修改原始 XML |
| `watch` | 实时 HTML 预览，自动刷新 |
| `open` | 启动驻留模式 |
| `close` | 保存并关闭驻留模式 |
| `mcp` | 启动 MCP 服务器，用于 AI 工具集成 |

## 对比

| | OfficeCLI | Microsoft Office | LibreOffice | python-docx / openpyxl |
|---|---|---|---|---|
| 开源免费 | ✓ (Apache 2.0) | ✗（付费授权） | ✓ | ✓ |
| AI 原生 CLI + JSON | ✓ | ✗ | ✗ | ✗ |
| 零安装（单一二进制文件） | ✓ | ✗ | ✗ | ✗（需 Python + pip） |
| PDF 读取/修改 | ✓ | ✗ | ✓ | ✗ |
| 文本/偏移 → 路径映射 | ✓ | ✗ | ✗ | ✗ |
| 基于路径的元素访问 | ✓ | ✗ | ✗ | ✗ |
| 原始 XML 兜底 | ✓ | ✗ | ✗ | 部分支持 |
| 跨平台原生 | ✓ (Rust) | Windows/Mac | ✓ | ✓ |
| Word + Excel + PowerPoint + PDF | ✓ | ✓ | ✓ | 需要多个库 |

## 从源码构建

需要 [Rust](https://rustup.rs/)（1.75+）。

```bash
cargo build --release
# 二进制文件位于 target/release/officecli
```

跨平台编译：

```bash
# macOS ARM
cargo build --release --target aarch64-apple-darwin

# Linux x64（需要交叉链接器）
cargo build --release --target x86_64-unknown-linux-gnu

# Windows x64（需要交叉链接器）
cargo build --release --target x86_64-pc-windows-msvc
```

## 项目结构

```
OfficeCLI/
├── Cargo.toml                 # Workspace 根目录
├── crates/
│   ├── officecli/              # CLI 入口 + 命令
│   ├── handler-common/         # 共享 trait + 类型
│   ├── oxml/                   # OOXML ZIP/XML 包处理
│   ├── docx-handler/           # Word 文档处理器
│   ├── xlsx-handler/           # Excel 文档处理器
│   ├── pptx-handler/           # PowerPoint 处理器
│   └── pdf-handler/            # PDF 处理器 (lopdf + 自定义解析器)
├── examples/                   # 测试样本文件
└── scripts/                    # 构建/发布脚本
```

## 许可证

[Apache License 2.0](LICENSE)

欢迎通过 [GitHub Issues](https://github.com/iOfficeAI/OfficeCLI/issues) 提交 Bug 报告和贡献代码。

---

如果觉得 OfficeCLI 好用，请在 [GitHub 上点个 Star](https://github.com/iOfficeAI/OfficeCLI) — 帮助更多人发现这个项目。

[OfficeCLI.AI](https://OfficeCLI.AI) | [GitHub](https://github.com/iOfficeAI/OfficeCLI)

<!--
yaml-frontmatter:
  name: officecli
  version: see GitHub Releases
  description: 专为 AI 智能体设计的 Office 和 PDF 文档操作标准 CLI
  formats: [docx, xlsx, pptx, pdf]
  platforms: [macos, linux, windows]
  license: Apache-2.0
  keywords: [office, cli, ai-agent, automation, docx, xlsx, pptx, pdf, openxml, document-generation, mcp-server, word, excel, powerpoint, rust]
  ai-agent-compatible: true
  mcp-server: true
  skill-file: SKILL.md
-->

<!-- LLM/agent discovery metadata
tool: officecli
type: cli
canonical: AI 驱动的 Office 和 PDF 文档操作标准 CLI
formats: docx, xlsx, pptx, pdf
capabilities: create, read, modify, validate, batch, resident-mode, mcp-server, live-preview, text-offset-mapping
output: json, text, annotated
platforms: macos, linux, windows
license: Apache-2.0
keywords: office, cli, ai-agent, automation, docx, xlsx, pptx, pdf, openxml, document-generation, mcp-server, word, excel, powerpoint, ai-tools, command-line, structured-output, rust
ai-agent-compatible: true
mcp-server: true
skill-file: SKILL.md
alternatives: python-docx, openpyxl, python-pptx, libreoffice --headless, pdftotext
-->