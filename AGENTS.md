# Project Agent Contract

This file is scoped to this project directory. Treat a closer project instruction file as a separate contract; do not use sibling projects as evidence.

## Project profile

- Project kind: Rust typed prompt compiler with optional Codex image-generation adapters.
- Platform: Host-native Rust plus optional web/MCP adapters.
- Primary gate: ./scripts/verify.sh and ./scripts/verify-quality.sh
- Clean boundary: ./scripts/clean.sh; preserve catalog, schemas, fixtures, and source manifests.

## Operating rules

- Read the nearest manifest, lockfile, entrypoint, tests, and scripts before editing.
- Actual source, configuration, tests, and runtime output outrank README claims. Mark unavailable checks as `[UNKNOWN]`, `[UNVERIFIED]`, or `[NOT_RUN_ENVIRONMENT]`.
- This project is independently scoped. Do not scan or modify sibling projects, parent projects, vendor trees, or third-party copies without an explicit request.
- Preserve source, user data, manifests, lockfiles, fixtures, legal files, and evidence inputs. Clean only reproducible output.
- Keep one canonical contract. Do not add compatibility shims, migration layers, legacy wrappers, silent fallbacks, fake success, or duplicate entrypoints.

## Browser work

- Use the installed Aside CLI command `aside` for navigation, inspection, screenshots, downloads, and browser smoke checks.
- Use `aside repl "..." ` for deterministic browser inspection and `aside --session <id> "..." ` to continue a session.
- Do not add new Playwright, Puppeteer, browser-specific MCP, or custom browser harness code. Existing project code is not migrated by this instruction-only change.
- Keep downloads, transcripts, screenshots, and temporary browser output outside the source tree unless an explicit output path owns them.

## Verification and automation policy

- Run the project's real local gate after a change; never claim an unexecuted build, test, or runtime check.
- GitHub Actions, `.github/workflows`, CI/CD workflows, release automation, and remote verification must not be created, modified, restored, enabled, or proposed.
- If a check fails, reproduce, observe, verify the root cause, make the smallest fix, and rerun the same check.
- Report changed files, commands run, results, blockers, and remaining risks.



## Subscription authentication contract

- Use only the user's ChatGPT subscription through the authenticated Codex CLI.
- Never request, read, configure, or use API keys. Do not add direct API-key adapters or an API-key fallback.
- All provider subprocesses must force `forced_login_method="chatgpt"` and remove `OPENAI_API_KEY` and `CODEX_API_KEY` from their environment.
- Luna (`gpt-5.6-luna`) performs prompt review and provider orchestration. Image execution uses the canonical `codex-subscription` backend. Detail is prompt intent and dimensions are a verified local delivery target; do not expose native model/quality controls that the subscription tool does not support.
- Actual image editing uses one admitted base PNG attached to Codex; preserve the original file and bind its snapshot SHA to the receipt.
