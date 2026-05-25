# OfficeCLI

> **OfficeCLI는 세계 최초이자 최고의, AI 에이전트를 위해 설계된 Office 스위트입니다.**

**모든 AI 에이전트에게 Word, Excel, PowerPoint, PDF의 완전한 제어권을 — 단 한 줄의 코드로.**

오픈소스. 단일 바이너리. Office 설치 불필요. 의존성 제로. 모든 플랫폼 지원.

**에이전트 친화적 렌더링 엔진 내장** — 에이전트가 자신이 만든 것을 "볼" 수 있고, Office 불필요. `.docx` / `.xlsx` / `.pptx` / `.pdf`를 HTML 또는 SVG로 렌더링하며, *렌더링 → 보기 → 수정* 루프는 바이너리가 실행되는 어디서나 닫힙니다.

[![GitHub Release](https://img.shields.io/github/v/release/iOfficeAI/OfficeCLI)](https://github.com/iOfficeAI/OfficeCLI/releases)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

[English](README.md) | [中文](README_zh.md) | [日本語](README_ja.md) | **한국어**

<p align="center">
  <strong>💬 커뮤니티:</strong> <a href="https://discord.gg/2QAwJn7Egx" target="_blank">Discord</a>
</p>

<p align="center">
  <img src="assets/ppt-process.webp" alt="OfficeCLI로 PowerPoint 프레젠테이션 생성" width="100%">
</p>

<p align="center"><em><a href="https://github.com/iOfficeAI/AionUi">AionUi</a>에서 OfficeCLI로 PPT 제작 과정</em></p>

## 지원 형식

| 형식 | 읽기 | 수정 | 생성 | 텍스트/오프셋 매핑 |
|------|------|------|------|-------------------|
| Word (.docx) | ✅ | ✅ | ✅ | ✅ |
| Excel (.xlsx) | ✅ | ✅ | ✅ | ✅ |
| PowerPoint (.pptx) | ✅ | ✅ | ✅ | ✅ |
| PDF (.pdf) | ✅ | ✅ (텍스트 바꾸기, 페이지 삭제) | — | ✅ |

## AI 에이전트용 — 텍스트/오프셋 → 경로 매핑

모든 문서는 **TextOffsetMap**을 출력 — 전체 텍스트와 문자 오프셋→경로 ID 매핑. AI 에이전트는 맵을 읽고, 변경해야 할 텍스트 위치를 찾고, 정확한 문서 경로(예: `/body/p[3]/r[1]`)를 가져와서 `set`으로 정확히 수정합니다. 추측 불필요, 정규식 파싱 불필요.

```bash
officecli extract-text report.docx --with-offsets --json
```

```json
{
  "full_text": "안녕하세요 세계\n두 번째 단락",
  "spans": [
    {"start": 0, "end": 5, "path": "/body/p[1]/r[1]", "text": "안녕하세요", "element_type": "run"},
    {"start": 5, "end": 7, "path": "/body/p[1]/r[2]", "text": "세계", "element_type": "run"},
    {"start": 7, "end": 13, "path": "/body/p[2]/r[1]", "text": "두 번째 단락", "element_type": "run"}
  ],
  "meta": {"format": "docx", "total_chars": 13, "total_spans": 3}
}
```

4가지 형식 모두 지원 — docx, xlsx, pptx, pdf.

## 개발자용 — 30초 만에 라이브로 확인

```bash
# 1. 설치 (macOS / Linux)
curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
# Windows: GitHub Releases에서 다운로드

# 2. 빈 PowerPoint 생성
officecli create deck.pptx

# 3. 라이브 미리보기 시작 — 브라우저에서 http://localhost:26315 열기
officecli watch deck.pptx

# 4. 다른 터미널을 열고 슬라이드 추가 — 브라우저 즉시 업데이트
officecli add deck.pptx / --type slide --prop title="Hello, World!"
```

## 빠른 시작

```bash
# 프레젠테이션을 생성하고 콘텐츠 추가
officecli create deck.pptx
officecli add deck.pptx / --type slide --prop title="Q4 보고서"

# 개요 보기
officecli view deck.pptx outline

# HTML로 보기 — 브라우저에서 렌더링된 미리보기 열기
officecli view deck.pptx html

# 모든 요소의 구조화된 데이터 가져오기
officecli get deck.pptx '/slide[1]' --json

# PDF 문서 보기
officecli view report.pdf --mode text
officecli get report.pdf '/page[1]' --json

# 텍스트와 오프셋 매핑 추출 (AI 에이전트 위치 지정용)
officecli extract-text report.docx --with-offsets --json
```

## 왜 OfficeCLI인가?

**OfficeCLI로 할 수 있는 것:**

- **생성** 문서 — 빈 문서 또는 콘텐츠 포함
- **읽기** 텍스트, 구조, 스타일 — 일반 텍스트 또는 구조화된 JSON
- **수정** 모든 요소 — 텍스트, 스타일, 레이아웃
- **재구성** 콘텐츠 — 요소 추가, 삭제, 이동, 복사
- **검증** 문서 구조, 문제 감지
- **추출** 텍스트와 오프셋→경로 매핑, AI 에이전트 위치 지정용
- **렌더링** 문서를 HTML/SVG로, 시각적 미리보기용
- **PDF 지원** — 읽기, 보기, 텍스트 수정, 페이지 삭제, 이미지 추출

## 설치

단일 네이티브 바이너리로 제공. 순 Rust 구현, 크로스 플랫폼, 런타임 의존성 없음.

**원라인 설치:**

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/iOfficeAI/OfficeCLI/main/install.sh | bash
```

**또는 수동 다운로드** [GitHub Releases](https://github.com/iOfficeAI/OfficeCLI/releases):

| 플랫폼 | 바이너리 |
|--------|---------|
| macOS Apple Silicon | `officecli-mac-arm64` |
| macOS Intel | `officecli-mac-x64` |
| Linux x64 | `officecli-linux-x64` |
| Linux ARM64 | `officecli-linux-arm64` |
| Windows x64 | `officecli-win-x64.exe` |
| Windows ARM64 | `officecli-win-arm64.exe` |

설치 확인: `officecli --version`

## 주요 기능

### 3계층 아키텍처

간단하게 시작하고, 필요할 때만 깊이 들어가세요.

| 레이어 | 용도 | 명령어 |
|--------|------|--------|
| **L1: 읽기** | 콘텐츠의 시맨틱 뷰 | `view` (text, annotated, outline, stats, issues, html, svg) |
| **L2: DOM** | 구조화된 요소 작업 | `get`, `query`, `set`, `add`, `remove`, `move`, `copy` |
| **L3: 원시 XML** | XPath 직접 접근 — 범용 폴백 | `raw`, `raw-set`, `add-part`, `validate` |

```bash
# L1 — 고수준 뷰
officecli view report.docx annotated
officecli view budget.xlsx stats
officecli view report.pdf text

# L2 — 요소 수준 작업
officecli query report.docx "paragraph"
officecli add budget.xlsx / --type sheet --prop name="Q2 보고서"
officecli remove report.pptx '/slide[3]'

# L3 — L2로 부족할 때 원시 XML
officecli raw deck.pptx 'ppt/slides/slide1.xml'
officecli raw-set report.docx document --xpath "//w:p[1]" --action append --xml '<w:r><w:t>삽입 텍스트</w:t></w:r>'
```

### 레지던트 모드와 배치

다단계 워크플로우에서 레지던트 모드는 문서를 메모리에 유지합니다. 배치 모드는 한 번의 open/save 사이클에서 여러 작업을 실행합니다.

```bash
# 레지던트 모드 — Unix Domain Socket으로 거의 제로 지연
officecli open report.docx
officecli set report.docx /body/p[1]/r[1] --prop text="업데이트됨"
officecli close report.docx

# 배치 모드 — 원자적 다중 명령 실행
echo '[{"command":"set","path":"/slide[1]/shape[1]","props":{"text":"안녕하세요"}}]' \
  | officecli batch deck.pptx --json
```

### PDF 지원

PDF 문서 읽기, 보기, 수정:

```bash
# PDF 텍스트 읽기
officecli view report.pdf text
officecli view report.pdf outline

# 페이지 콘텐츠 가져오기
officecli get report.pdf '/page[1]'

# 텍스트와 오프셋 매핑 추출
officecli extract-text report.pdf --with-offsets --json

# PDF 수정 — 페이지 텍스트 바꾸기
officecli set report.pdf '/page[1]' --prop text="새 콘텐츠"
officecli save report.pdf

# 페이지 삭제
officecli remove report.pdf '/page[3]'
officecli save report.pdf

# SVG 미리보기로 렌더링
officecli view report.pdf svg
```

### 텍스트/오프셋 → 경로 매핑

모든 형식에서 오프셋→경로 매핑을 출력하여 AI 에이전트가 텍스트를 정확히 찾고 수정할 수 있습니다:

```bash
# Docx: 문자 오프셋이 단락/런 경로에 매핑
officecli extract-text report.docx --with-offsets --json

# Xlsx: 셀 오프셋이 시트/셀 경로에 매핑
officecli extract-text budget.xlsx --with-offsets --json

# Pptx: 텍스트 오프셋이 슬라이드/도형/단락 경로에 매핑
officecli extract-text deck.pptx --with-offsets --json

# Pdf: 문자 오프셋이 페이지/텍스트블록 경로에 매핑
officecli extract-text report.pdf --with-offsets --json
```

## AI 통합

### MCP 서버

내장 [MCP](https://modelcontextprotocol.io) 서버:

```bash
officecli mcp         # MCP stdio 서버 시작
```

JSON-RPC로 모든 문서 작업 제공 — 셸 접근 불필요.

### 내장 도움말

```bash
officecli --help                     # 전체 명령어 개요
officecli view --help                # view 명령어 상세
officecli get --help                 # get 명령어 상세
```

## 명령어 참조

| 명령어 | 설명 |
|--------|------|
| `create` | 빈 .docx, .xlsx, .pptx 생성 |
| `view` | 콘텐츠 보기 (모드: text, annotated, outline, stats, issues, html, svg) |
| `get` | 요소와 하위 요소 가져오기 (`--depth N`, `--json`) |
| `query` | CSS 스타일 쿼리 |
| `set` | 요소 속성 수정 |
| `add` | 요소 추가 |
| `remove` | 요소 삭제 |
| `move` | 요소 이동 |
| `copy` | 소스에서 타겟으로 요소 복사 |
| `validate` | 문서 구조 검증 |
| `extract-text` | 텍스트와 오프셋→경로 매핑 추출 (`--with-offsets`, `--json`) |
| `batch` | 한 사이클에서 여러 작업 실행 |
| `dump` | 문서를 재생 가능한 JSON으로 직렬화 |
| `raw` | 문서 파트의 원시 XML 보기 |
| `raw-set` | XPath로 원시 XML 수정 |
| `watch` | 라이브 HTML 미리보기, 자동 새로고침 |
| `open` | 레지던트 모드 시작 |
| `close` | 저장하고 레지던트 모드 종료 |
| `mcp` | AI 도구 통합용 MCP 서버 시작 |

## 비교

| | OfficeCLI | Microsoft Office | LibreOffice | python-docx / openpyxl |
|---|---|---|---|---|
| 오픈소스 & 무료 | ✓ (Apache 2.0) | ✗ (유료 라이선스) | ✓ | ✓ |
| AI 네이티브 CLI + JSON | ✓ | ✗ | ✗ | ✗ |
| 제로 설치 (단일 바이너리) | ✓ | ✗ | ✗ | ✗ (Python + pip 필요) |
| PDF 읽기/수정 | ✓ | ✗ | ✓ | ✗ |
| 텍스트/오프셋 → 경로 매핑 | ✓ | ✗ | ✗ | ✗ |
| 경로 기반 요소 접근 | ✓ | ✗ | ✗ | ✗ |
| 원시 XML 폴백 | ✓ | ✗ | ✗ | 부분 지원 |
| 크로스 플랫폼 네이티브 | ✓ (Rust) | Windows/Mac | ✓ | ✓ |
| Word + Excel + PowerPoint + PDF | ✓ | ✓ | ✓ | 여러 라이브러리 필요 |

## 소스에서 빌드

[Rust](https://rustup.rs/) (1.75+) 필요.

```bash
cargo build --release
# 바이너리는 target/release/officecli에 생성
```

다른 플랫폼용 크로스 컴파일:

```bash
# macOS ARM
cargo build --release --target aarch64-apple-darwin

# Linux x64 (크로스 링커 필요)
cargo build --release --target x86_64-unknown-linux-gnu

# Windows x64 (크로스 링커 필요)
cargo build --release --target x86_64-pc-windows-msvc
```

## 프로젝트 구조

```
OfficeCLI/
├── Cargo.toml                 # Workspace 루트
├── crates/
│   ├── officecli/              # CLI 엔트리 + 명령어
│   ├── handler-common/         # 공유 trait + 타입
│   ├── oxml/                   # OOXML ZIP/XML 패키지 처리
│   ├── docx-handler/           # Word 문서 핸들러
│   ├── xlsx-handler/           # Excel 문서 핸들러
│   ├── pptx-handler/           # PowerPoint 핸들러
│   └── pdf-handler/            # PDF 핸들러 (lopdf + 커스텀 파서)
├── examples/                   # 테스트 샘플 파일
└── scripts/                    # 빌드/릴리스 스크립트
```

## 라이선스

[Apache License 2.0](LICENSE)

버그 리포트와 기여는 [GitHub Issues](https://github.com/iOfficeAI/OfficeCLI/issues)로 환영합니다.

---

OfficeCLI가 유용하다면 [GitHub에서 스타를 눌러주세요](https://github.com/iOfficeAI/OfficeCLI) — 더 많은 사람들이 프로젝트를 발견하는 데 도움이 됩니다.

[OfficeCLI.AI](https://OfficeCLI.AI) | [GitHub](https://github.com/iOfficeAI/OfficeCLI)

<!--
yaml-frontmatter:
  name: officecli
  version: see GitHub Releases
  description: AI 에이전트용 Office & PDF 문서 작업 표준 CLI
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
canonical: AI 기반 Office & PDF 문서 작업 표준 CLI
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