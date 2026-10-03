# 실행 데이터와 삭제 운영

저장소 밖 CLI output, `CODEX_HOME`, browser storage는 `var/`와 별도 owner다. 생성 PNG는 확률적·외부 효과이므로 build cache로 취급하지 않는다.

| 데이터 | 위치·owner | 수명·주의 |
|---|---|---|
| 예제·schema·catalog | repository | maintained source/generated contract. cleanup 기본 대상 아님 |
| Studio 진행 입력 | browser memory | 새로고침으로 소멸 가능 |
| Studio 최근 ready 기록 | `localStorage` `promptgen.recent.v5` | 최대 6개; durable run ledger 아님 |
| Studio PNG | 기본 `var/output/` 또는 `--output-dir` | 사용자 생성 결과. 기본 cleanup에서 보존. Studio가 표시할 때 receipt SHA query와 current bytes를 재대조 |
| Studio receipt | HTTP response/browser state | PNG 옆 durable receipt 파일을 자동 생성하지 않음 |
| CLI result/PNG | caller 지정 path/stdout | caller가 보존·백업 소유 |
| Codex 후보·인증 | `CODEX_HOME` | promptGen published output과 별도 외부 경계 |
| build output | `target/`, `dist/` | 재빌드 가능한 cleanup 대상 |

## 정리

조회만 할 때:

```sh
bash scripts/cleanup_inventory.sh
bash scripts/clean.sh --dry-run
```

기본 실제 정리:

```sh
bash scripts/clean.sh
```

기본 동작은 `target`, `dist`, retired root snapshot 파일, Python/editor/test noise와 복구 manifest가 있는 reinstallable dependency만 제거하고 **`var/output`을 보존**한다.

생성 결과까지 삭제할 의도가 있을 때만:

```sh
bash scripts/clean.sh --generated-output
```

이 opt-in은 백업을 수행하지 않는다. `--output-dir`로 저장소 밖에 둔 결과와 CLI 임의 output은 스크립트 범위 밖이다. `*.tmp`/`*.bak` 등 일반 noise도 실제 의미가 없는지 caller가 판단한 뒤 실행해야 한다.
