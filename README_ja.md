# OfficeCLI

> **OfficeCLI は世界初にして最高の、AI エージェント向けに設計された Office スイートです。**

**あらゆる AI エージェントに Word、Excel、PowerPoint、PDF の完全な制御権を — たった一行のコードで。**

オープンソース。単一バイナリ。Office のインストール不要。依存関係ゼロ。全プラットフォーム対応。

**エージェントフレンドリーなレンダリングエンジンを内蔵** — エージェントは自分が作ったものを "見る" ことができ、Office 不要。`.docx` / `.xlsx` / `.pptx` / `.pdf` を HTML または SVG にレンダリングし、*レンダリング → 見る → 修正* のループはバイナリが動くあらゆる場所で完結します。

[![GitHub Release](https://img.shields.io/github/v/release/iOfficeAI/OfficeCLI)](https://github.com/iOfficeAI/OfficeCLI/releases)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

[English](README.md) | [中文](README_zh.md) | **日本語** | [한국어](README_ko.md)

<p align="center">
  <strong>💬 コミュニティ:</strong> <a href="https://discord.gg/2QAwJn7Egx" target="_blank">Discord</a>
</p>

<p align="center">
  <img src="assets/ppt-process.webp" alt="OfficeCLI で PowerPoint プレゼンテーションを作成" width="100%">
</p>

<p align="center"><em><a href="https://github.com/iOfficeAI/AionUi">AionUi</a> で OfficeCLI を使った PPT 作成プロセス</em></p>

## 対応フォーマット

| フォーマット | 読み取り | 修正 | 作成 | テキスト/オフセットマッピング |
|-------------|---------|------|------|---------------------------|
| Word (.docx) | ✅ | ✅ | ✅ | ✅ |
| Excel (.xlsx) | ✅ | ✅ | ✅ | ✅ |
| PowerPoint (.pptx) | ✅ | ✅ | ✅ | ✅ |
| PDF (.pdf) | ✅ | ✅ (テキスト置換、ページ削除) | — | ✅ |

## AI エージェント向け — テキスト/オフセット → パスマッピング

すべてのドキュメントは **TextOffsetMap** を出力 — 全テキストと文字オフセット→パス ID マッピング。AI エージェントはマップを読み、変更が必要なテキスト位置を見つけ、正確なドキュメントパス（例: `/body/p[3]/r[1]`）を取得し、`set` で正確に修正します。推測不要、正規表現パース不要。

```bash
officecli extract-text report.docx --with-offsets --json
```

```json
{
  "full_text": "こんにちは世界\n二番目の段落",
  "spans": [
    {"start": 0, "end": 5, "path": "/body/p[1]/r[1]", "text": "こんにちは", "element_type": "run"},
    {"start": 5, "end": 7, "path": "/body/p[1]/r[2]", "text": "世界", "element_type": "run"},
    {"start": 7, "end": 13, "path": "/body/p[2]/r[1]", "text": "二番目の段落", "element_type": "run"}
  ],
  "meta": {"format": "docx", "total_chars": 13, "total_spans": 3}
}
```

4つのフォーマットすべてで動作 — docx、xlsx、pptx、pdf。

## 開発者向け — 30秒でライブ体験

```bash
# 1. インストール（macOS / Linux）
curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
# Windows: GitHub Releases からダウンロード

# 2. 空の PowerPoint を作成
officecli create deck.pptx

# 3. ライブプレビューを開始 — ブラウザで http://localhost:26315 が開きます
officecli watch deck.pptx

# 4. 別のターミナルを開いてスライドを追加 — ブラウザが即座に更新されます
officecli add deck.pptx / --type slide --prop title="Hello, World!"
```

## クイックスタート

```bash
# プレゼンテーションを作成してコンテンツを追加
officecli create deck.pptx
officecli add deck.pptx / --type slide --prop title="Q4 レポート"

# アウトラインを表示
officecli view deck.pptx outline

# HTML で表示 — ブラウザでレンダリングされたプレビューを開きます
officecli view deck.pptx html

# 任意の要素の構造化データを取得
officecli get deck.pptx '/slide[1]' --json

# PDF 文書を表示
officecli view report.pdf --mode text
officecli get report.pdf '/page[1]' --json

# テキストとオフセットマッピングを抽出（AI エージェントの位置特定用）
officecli extract-text report.docx --with-offsets --json
```

## なぜ OfficeCLI？

**OfficeCLI でできること：**

- **作成** ドキュメント — 空白またはコンテンツ付き
- **読み取り** テキスト、構造、スタイル — プレーンテキストまたは構造化 JSON
- **修正** 任意の要素 — テキスト、スタイル、レイアウト
- **再構成** コンテンツ — 要素の追加、削除、移動、コピー
- **検証** ドキュメント構造、問題の検出
- **抽出** テキストとオフセット→パスマッピング、AI エージェントの位置特定用
- **レンダリング** ドキュメントを HTML/SVG にレンダリング、ビジュアルプレビュー用
- **PDF サポート** — 読み取り、表示、テキスト修正、ページ削除、画像抽出

## インストール

単一のネイティブバイナリとして配布。純 Rust 実装、クロスプラットフォーム、ランタイム依存関係なし。

**ワンライナーインストール：**

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
```

**または手動ダウンロード** [GitHub Releases](https://github.com/iOfficeAI/OfficeCLI/releases)：

| プラットフォーム | バイナリ |
|----------------|---------|
| macOS Apple Silicon | `officecli-mac-arm64` |
| macOS Intel | `officecli-mac-x64` |
| Linux x64 | `officecli-linux-x64` |
| Linux ARM64 | `officecli-linux-arm64` |
| Windows x64 | `officecli-win-x64.exe` |
| Windows ARM64 | `officecli-win-arm64.exe` |

インストール確認：`officecli --version`

## 主な機能

### 三層アーキテクチャ

シンプルに始めて、必要な時だけ深く。

| レイヤー | 用途 | コマンド |
|---------|------|---------|
| **L1：読み取り** | コンテンツのセマンティックビュー | `view`（text、annotated、outline、stats、issues、html、svg） |
| **L2：DOM** | 構造化された要素操作 | `get`、`query`、`set`、`add`、`remove`、`move`、`copy` |
| **L3：生 XML** | XPath による直接アクセス — 万能フォールバック | `raw`、`raw-set`、`add-part`、`validate` |

```bash
# L1 — 高レベルビュー
officecli view report.docx annotated
officecli view budget.xlsx stats
officecli view report.pdf text

# L2 — 要素レベルの操作
officecli query report.docx "paragraph"
officecli add budget.xlsx / --type sheet --prop name="Q2 レポート"
officecli remove report.pptx '/slide[3]'

# L3 — L2 では足りない時に生 XML
officecli raw deck.pptx 'ppt/slides/slide1.xml'
officecli raw-set report.docx document --xpath "//w:p[1]" --action append --xml '<w:r><w:t>注入テキスト</w:t></w:r>'
```

### レジデントモードとバッチ

複数ステップのワークフローでは、レジデントモードがドキュメントをメモリに保持。バッチモードは一度の open/save サイクルで複数操作を実行します。

```bash
# レジデントモード — Unix Domain Socket 経由で遅延ほぼゼロ
officecli open report.docx
officecli set report.docx /body/p[1]/r[1] --prop text="更新済み"
officecli close report.docx

# バッチモード — アトミックなマルチコマンド実行
echo '[{"command":"set","path":"/slide[1]/shape[1]","props":{"text":"こんにちは"}}]' \
  | officecli batch deck.pptx --json
```

### PDF サポート

PDF 文書の読み取り、表示、修正：

```bash
# PDF テキストを読み取る
officecli view report.pdf text
officecli view report.pdf outline

# ページコンテンツを取得
officecli get report.pdf '/page[1]'

# テキストとオフセットマッピングを抽出
officecli extract-text report.pdf --with-offsets --json

# PDF を修正 — ページのテキストを置換
officecli set report.pdf '/page[1]' --prop text="新しいコンテンツ"
officecli save report.pdf

# ページを削除
officecli remove report.pdf '/page[3]'
officecli save report.pdf

# SVG プレビューにレンダリング
officecli view report.pdf svg
```

### テキスト/オフセット → パスマッピング

すべてのフォーマットでオフセット→パスマッピングを出力し、AI エージェントがテキストを正確に特定して修正できるようにします：

```bash
# Docx：文字オフセットが段落/ランパスにマッピング
officecli extract-text report.docx --with-offsets --json

# Xlsx：セルオフセットがシート/セルパスにマッピング
officecli extract-text budget.xlsx --with-offsets --json

# Pptx：テキストオフセットがスライド/シェイプ/段落パスにマッピング
officecli extract-text deck.pptx --with-offsets --json

# Pdf：文字オフセットがページ/テキストブロックパスにマッピング
officecli extract-text report.pdf --with-offsets --json
```

## AI 統合

### MCP サーバー

組み込み [MCP](https://modelcontextprotocol.io) サーバー：

```bash
officecli mcp         # MCP stdio サーバーを起動
```

JSON-RPC で全ドキュメント操作を公開 — シェルアクセス不要。

### 組み込みヘルプ

```bash
officecli --help                     # 完全なコマンド概要
officecli view --help                # view コマンドの詳細
officecli get --help                 # get コマンドの詳細
```

## コマンドリファレンス

| コマンド | 説明 |
|---------|------|
| `create` | 空白の .docx、.xlsx、.pptx を作成 |
| `view` | コンテンツを表示（モード：text、annotated、outline、stats、issues、html、svg） |
| `get` | 要素と子要素を取得（`--depth N`、`--json`） |
| `query` | CSS スタイルのクエリ |
| `set` | 要素のプロパティを変更 |
| `add` | 要素を追加 |
| `remove` | 要素を削除 |
| `move` | 要素を移動 |
| `copy` | ソースからターゲットに要素をコピー |
| `validate` | ドキュメント構造を検証 |
| `extract-text` | テキストとオフセット→パスマッピングを抽出（`--with-offsets`、`--json`） |
| `batch` | 一度のサイクルで複数操作を実行 |
| `dump` | ドキュメントを再生可能な JSON にシリアライズ |
| `raw` | ドキュメントパートの生 XML を表示 |
| `raw-set` | XPath で生 XML を変更 |
| `watch` | ライブ HTML プレビュー、自動更新 |
| `open` | レジデントモードを開始 |
| `close` | 保存してレジデントモードを終了 |
| `mcp` | AI ツール統合用の MCP サーバーを起動 |

## 比較

| | OfficeCLI | Microsoft Office | LibreOffice | python-docx / openpyxl |
|---|---|---|---|---|
| オープンソース＆無料 | ✓ (Apache 2.0) | ✗（有料ライセンス） | ✓ | ✓ |
| AI ネイティブ CLI + JSON | ✓ | ✗ | ✗ | ✗ |
| ゼロインストール（単一バイナリ） | ✓ | ✗ | ✗ | ✗（Python + pip 必要） |
| PDF 読み取り/修正 | ✓ | ✗ | ✓ | ✗ |
| テキスト/オフセット → パスマッピング | ✓ | ✗ | ✗ | ✗ |
| パスベースの要素アクセス | ✓ | ✗ | ✗ | ✗ |
| 生 XML フォールバック | ✓ | ✗ | ✗ | 部分対応 |
| クロスプラットフォームネイティブ | ✓ (Rust) | Windows/Mac | ✓ | ✓ |
| Word + Excel + PowerPoint + PDF | ✓ | ✓ | ✓ | 複数ライブラリが必要 |

## ソースからビルド

[Rust](https://rustup.rs/)（1.75+）が必要。

```bash
cargo build --release
# バイナリは target/release/officecli に生成
```

他のプラットフォーム向けにクロスコンパイル：

```bash
# macOS ARM
cargo build --release --target aarch64-apple-darwin

# Linux x64（クロスリンカーが必要）
cargo build --release --target x86_64-unknown-linux-gnu

# Windows x64（クロスリンカーが必要）
cargo build --release --target x86_64-pc-windows-msvc
```

## プロジェクト構成

```
OfficeCLI/
├── Cargo.toml                 # Workspace ルート
├── crates/
│   ├── officecli/              # CLI エントリ + コマンド
│   ├── handler-common/         # 共有 trait + 型
│   ├── oxml/                   # OOXML ZIP/XML パッケージ処理
│   ├── docx-handler/           # Word ドキュメントハンドラ
│   ├── xlsx-handler/           # Excel ドキュメントハンドラ
│   ├── pptx-handler/           # PowerPoint ハンドラ
│   └── pdf-handler/            # PDF ハンドラ (lopdf + カスタムパーサー)
├── examples/                   # テスト用サンプルファイル
└── scripts/                    # ビルド/リリーススクリプト
```

## ライセンス

[Apache License 2.0](LICENSE)

バグ報告やコントリビューションは [GitHub Issues](https://github.com/iOfficeAI/OfficeCLI/issues) まで。

---

OfficeCLI が役に立ったら、ぜひ [GitHub でスターを付けてください](https://github.com/iOfficeAI/OfficeCLI) — より多くの人にプロジェクトを届ける力になります。

[OfficeCLI.AI](https://OfficeCLI.AI) | [GitHub](https://github.com/iOfficeAI/OfficeCLI)

<!--
yaml-frontmatter:
  name: officecli
  version: see GitHub Releases
  description: AI エージェント向け Office & PDF ドキュメント操作の標準 CLI
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
canonical: AI 駆動の Office & PDF ドキュメント操作標準 CLI
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