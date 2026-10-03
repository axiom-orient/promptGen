# promptgen-web

embedded Studio와 image HTTP API를 제공한다. core가 요청 의미를, Codex adapter가 생성 실행을 소유한다.
CLI `promptgen serve`가 일반 시작점이며 `WebConfig`, `WebError`, `serve`, `serve_listener` 등은 공개 library surface다.

Studio asset·interview·generate·catalog/LUT·artifact route, HTTP 검증·process-local 단일 generation 제한을 담당한다.
별도 DB나 multi-user service, 사진 업로드 transport, screen 입력 UI는 소유하지 않는다.

기본 loopback 정책과 외부에서 받은 listener의 실제 주소 검증은 다른 문제다. 현재 public listener gap,
browser stale completion, 저장·shutdown·asset 경계는 [ANALYSIS](ANALYSIS.md),
사용자 계약은 [Studio UX](../../docs/IMAGE_STUDIO_UX_SPEC.md), 파일 운영은 [RUNTIME_STORAGE](../../docs/RUNTIME_STORAGE.md)에서 확인한다.
