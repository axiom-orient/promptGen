# agent-contract 0.1.0

UpAgent, Vergerail, PromptGen이 직접 사용하는 공통 Rust 계약이다. 독립 PromptGen
저장소의 source에 포함하며 UpAgent runtime을 의존하지 않는다. DTO 복사와 문자열
재변환을 제거하고 producer와 consumer가 같은 타입을 사용한다.

| 경계 | 공통 원본 |
|---|---|
| 모델·tool·usage | `model` |
| provider 요청·응답·실패·이미지 옵션·artifact·관찰 | `provider` |
| artifact·tool call ID | `identity` |
| 컴파일 결과·진단·detail·background | `compilation` |
| semantic edit·fidelity·refinement 영수증 | `receipt` |
| frame·tool·timeout 한도 | `limits` |
| 공통 schema 카탈로그 | `schema` |

기본 feature에는 외부 의존성이 없다. `serde`는 serialization, `schema`는 같은
타입에서 JSON Schema를 생성한다. 파일·HTTP·process·인증·상태 전이·승인 코드는 없다.
순수 compiler는 기본 feature만 사용한다.

기존 wire `vergerail.upagent/2`와 compiler JSON 형식을 보존한다. 카탈로그는
`agent.shared-contract/1`이다. 형식 변경 시 producer, consumer, schema와 회귀 증거를
함께 변경한다. 구조 schema는 필드·enum·수치 한도를 나타낸다. `validate_wire()`는
byte 길이·deadline·배열 한도를 확인한다. operation별 capability, 파일 권한, PNG
검증과 승인·후보 결정은 실제 실행 owner가 확인한다.

PromptGen `detail`은 프롬프트 의도이고 provider `quality`는 실행 제어다. 서로
변환하지 않는다. compiler background는 `auto/opaque`, provider background는
`auto/transparent/opaque`다. Codex 외부 JSON-RPC와 IFSC screen 계약, UpAgent의
task/state/approval 계약도 각각의 owner에 남는다.

현재 배포 단위는 GitHub source다. registry 게시를 전제로 하지 않는다. 이 저장소의
고정 commit을 checkout하여 path로 연결하거나, GitHub 게시 후 `git` dependency의
`rev`를 정확한 commit으로 고정해 이 crate만 사용할 수 있다. 이 통합 저장소는
`components.lock.json`의 commit/tree를 확인한 sibling source layout에서 빌드한다.

아래 카탈로그 조회는 효과나 로그인을 실행하지 않는다.

```text
promptgen schema shared-contract
agent schema shared-contract
vergerail-upagent-provider --schema
```

출처와 라이선스는 [NOTICE](NOTICE.md), [Apache](LICENSE-APACHE), [BSD](LICENSE-BSD)에 있다.
