# promptgen-mcp 분석

## 책임 경계

promptgen-mcp는 image compiler를 노출하는 read-only MCP server다. tool 실행은 prompt compilation, interview, catalog, LUT 조회로 한정하며 Luna, Codex imagegen, OpenAI API를 호출하지 않는다. core가 domain 의미를 소유하고 rmcp가 MCP protocol/HTTP validation을 담당한다.

## 단일 tool contract

| 도구 | 입력 | 효과 |
|---|---|---|
| `promptgen_compile_image` | strict typed image request | compile/diagnostics 반환 |
| `promptgen_interview` | image brief/answers | 질문·ready request·compile 결과 반환 |
| `promptgen_catalog` | 빈 object | 전체 catalog 반환 |
| `promptgen_lut_presets` | 빈 object | LUT 목록 반환 |

알 수 없는 tool argument는 strict decoder가 거부한다. catalog는 family/pattern 조건 없이 여섯 category 전체를 반환한다. transport는 stdio 또는 stateless Streamable HTTP다.

## HTTP admission

non-loopback bind에는 bearer token과 allowed Host가 필요하다. HTTP Origin allowlist는 실제 bind port와 기존 Host 설정으로 계산하고 rmcp validator 한 곳에서 검사한다. same-origin은 허용, disallowed 또는 path가 추가된 origin은 403, 파싱 불가능한 origin은 400이다. Host·Origin·bearer는 각각 별도 검사다.

## 검증 상태

`./scripts/verify.sh`의 MCP integration은 정상 요청, protocol metadata, bearer/Host, Origin 허용·거부를 검증한다. 실제 현재 검증 결과는 [구현 현황](../../docs/IMPLEMENTATION_STATUS.md)에 기록한다. server 자체는 plain HTTP이며 외부 TLS proxy origin 설정은 계약에 없다.

