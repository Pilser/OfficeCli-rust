# OfficeCLI (Rust)

> **AI 에이전트를 위한 순수 Rust CLI — Office 문서와 PDF 생성, 읽기, 수정, 렌더링.**

**모든 AI 에이전트에게 Word, Excel, PowerPoint, PDF의 구조화된 제어권을 — 단 한 줄의 코드로.**

오픈소스. 단일 바이너리. Office 설치 불필요. 런타임 의존성 없음. macOS, Linux, Windows 지원.

[![GitHub Release](https://img.shields.io/github/v/release/RainLib/OfficeCli-rust)](https://github.com/RainLib/OfficeCli-rust/releases)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

[English](README.md) | [中文](README_zh.md) | [日本語](README_ja.md) | **한국어**

## 이 저장소에 대하여

이것은 **[RainLib/OfficeCli-rust](https://github.com/RainLib/OfficeCli-rust)** — [OfficeCLI](https://github.com/iOfficeAI/OfficeCLI)의 **Rust 재구현**입니다. OfficeCLI는 [iOfficeAI](https://github.com/iOfficeAI)가 C#/.NET으로 만든 오픈소스 Office 자동화 CLI입니다.

| | **이 저장소 (Rust)** | **[업스트림 (C#)](https://github.com/iOfficeAI/OfficeCLI)** |
|---|---|---|
| 저장소 | [RainLib/OfficeCli-rust](https://github.com/RainLib/OfficeCli-rust) | [iOfficeAI/OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) |
| 언어 | 순수 Rust | C# / .NET (자체 포함 바이너리) |
| 버전 | v0.1.x (명령어 패리티) | v1.0.x (성숙, 6k+ stars) |
| 런타임 | 없음 — 네이티브 바이너리 | 바이너리 내장 .NET |
| PDF 지원 | ✅ 읽기 / 수정 / 미리보기 | 플러그인 경유 |
| 목표 | 경량, 감사 가능, 임베딩 가능한 Rust 코어 | 풀기능 프로덕션 CLI + 생태계 |

Rust 버전은 동일한 **CLI 철학** — 경로 기반 DOM 작업, JSON 출력, TextOffsetMap, 3계층 아키텍처, MCP 서버, 라이브 HTML 미리보기 — 을 공유하며, C# 업스트림과의 **명령어 수준 패리티**에 도달했습니다. 남은 차이는 엣지 케이스 충실도와 생태계 도구에 있으며, 명령어 커버리지에는 없습니다. 최대 생태계 통합(AionUi, 플러그인 마켓)이 필요하면 업스트림을, **의존성 없는 Rust 바이너리**나 Rust 구현 기여가 필요하면 이 저장소를 사용하세요.

## 지원 형식

| 형식 | 읽기 | 수정 | 생성 | 텍스트/오프셋 매핑 | 레거시 변환 |
|------|------|------|------|-------------------|------------|
| Word (.docx) | ✅ | ✅ | ✅ | ✅ | ✅ .doc → .docx |
| Excel (.xlsx) | ✅ | ✅ | ✅ | ✅ | ✅ .xls → .xlsx |
| PowerPoint (.pptx) | ✅ | ✅ | ✅ | ✅ | ✅ .ppt → .pptx |
| PDF (.pdf) | ✅ | ✅ (텍스트 바꾸기, 페이지 삭제) | ✅ | ✅ | — |

## AI 에이전트용 — 텍스트/오프셋 → 경로 매핑

지원하는 모든 형식이 **TextOffsetMap**을 출력합니다 — 전체 텍스트와 문자 오프셋→경로 매핑. 에이전트는 맵을 읽고, 변경할 텍스트를 찾고, 정확한 경로(예: `/body/p[3]/r[1]`)로 `set`을 호출합니다. 정규식 추측이 필요 없습니다.

```bash
officecli extract-text report.docx --with-offsets --json
```

```json
{
  "full_text": "안녕하세요 세계\n두 번째 단락",
  "spans": [
    { "start": 0, "end": 5, "path": "/body/p[1]/r[1]", "text": "안녕하세요", "element_type": "run" },
    { "start": 5, "end": 7, "path": "/body/p[1]/r[2]", "text": "세계", "element_type": "run" },
    { "start": 7, "end": 13, "path": "/body/p[2]/r[1]", "text": "두 번째 단락", "element_type": "run" }
  ],
  "meta": { "format": "docx", "total_chars": 13, "total_spans": 3 }
}
```

**에이전트 설정** — 스킬 파일을 코딩 에이전트에 제공:

```bash
curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/SKILL.md
```

또는 한 번에 바이너리와 스킬 설치 ([설치](#설치) 참조).

## 빠른 시작

```bash
# 1. 설치 (macOS / Linux)
curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.sh | bash
# Windows (PowerShell):
#   irm https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.ps1 | iex

# 2. 빈 PowerPoint 생성
officecli create deck.pptx

# 3. 슬라이드 추가
officecli add deck.pptx / --type slide --prop title="Hello, World!"

# 4. HTML 미리보기
officecli view deck.pptx --mode html

# 5. 라이브 미리보기 — 편집마다 자동 새로고침
officecli watch deck.pptx
```

다른 터미널에서 `add` / `set` / `remove` 할 때마다 `http://localhost:26315` 브라우저가 갱신됩니다.

## 왜 OfficeCLI인가?

50줄 Python과 3개 라이브러리가 필요했던 작업이:

```python
from pptx import Presentation
prs = Presentation()
slide = prs.slides.add_slide(prs.slide_layouts[0])
slide.shapes.title.text = "Q4 보고서"
# ... 수십 줄 더 ...
prs.save("deck.pptx")
```

한 줄 명령으로:

```bash
officecli add deck.pptx / --type slide --prop title="Q4 보고서"
```

**본 Rust 빌드의 핵심 기능:**

- **생성** 빈 문서 또는 구조화된 콘텐츠 추가
- **읽기** 텍스트, 개요, 통계, 주석 뷰 — 일반 텍스트 또는 `--json`
- **수정** 경로 기반 `set` / `add` / `remove` / `move`
- **검증** 문서 구조 및 문제 감지
- **추출** 오프셋→경로 매핑이 있는 텍스트
- **렌더링** HTML/SVG 시각적 피드백
- **변환** 레거시 `.doc` / `.xls` / `.ppt` → 현대 형식
- **PDF** — 읽기, 미리보기, 텍스트 바꾸기, 페이지 삭제
- **배치** — 한 사이클에 여러 작업
- **MCP** — JSON-RPC로 모든 작업을 AI 도구로 노출

## 설치

단일 네이티브 바이너리로 제공. 순 Rust — .NET, Python, Office 불필요.

**원라인 설치:**

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.sh | bash

# Windows (PowerShell)
irm https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.ps1 | iex
```

특정 버전 지정:

```bash
OFFICECLI_VERSION=v0.1.2 curl -fsSL https://raw.githubusercontent.com/RainLib/OfficeCli-rust/main/install.sh | bash
```

**수동 다운로드** [GitHub Releases](https://github.com/RainLib/OfficeCli-rust/releases):

| 플랫폼 | 바이너리 |
|--------|---------|
| macOS Apple Silicon | `officecli-mac-arm64` |
| macOS Intel | `officecli-mac-x64` |
| Linux x64 | `officecli-linux-x64` |
| Linux ARM64 | `officecli-linux-arm64` |
| Linux Alpine x64 | `officecli-linux-alpine-x64` |
| Windows x64 | `officecli-win-x64.exe` |
| Windows ARM64 | `officecli-win-arm64.exe` |

```bash
./scripts/download.sh
./scripts/download.sh v0.1.2 all
gh release download v0.1.2 --repo RainLib/OfficeCli-rust --pattern 'officecli-*'
```

> **릴리스 참고:** `v*` 태그 푸시 시 CI가 바이너리를 **Draft** Release에 업로드합니다. [Releases](https://github.com/RainLib/OfficeCli-rust/releases)에서 Publish 후 `latest` URL을 사용하세요. 태그는 `github` 리모트에 푸시하세요 (`git push github v0.1.2`).

설치 확인: `officecli --version`

## 주요 기능

### 3계층 아키텍처

| 레이어 | 용도 | 명령어 |
|--------|------|--------|
| **L1: 읽기** | 시맨틱 뷰 | `view` (text, annotated, outline, stats, issues, html, svg, screenshot, pdf, forms) |
| **L2: DOM** | 구조화된 요소 작업 | `get`, `query`, `set`, `add`, `add-part`, `remove`, `move`, `swap` |
| **L3: 원시 XML** | XPath 직접 접근 | `raw`, `raw-set`, `validate` |

### 라이브 미리보기와 렌더링

```bash
officecli view deck.pptx --mode html
officecli view deck.pptx --mode svg
officecli watch deck.pptx
```

### 형식 변환

```bash
officecli convert old.doc
officecli convert old.xls -o new.xlsx
officecli convert old.ppt --engine oxide
```

| 엔진 | 충실도 | 속도 | 의존성 |
|------|--------|------|--------|
| `libreoffice` (기본) | ~1:1 | 느림 | LibreOffice (~700MB) |
| `oxide` | 낮음 | 빠름 | 없음 (순 Rust) |

### 레지던트 모드와 배치

```bash
officecli open report.docx
officecli set report.docx /body/p[1]/r[1] --prop text="업데이트됨"
officecli save report.docx
officecli close report.docx
```

### PDF 지원

```bash
officecli view report.pdf --mode text
officecli extract-text report.pdf --with-offsets --json
officecli set report.pdf '/page[1]' --prop text="새 콘텐츠"
officecli remove report.pdf '/page[3]'
officecli save report.pdf
```

## AI 통합

### MCP 서버

```bash
officecli mcp
```

### 내장 도움말

```bash
officecli --help
officecli help docx paragraph
officecli help xlsx cell --json
```

## 비교

### 기존 도구와 비교

| | OfficeCLI (Rust) | [OfficeCLI (C#)](https://github.com/iOfficeAI/OfficeCLI) | Microsoft Office | python-docx / openpyxl |
|---|---|---|---|---|
| 오픈소스 & 무료 | ✅ | ✅ | ✗ | ✅ |
| AI 네이티브 CLI + JSON | ✅ | ✅ | ✗ | ✗ |
| 제로 런타임 (단일 바이너리) | ✅ (Rust) | ✅ (.NET 내장) | ✗ | ✗ |
| Word + Excel + PowerPoint + PDF | ✅ | ✅ | ✅ | 별도 라이브러리 |
| 텍스트/오프셋 → 경로 매핑 | ✅ | ✅ | ✗ | ✗ |
| 라이브 HTML 미리보기 | ✅ | ✅ | ✗ | ✗ |
| MCP 서버 | ✅ | ✅ | ✗ | ✗ |
| 헤드리스 / CI / Docker | ✅ | ✅ | ✗ | ✅ |

### 업스트림 OfficeCLI (C#)와 비교

| 기능 | 업스트림 (C#) | 이 저장소 (Rust) |
|------|--------------|-----------------|
이 Rust 이식판은 C# 업스트림과 **API 호환** (동일한 명령어 이름, 경로 구문, `--prop` 규약)이며, **명령어 수준 패리티**에 도달했습니다. 남은 차이는 엣지 케이스 충실도와 생태계 도구에 있으며, 명령어 커버리지에는 없습니다.

| 기능 | 업스트림 (C#) | 이 저장소 (Rust) |
|------|--------------|-----------------|
| 템플릿 `merge` | ✅ | ✅ |
| `view screenshot` (PNG) | ✅ | ✅ (헤드리스 Chrome/Edge/Firefox) |
| `view pdf` (PDF 내보내기) | ✅ | ✅ (헤드리스 Chromium `--print-to-pdf`) |
| `view forms` (SDT 양식 필드) | ✅ | ✅ (docx SDT 파싱) |
| `swap`, `refresh`, `plugins` | ✅ | ✅ |
| `add-part` (차트/헤더/푸터) | ✅ | ✅ |
| `import` (CSV/TSV → xlsx) | ✅ | ✅ |
| `mark/unmark/marks/goto` (watch) | ✅ | ✅ (watch 서버 라우트) |
| `officecli install` | ✅ | ✅ (바이너리 + 스킬 + MCP) |
| 수식 엔진 (150+ 함수) | ✅ | ✅ (80+ 함수) |
| 피벗 테이블 (목록) | ✅ | ✅ (목록 + 소스 범위) |
| Morph 전환 (보고) | ✅ | ✅ (감지 + 후보 카운트) |
| 3D 모델 | ✅ | ✅ (HTML 미리보기) |
| Python SDK | ✅ | ✅ (Unix 도메인 소켓 IPC) |
| CLI 스모크 & 통합 테스트 | ✅ | ✅ (39 CLI + 32 유닛 테스트) |
| `cargo clippy -D warnings` 클린 | 해당 없음 | ✅ |
| AionUi GUI | ✅ | 해당 없음 |
| Wiki 및 성숙한 생태계 | ✅ | 초기 단계 |

전체 참조: [iOfficeAI/OfficeCLI Wiki](https://github.com/iOfficeAI/OfficeCLI/wiki)

## 명령어 참조

| 명령어 | 설명 |
|--------|------|
| `create` | 빈 `.docx` / `.xlsx` / `.pptx` / `.pdf` 생성 |
| `view` | 콘텐츠 보기 (text, annotated, outline, stats, issues, html, svg, screenshot, pdf, forms) |
| `get` | 요소와 하위 요소 가져오기 (`--depth N`, `--json`) |
| `query` | CSS 스타일 쿼리 |
| `set` | 요소 속성 수정 |
| `add` | 요소 추가 |
| `add-part` | 문서 파트 (차트/헤더/푸터) 생성 및 rel ID 반환 |
| `remove` | 요소 삭제 |
| `move` | 요소 이동 |
| `swap` | 두 요소 교환 (단락/슬라이드/셀) |
| `save` | 변경사항을 파일에 저장 |
| `validate` | 문서 구조 검증 |
| `extract-text` | 텍스트와 오프셋→경로 매핑 추출 (`--with-offsets`, `--json`) |
| `convert` | 레거시 형식 변환 (`.doc`/`.xls`/`.ppt`) (`--engine libreoffice|oxide`) |
| `batch` | 하나의 사이클에서 여러 작업 실행 |
| `dump` | 문서 구조를 재생 가능한 JSON으로 직렬화 |
| `raw` | 원시 XML 보기 |
| `raw-set` | XPath로 원시 XML 수정 (`setattr`, `remove`) |
| `import` | CSV/TSV 데이터를 Excel 시트에 가져오기 |
| `merge` | 템플릿 플레이스홀더 (`{{key}}`)와 JSON 데이터 병합 |
| `refresh` | 파생 필드 새로고침 (목차, 상호 참조) |
| `watch` | 라이브 미리보기 (자동 새로고침) |
| `unwatch` | watch 서버 중지 |
| `open` / `close` | 레지던트 모드 (Unix) |
| `plugins` | 플러그인 목록/검사/린트 (`list`, `info`, `lint`) |
| `install` | 바이너리, 스킬, MCP 구성 설치 (`--dry-run`, `--prefix`) |
| `info` | 도구 또는 문서 주제 정보 |
| `mcp` | MCP 서버 시작 (AI 도구 통합) |

전역 플래그: `--json`

## 사용 사례

**개발자** — CI/CD 자동화, Docker 헤드리스 처리, 경량 Rust 바이너리 임베딩

**AI 에이전트** — TextOffsetMap 정밀 편집, `watch` 시각 피드백, MCP 통합

**팀** — 감사 가능한 Rust 코드로 내부 자동화, 업스트림에서 점진적 마이그레이션

## 소스에서 빌드

[Rust](https://rustup.rs/) 1.75+ (CI는 1.90.0) 필요.

```bash
git clone https://github.com/RainLib/OfficeCli-rust.git
cd OfficeCli-rust
cargo build --release
```

```bash
make dist
make download VERSION=v0.1.2 PLATFORM=all
make smoke
```

## 프로젝트 구조

```
OfficeCli-rust/
├── Cargo.toml
├── install.sh / install.ps1
├── scripts/download.sh
├── SKILL.md
├── crates/ (officecli, handler-common, oxml, docx/xlsx/pptx/pdf-handler)
├── examples/
└── skills/
```

## 기여

[CONTRIBUTING.md](CONTRIBUTING.md) 참조. 이슈: [GitHub Issues](https://github.com/RainLib/OfficeCli-rust/issues)

업스트림 참조: [iOfficeAI/OfficeCLI](https://github.com/iOfficeAI/OfficeCLI)

## 라이선스

[Apache License 2.0](LICENSE)

---

[GitHub — RainLib/OfficeCli-rust](https://github.com/RainLib/OfficeCli-rust) | [업스트림 — iOfficeAI/OfficeCLI](https://github.com/iOfficeAI/OfficeCLI) | [Releases](https://github.com/RainLib/OfficeCli-rust/releases)

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
