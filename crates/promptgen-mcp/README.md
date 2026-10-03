# promptgen-mcp

image compiler/interview/catalog/LUT의 네 read-only 도구를 stdio와 Streamable HTTP로 노출한다.
provider 실행·reference 파일 전송·screen compiler 도구는 제공하지 않는다.

caller는 `promptgen_interview`에 brief와 누적 answers를 다시 보내고 `ready`의
`structuredContent.compilation.prompt`를 사용한다. 서버가 interview 상태를 영속 저장한다고 가정하지 않는다.
HTTP는 protocol `2026-07-28`, metadata·header 규칙을 따르며 stdio에는 HTTP header가 없다.

빌드·연결·도구·오류는 [MCP 운영](../../docs/MCP.md), 실제 Origin·Host·인증 경계와 검증 범위는 [ANALYSIS](ANALYSIS.md)다.
