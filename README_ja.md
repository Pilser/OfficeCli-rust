# OfficeCLI (Rust)

> **AI エージェント向けの純 Rust CLI — Office 文書と PDF の作成・読み取り・修正・レンダリング。**

**あらゆる AI エージェントに Word、Excel、PowerPoint、PDF の構造化制御を — たった一行のコードで。**

オープンソース。単一バイナリ。Office 不要。ランタイム依存なし。macOS、Linux、Windows 対応。

[![GitHub Release](https://img.shields.io/github/v/release/RainLib/OfficeCli-rust)](https://github.com/RainLib/OfficeCli-rust/releases)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

[English](README.md) | [中文](README_zh.md) | **日本語** | [한국어](README_ko.md)

## 本リポジトリについて

これは **[RainLib/OfficeCli-rust](https://github.com/RainLib/OfficeCli-rust)** — [OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) の **Rust 再実装**です。OfficeCLI は [iOfficeAI](https://github.com/iOfficeAI) による C#/.NET 製のオープンソース Office 自動化 CLI です。

| | **本リポジトリ (Rust)** | **[上流 (C#)](https://github.com/iOfficeAI/OfficeCLI)** |
|---|---|---|
| リポジトリ | [RainLib/OfficeCli-rust](https://github.com/RainLib/OfficeCli-rust) | [iOfficeAI/OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) |
| 言語 | 純 Rust | C# / .NET（自己完結バイナリ） |
| バージョン | v0.1.x（初期段階） | v1.0.x（成熟、6k+ stars） |
| ランタイム | なし — ネイティブバイナリ | バイナリ内蔵 .NET |
| PDF サポート | ✅ 読み取り / 修正 / プレビュー | プラグイン経由 |
| 目的 | 軽量・監査可能・埋め込み可能な Rust コア | フル機能の本番 CLI + エコシステム |

Rust 版は同じ **CLI 思想** — パスベース DOM 操作、JSON 出力、TextOffsetMap、三層アーキテクチャ、MCP サーバー、ライブ HTML プレビュー — を共有しますが、機能の幅ではまだ上流に追いついています。最大互換性が必要なら上流を、**依存関係ゼロの Rust バイナリ**や Rust 実装への貢献が必要なら本リポジトリをご利用ください。

## 対応フォーマット

| フォーマット | 読み取り | 修正 | 作成 | テキスト/オフセットマッピング | レガシー変換 |
|-------------|---------|------|------|---------------------------|-------------|
| Word (.docx) | ✅ | ✅ | ✅ | ✅ | ✅ .doc → .docx |
| Excel (.xlsx) | ✅ | ✅ | ✅ | ✅ | ✅ .xls → .xlsx |
| PowerPoint (.pptx) | ✅ | ✅ | ✅ | ✅ | ✅ .ppt → .pptx |
| PDF (.pdf) | ✅ | ✅（テキスト置換、ページ削除） | ✅ | ✅ | — |

## AI エージェント向け — テキスト/オフセット → パスマッピング

対応フォーマットすべてが **TextOffsetMap** を出力 — 全テキストと文字オフセット→パスマッピング。エージェントはマップを読み、変更箇所を特定し、正確なパス（例: `/body/p[3]/r[1]`）で `set` を呼び出します。正規表現の推測は不要です。

```bash
officecli extract-text report.docx --with-offsets --json
```

```json
{
  "full_text": "こんにちは世界\n二番目の段落",
  "spans": [
    { "start": 0, "end": 5, "path": "/body/p[1]/r[1]", "text": "こんにちは", "element_type": "run" },
    { "start": 5, "end": 7, "path": "/body/p[1]/r[2]", "text": "世界", "element_type": "run" },
    { "start": 7, "end": 13, "path": "/body/p[2]/r[1]", "text": "二番目の段落", "element_type": "run" }
  ],
  "meta": { "format": "docx", "total_chars": 13, "total_spans": 3 }
}
```

**エージェント設定** — スキルファイルをコーディングエージェントに渡す：

```bash
curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/SKILL.md
```

またはワンステップでバイナリとスキルをインストール（[インストール](#インストール)参照）。

## クイックスタート

```bash
# 1. インストール（macOS / Linux）
curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.sh | bash
# Windows (PowerShell):
#   irm https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.ps1 | iex

# 2. 空の PowerPoint を作成
officecli create deck.pptx

# 3. スライドを追加
officecli add deck.pptx / --type slide --prop title="Hello, World!"

# 4. HTML プレビュー
officecli view deck.pptx --mode html

# 5. ライブプレビュー — 編集のたびに自動更新
officecli watch deck.pptx
```

別ターミナルで `add` / `set` / `remove` のたびに `http://localhost:26315` のブラウザが更新されます。

## なぜ OfficeCLI？

50 行の Python と 3 つのライブラリが必要だった作業が：

```python
from pptx import Presentation
prs = Presentation()
slide = prs.slides.add_slide(prs.slide_layouts[0])
slide.shapes.title.text = "Q4 レポート"
# ... さらに数十行 ...
prs.save("deck.pptx")
```

1 コマンドに：

```bash
officecli add deck.pptx / --type slide --prop title="Q4 レポート"
```

**本 Rust ビルドの主要機能：**

- **作成** 空白文書または構造化コンテンツの追加
- **読み取り** テキスト、アウトライン、統計、注釈ビュー — プレーンテキストまたは `--json`
- **修正** パスベースの `set` / `add` / `remove` / `move`
- **検証** 文書構造と問題の検出
- **抽出** オフセット→パスマッピング付きテキスト
- **レンダリング** HTML/SVG による視覚的フィードバック
- **変換** レガシー `.doc` / `.xls` / `.ppt` を現行形式へ
- **PDF** — 読み取り、プレビュー、テキスト置換、ページ削除
- **バッチ** — 1 サイクルで複数操作
- **MCP** — JSON-RPC で全操作を AI ツールとして公開

## インストール

単一のネイティブバイナリとして配布。純 Rust — .NET、Python、Office 不要。

**ワンライナーインストール：**

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.ps1 | iex
```

特定バージョンを指定：

```bash
OFFICECLI_VERSION=v0.1.1 curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.sh | bash
```

**手動ダウンロード** [GitHub Releases](https://github.com/RainLib/OfficeCli-rust/releases)：

| プラットフォーム | バイナリ |
|----------------|---------|
| macOS Apple Silicon | `officecli-mac-arm64` |
| macOS Intel | `officecli-mac-x64` |
| Linux x64 | `officecli-linux-x64` |
| Linux ARM64 | `officecli-linux-arm64` |
| Linux Alpine x64 | `officecli-linux-alpine-x64` |
| Windows x64 | `officecli-win-x64.exe` |
| Windows ARM64 | `officecli-win-arm64.exe` |

```bash
./scripts/download.sh
./scripts/download.sh v0.1.1 all
gh release download v0.1.1 --repo RainLib/OfficeCli-rust --pattern 'officecli-*'
```

> **リリース注意：** `v*` タグのプッシュで CI がバイナリを **Draft** Release にアップロードします。[Releases](https://github.com/RainLib/OfficeCli-rust/releases) で Publish してから `latest` URL を使用してください。タグは `github` リモート（`git push github v0.1.2`）にプッシュしてください。

インストール確認：`officecli --version`

## 主な機能

### 三層アーキテクチャ

シンプルに始め、必要な時だけ深く。

| レイヤー | 用途 | コマンド |
|---------|------|---------|
| **L1：読み取り** | セマンティックビュー | `view`（text、annotated、outline、stats、issues、html、svg） |
| **L2：DOM** | 構造化要素操作 | `get`、`query`、`set`、`add`、`remove`、`move` |
| **L3：生 XML** | XPath 直接アクセス | `raw`、`raw-set`、`validate` |

```bash
officecli view report.docx --mode annotated
officecli query report.docx paragraph
officecli raw deck.pptx 'ppt/slides/slide1.xml'
```

### ライブプレビューとレンダリング

```bash
officecli view deck.pptx --mode html
officecli view deck.pptx --mode svg
officecli watch deck.pptx
```

### フォーマット変換

```bash
officecli convert old.doc
officecli convert old.xls -o new.xlsx
officecli convert old.ppt --engine oxide
```

| エンジン | 忠実度 | 速度 | 依存関係 |
|---------|--------|------|---------|
| `libreoffice`（デフォルト） | ~1:1 | 遅い | LibreOffice（~700MB） |
| `oxide` | 低め | 高速 | なし（純 Rust） |

### レジデントモードとバッチ

```bash
officecli open report.docx
officecli set report.docx /body/p[1]/r[1] --prop text="更新済み"
officecli save report.docx
officecli close report.docx
```

### PDF サポート

```bash
officecli view report.pdf --mode text
officecli extract-text report.pdf --with-offsets --json
officecli set report.pdf '/page[1]' --prop text="新しいコンテンツ"
officecli remove report.pdf '/page[3]'
officecli save report.pdf
```

## AI 統合

### MCP サーバー

```bash
officecli mcp
```

### 組み込みヘルプ

```bash
officecli --help
officecli help docx paragraph
officecli help xlsx cell --json
```

## 比較

### 従来ツールとの比較

| | OfficeCLI (Rust) | [OfficeCLI (C#)](https://github.com/iOfficeAI/OfficeCLI) | Microsoft Office | python-docx / openpyxl |
|---|---|---|---|---|
| オープンソース＆無料 | ✅ | ✅ | ✗ | ✅ |
| AI ネイティブ CLI + JSON | ✅ | ✅ | ✗ | ✗ |
| ゼロランタイム（単一バイナリ） | ✅ (Rust) | ✅ (.NET 内蔵) | ✗ | ✗ |
| Word + Excel + PowerPoint + PDF | ✅ | ✅ | ✅ | 複数ライブラリ |
| テキスト/オフセット → パス | ✅ | ✅ | ✗ | ✗ |
| ライブ HTML プレビュー | ✅ | ✅ | ✗ | ✗ |
| MCP サーバー | ✅ | ✅ | ✗ | ✗ |
| ヘッドレス / CI / Docker | ✅ | ✅ | ✗ | ✅ |

### 上流 OfficeCLI (C#) との比較

| 機能 | 上流 (C#) | 本リポジトリ (Rust) |
|------|-----------|-------------------|
| テンプレート `merge` | ✅ | 🔜 |
| `view screenshot` | ✅ | 🔜 |
| `swap`、`refresh`、`plugins` | ✅ | 🔜 |
| `officecli install` | ✅ | `install.sh` / `install.ps1` を使用 |
| 数式エンジン（150+ 関数） | ✅ | 部分対応 |
| ピボット、Morph、3D モデル | ✅ | 部分対応 / 開発中 |
| Python SDK | ✅ | 🔜 |
| AionUi GUI | ✅ | 該当なし |
| Wiki と成熟したエコシステム | ✅ | 初期段階 |

詳細は [iOfficeAI/OfficeCLI Wiki](https://github.com/iOfficeAI/OfficeCLI/wiki) を参照。

## コマンドリファレンス

| コマンド | 説明 |
|---------|------|
| `create` | 空白 `.docx` / `.xlsx` / `.pptx` / `.pdf` を作成 |
| `view` | コンテンツ表示（text、annotated、outline、stats、issues、html、svg） |
| `get` | 要素と子要素を取得 |
| `query` | CSS 風クエリ |
| `set` / `add` / `remove` / `move` | 要素の変更 |
| `save` / `validate` / `extract-text` / `convert` / `batch` / `dump` | 各種操作 |
| `raw` / `raw-set` | 生 XML 操作 |
| `watch` / `unwatch` | ライブプレビュー |
| `open` / `close` | レジデントモード（Unix） |
| `info` / `mcp` | 情報表示 / MCP サーバー |

グローバルフラグ：`--json`

## ユースケース

**開発者** — CI/CD 自動化、Docker ヘッドレス処理、軽量 Rust バイナリの埋め込み

**AI エージェント** — TextOffsetMap による精密編集、`watch` による視覚フィードバック、MCP 統合

**チーム** — 監査可能な Rust コードによる内部自動化、上流からの段階的移行

## ソースからビルド

[Rust](https://rustup.rs/) 1.75+（CI は 1.90.0）が必要。

```bash
git clone https://github.com/RainLib/OfficeCli-rust.git
cd OfficeCli-rust
cargo build --release
```

```bash
make dist
make download VERSION=v0.1.1 PLATFORM=all
make smoke
```

## プロジェクト構成

```
OfficeCli-rust/
├── Cargo.toml
├── install.sh / install.ps1
├── scripts/download.sh
├── SKILL.md
├── crates/（officecli, handler-common, oxml, docx/xlsx/pptx/pdf-handler）
├── examples/
└── skills/
```

## コントリビューション

[CONTRIBUTING.md](CONTRIBUTING.md) を参照。Issue：[GitHub Issues](https://github.com/RainLib/OfficeCli-rust/issues)

上流参考実装：[iOfficeAI/OfficeCLI](https://github.com/iOfficeAI/OfficeCLI)

## ライセンス

[Apache License 2.0](LICENSE)

---

[GitHub — RainLib/OfficeCli-rust](https://github.com/RainLib/OfficeCli-rust) | [上流 — iOfficeAI/OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) | [Releases](https://github.com/RainLib/OfficeCli-rust/releases)

<!-- LLM/agent discovery metadata
tool: officecli
repo: RainLib/OfficeCli-rust
upstream: iOfficeAI/OfficeCLI
type: cli
language: rust
formats: docx, xlsx, pptx, pdf
license: Apache-2.0
skill-file: SKILL.md
-->
