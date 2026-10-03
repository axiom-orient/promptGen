# Rust workspace·분석 단위

하나의 promptGen 제품에 다섯 crate가 있다. 아래 분석은 폴더 수가 아니라 공개 surface·caller·효과 소유권으로 구분했다.

| crate | 실제 책임 | 분석 |
|---|---|---|
| `promptgen-core` | image/interview/screen 컴파일·catalog/schema·PNG/SHA; 공개 파일 읽기 API도 존재 | [ANALYSIS](promptgen-core/ANALYSIS.md) |
| `promptgen-codex` | Codex process/event/후보 검수·PNG 게시 | [ANALYSIS](promptgen-codex/ANALYSIS.md) |
| `promptgen-web` | embedded Studio·HTTP·browser 상태·Codex 생성 | [ANALYSIS](promptgen-web/ANALYSIS.md) |
| `promptgen-mcp` | read-only 도구·stdio/HTTP protocol adapter | [ANALYSIS](promptgen-mcp/ANALYSIS.md) |
| `promptgen-cli` | binary composition root·옵션·stdio/file·서비스 시작 | [ANALYSIS](promptgen-cli/ANALYSIS.md) |

의존 방향은 `CLI → Web/MCP/Codex/core`, `Web → Codex/core`, 나머지 adapter는 core다.
core는 Cargo 외부 dependency가 없지만 crate 전체가 I/O-free인 것은 아니다. MCP의 rmcp/axum/tokio,
CLI의 tokio를 구분한다. optional execution mode는 Cargo-feature 분리와 다르다.
실제 버전은 [workspace manifest](../Cargo.toml), [lockfile](../Cargo.lock), [toolchain](../rust-toolchain.toml)이 정한다.
확정 경계는 [ARCHITECTURE](../docs/ARCHITECTURE.md), 통합 현황은 [Current](../docs/IMPLEMENTATION_STATUS.md)다.
