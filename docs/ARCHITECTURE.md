# promptGen 아키텍처 규범

## 경계·책임·의존 방향

core는 image/screen 입력 의미·검증·진단·deterministic render의 권위다. CLI/Studio/MCP는 caller/transport이며 domain 규칙을 독립적으로 재정의하지 않는다. Codex adapter는 구독을 통한 외부 실행·파일 admission·산출물 게시를 책임진다. CLI는 제품 binary의 composition root이며 외부 Rust caller는 자신이 사용하는 공개 library를 구성한다.

```text
CLI / Studio / MCP / Rust host → image/interview core → compilation / diagnostics
CLI screen / Rust host        → screen core          → handoff
CLI / Studio (명시 선택)       → Luna review           → append-only refinement
CLI / Studio                  → Codex adapter         → candidate / PNG / receipt
```

컴파일 계산에는 provider·HTTP·저장 의존성을 넣지 않는다. PNG/해시의 byte 계산과 path를 읽는 편의 I/O를 구별하고 public API 이동 여부는 소비자 계약 검토 후 결정한다. 단순히 “core”라는 폴더명으로 purity를 증명하지 않는다. 정확한 현재 패키지 배치/예외는 [Current](IMPLEMENTATION_STATUS.md)와 상세 ANALYSIS에 둔다.

screen은 C6 `app_web_ui`와 별도 도메인 계약이다. 생성 이미지의 구조 표현과 화면 design 좌표/행/state handoff를 같은 schema로 합치지 않는다. 임의 외부 host를 제품의 내부 실행 시스템에 포함하지 않는다.

### 권위 모델

| truth / decision | authoritative owner | 경계 |
|---|---|---|
| Meaning authority | `promptgen-core` | image/screen 의미·validation·deterministic render |
| Process composition | `promptgen-cli` 또는 public library를 조립하는 external host | 기능 선택·configuration·result destination. Studio server는 선택된 Web surface 내부 handler를 조립하지만 domain 의미를 소유하지 않음 |
| Provider effect | 각 provider adapter | auth·process/network·candidate/reference admission·publication·provider-specific failure |
| Browser intent state | Studio JS | 현재 brief/answer/mode/request identity와 stale completion 적용 여부 |
| Protocol ingress | MCP/Web transport | protocol/header/body/auth admission. domain 의미는 core로 위임 |
| Artifact truth | 실제 artifact bytes + SHA-256 | path 이름이나 성공 문구만으로 결과 identity를 주장하지 않음 |
| Verification truth | `VERIFICATION.json` + current source snapshot | 실행한 gate만 기록하며 source hash가 다르면 현재 성공 증거가 아님 |

권위 흐름은 `Intent → Typed Contract → Compile → Execute → Observed Artifact → Evidence`다. 같은 domain 의미는 core에 통합하지만 provider별 effect/auth/failure semantics는 공용 manager/framework로 억지로 합치지 않는다. 각 단계는 이전 단계의 identity를 검증 가능한 형태로 이어받고, 관찰하지 않은 effect나 artifact를 성공으로 추정하지 않는다.

## 실행·상태 모델

컴파일은 Input → Decode/Validation → Decision/Render → Outcome이다. image interview는 한 호출 안의 결정 과정이며 user 답의 영속 누적은 caller가 소유한다. catalog cache/normalized answers/검증된 candidate 같은 파생 상태는 원본 의도나 사용자 설정과 혼동하지 않는다.

| 값의 역할 | 예 | 소유 원칙 |
|---|---|---|
| Constant | schema/model 식별자, 지원 enum, codec 한도 | 해당 domain/adapter에 하나의 명시 원본 |
| Policy | 허용 크기·attempt·deadline·수용 기준 | 의미와 적용 phase를 지정; 실행 mechanism과 구분 |
| Configuration | bind、output path、binary、CODEX_HOME | composition root/caller가 제공 |
| Settings | browser theme·선택한 사용 옵션 | 사용자 설정 owner가 보존 |
| Feature selection | prompt-only / luna-refine / codex-imagegen | `luna-refine`은 검토만, 이미지 mode는 Luna 통과 뒤 명시 실행; 숨은 fallback 아님 |
| State | answers/outcome/request identity/child/candidate/commit | 실제 lifecycle owner에게 귀속 |

browser의 explicit answers, normalized 답, outstanding request, artifact는 서로 다른 수명이다. 결과를 현재 의도에 적용할 때 identity를 확인해야 한다. HTTP connection admission과 generation 동시성, shutdown과 외부 effect 종료를 별도로 다룬다.

## Persistence·transaction·권한

입력 JSON/schema/catalog의 정본, caller가 전달한 reference bytes, provider가 만든 artifact, receipt는 각각의 authority를 갖는다. reference 역할과 실제 전달 bytes를 결합하고 prompt로 파일 경로나 credential을 추측하지 않는다. 외부 host의 task/approval/budget/CAS는 그 host 경계이며 promptGen hash를 승인 토큰으로 취급하지 않는다.

파일 publication은 목적지 보호·temp·검증·commit·후처리를 구분한다. 여러 파일과 remote dispatch를 하나의 원자 transaction처럼 설명하지 않는다. CLI 결과와 provider PNG, browser 표시 성공과 서버 file 존재, 검수 candidate와 final bytes를 구분하여 관찰해야 한다. storage 사용 조건은 [RUNTIME_STORAGE](RUNTIME_STORAGE.md)가 소유한다.

같은 data를 입력과 외부 결과에서 다시 검증하는 것은 trust boundary별 admission이다. 이를 DRY 명목으로 지우지 않는다. 의미 owner의 모순이나 동일 output 역할의 alias 해석 차이는 통합해야 할 concern이다.

## 동시성·취소·복구

순수 컴파일의 재호출과 원격 생성 재시도를 분리한다. candidate 품질 실패에 대한 bounded repair는 timeout 뒤 원격 미실행을 가정하는 blind retry가 아니다. timeout은 적용 phase와 전체 lifetime을 명확히 하며 직계 process 중단, descendants, remote cancellation은 다른 보장이다.

실패·partial commit·unknown remote outcome을 숨기지 않는다. durable evidence/재시도 식별자/복구 정책의 새 형식은 별도 계약 결정으로 다루며 문서만으로 지원한다고 선언하지 않는다. 이미 게시한 artifact와 전달하지 못한 receipt가 있을 수 있는 경계를 고려해 caller가 행동하도록 해야 한다.

## 코어·선택 기능·확장 지점

필수는 재현 가능한 image/screen 계약과 진단이다. Studio/MCP/provider는 명시 경계 위의 선택 surface다. 새 capability는 기존 request/meaning owner를 소비하고 실제 호출·출력·실패·취소·증거 경계를 연결할 때만 완성으로 간주한다. 공용 provider framework·plugin manager·범용 state machine·새 DB를 미래 가능성만으로 추가하지 않는다.

지속적 결정은 [ADR-001 taxonomy](adr/001-outcome-taxonomy.md), [ADR-002 screen](adr/002-screen-contract.md)에 둔다. 구현 현황이나 test 성적을 아키텍처 규범에 섞지 않는다.
