#!/usr/bin/env python3
from __future__ import annotations

import json
import pathlib
import tomllib
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
BIN = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "target" / "release" / "promptgen"
VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]


def fail(message: str, process: subprocess.Popen[str] | None = None) -> None:
    if process is not None:
        process.terminate()
        try:
            _, stderr = process.communicate(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            _, stderr = process.communicate()
        if stderr:
            message += f"\nserver stderr:\n{stderr}"
    print(f"[FAIL] {message}", file=sys.stderr)
    raise SystemExit(1)


with socket.socket() as probe:
    probe.bind(("127.0.0.1", 0))
    port = probe.getsockname()[1]

with tempfile.TemporaryDirectory(prefix="promptgen-web-smoke-") as temporary:
    process = subprocess.Popen(
        [str(BIN), "serve", "--bind", f"127.0.0.1:{port}", "--output-dir", temporary],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    base = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 8
    health: dict[str, object] | None = None
    while time.monotonic() < deadline:
        if process.poll() is not None:
            fail(f"server exited with {process.returncode}", process)
        try:
            with urllib.request.urlopen(base + "/api/v3/health", timeout=1) as response:
                health = json.load(response)
                break
        except (urllib.error.URLError, TimeoutError, ConnectionError):
            time.sleep(0.05)
    if health is None:
        fail("server did not become ready", process)
    if health.get("version") != VERSION:
        fail(f"unexpected health version: {health}", process)

    for path, marker in [
        ("/", "이미지 프롬프트 작업실"),
        ("/app.css", ".inspector-panel"),
        ("/app.js", "dataset.questionId"),
        ("/api/v3/catalog", '"prompt_directives"'),
        ("/api/v3/lut-presets", '"warm_pastel_filmic"'),
    ]:
        with urllib.request.urlopen(base + path, timeout=2) as response:
            body = response.read().decode("utf-8")
        if marker not in body:
            fail(f"{path} is missing marker {marker!r}", process)

    with urllib.request.urlopen(base + "/api/v3/catalog/assets/C5", timeout=5) as response:
        catalog_asset = response.read()
        content_type = response.headers.get_content_type()
    if content_type != "image/png" or not catalog_asset.startswith(b"\x89PNG\r\n\x1a\n"):
        fail("catalog asset endpoint did not return a complete PNG", process)

    def post(path: str, payload: dict[str, object]) -> dict[str, object]:
        data = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        request = urllib.request.Request(
            base + path,
            data=data,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "X-PromptGen-Client": "web-v3",
            },
        )
        with urllib.request.urlopen(request, timeout=3) as response:
            return json.load(response)

    first = post(
        "/api/v3/interview",
        {
            "kind": "image",
            "brief": "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.",
            "answers": {},
        },
    )
    interview = first.get("interview")
    if not isinstance(interview, dict) or interview.get("status") != "needs_input":
        fail(f"guided interview did not request missing input: {first}", process)
    questions = interview.get("questions")
    if not isinstance(questions, list) or not any(isinstance(item, dict) and item.get("id") == "image.wardrobe" for item in questions):
        fail(f"C1 wardrobe question missing: {first}", process)

    poster_brief = (
        "출근길 직장인 대상 독립 커피 브랜드 콜드브루 출시 캠페인 포스터. "
        "무광 검정 커피 캔 1개가 히어로이고 좌하단 CTA 여백 18%, 비 오는 밤 서울 골목."
    )
    poster = post(
        "/api/v3/interview",
        {
            "kind": "image",
            "brief": poster_brief,
            "answers": {},
        },
    )
    poster_outcome = poster.get("interview")
    if not isinstance(poster_outcome, dict) or poster_outcome.get("status") != "needs_input":
        fail(f"poster interview did not request exact copy: {poster}", process)
    poster_answers = poster_outcome.get("normalized_answers")
    poster_questions = poster_outcome.get("questions")
    if (
        not isinstance(poster_answers, dict)
        or poster_answers.get("image.category") != "C5"
        or poster_answers.get("image.medium") != "graphic_design"
        or not isinstance(poster_questions, list)
        or [item.get("id") for item in poster_questions if isinstance(item, dict)] != ["image.text_mode"]
    ):
        fail(f"clean poster route is not isolated from fashion questions: {poster}", process)
    if "image.wardrobe" in poster_answers or "STY-17" in str(poster_answers):
        fail(f"clean poster route retained editorial state: {poster}", process)

    explicit_poster = post(
        "/api/v3/interview",
        {
            "kind": "image",
            "brief": poster_brief,
            "answers": {"image.category": "C5", "image.text_mode": "exact"},
        },
    )
    explicit_outcome = explicit_poster.get("interview")
    explicit_answers = (
        explicit_outcome.get("normalized_answers")
        if isinstance(explicit_outcome, dict)
        else None
    )
    explicit_questions = (
        explicit_outcome.get("questions")
        if isinstance(explicit_outcome, dict)
        else None
    )
    if (
        not isinstance(explicit_answers, dict)
        or explicit_answers.get("image.category") != "C5"
        or not isinstance(explicit_questions, list)
        or [item.get("id") for item in explicit_questions if isinstance(item, dict)] != ["image.text"]
        or any(
            isinstance(item, dict) and item.get("id") == "image.wardrobe"
            for item in explicit_questions
        )
    ):
        fail(f"explicit C5 route accepted an editorial-only style or question: {explicit_poster}", process)

    ready = post(
        "/api/v3/interview",
        {
            "kind": "image",
            "brief": "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.",
            "answers": {
                "image.wardrobe": "세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏",
            },
        },
    )
    outcome = ready.get("interview")
    if not isinstance(outcome, dict) or outcome.get("status") != "ready":
        fail(f"completed interview is not ready: {ready}", process)
    compilation = outcome.get("compilation")
    prompt = compilation.get("prompt") if isinstance(compilation, dict) else None
    if (
        not isinstance(prompt, str)
        or "리넨" not in prompt
        or "resort_beach" in prompt
        or "2:3" not in prompt
    ):
        fail("ready editorial prompt is missing the lossless image contract", process)

    process.terminate()
    try:
        process.wait(timeout=3)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=3)

print("[OK] release web smoke: UI/assets/catalog v5/LUT/C1→C5 isolation/lossless guided flow")
