#!/usr/bin/env python3
"""Generate and launch Ariadne's deterministic synthetic trace bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from typing import Any


TRACE_ID = "trace-ariadne-synthetic-v1"
ROLLOUT_ID = "019d1111-0000-7000-8000-000000000001"
ROOT_THREAD_ID = ROLLOUT_ID
CHILD_THREAD_ID = "019d1111-0000-7000-8000-000000000002"
STARTED_AT_UNIX_MS = 1_753_632_000_000
RESULT_TEXT = (
    "The synthetic bundle is deterministic and contains no captured trace data."
)
RESULT_NOTIFICATION = (
    '<subagent_notification>{"agent_path":"/root/synthetic_reviewer",'
    f'"status":{{"completed":"{RESULT_TEXT}"}}}}'
    "</subagent_notification>"
)


def encoded_json(value: Any, *, pretty: bool = False) -> bytes:
    options = {
        "ensure_ascii": False,
        "sort_keys": True,
    }
    if pretty:
        options["indent"] = 2
    else:
        options["separators"] = (",", ":")
    return json.dumps(value, **options).encode("utf-8")


def message(role: str, text: str, text_type: str) -> dict[str, Any]:
    return {
        "content": [{"text": text, "type": text_type}],
        "role": role,
        "type": "message",
    }


def inter_agent_message(
    author: str,
    recipient: str,
    content: str,
    *,
    trigger_turn: bool,
) -> str:
    return json.dumps(
        {
            "author": author,
            "content": content,
            "other_recipients": [],
            "recipient": recipient,
            "trigger_turn": trigger_turn,
        },
        separators=(",", ":"),
        sort_keys=True,
    )


class BundleBuilder:
    def __init__(self) -> None:
        self._files: dict[str, bytes] = {}
        self._events: list[dict[str, Any]] = []

    def payload(self, kind: str, value: Any) -> dict[str, Any]:
        ordinal = 1 + sum(path.startswith("payloads/") for path in self._files)
        path = f"payloads/{ordinal}.json"
        self._files[path] = encoded_json(value, pretty=True)
        return {
            "kind": {"type": kind},
            "path": path,
            "raw_payload_id": f"raw_payload:{ordinal}",
        }

    def event(
        self,
        event_type: str,
        *,
        context_thread_id: str | None = None,
        context_turn_id: str | None = None,
        **fields: Any,
    ) -> None:
        seq = len(self._events) + 1
        self._events.append(
            {
                "codex_turn_id": context_turn_id,
                "payload": {"type": event_type, **fields},
                "rollout_id": ROLLOUT_ID,
                "schema_version": 1,
                "seq": seq,
                "thread_id": context_thread_id,
                "wall_time_unix_ms": STARTED_AT_UNIX_MS + seq * 100,
            }
        )

    def files(self) -> dict[str, bytes]:
        manifest = {
            "payloads_dir": "payloads",
            "raw_event_log": "trace.jsonl",
            "rollout_id": ROLLOUT_ID,
            "root_thread_id": ROOT_THREAD_ID,
            "schema_version": 1,
            "started_at_unix_ms": STARTED_AT_UNIX_MS,
            "trace_id": TRACE_ID,
        }
        return {
            **self._files,
            "manifest.json": encoded_json(manifest, pretty=True),
            "trace.jsonl": b"".join(
                encoded_json(event) + b"\n" for event in self._events
            ),
        }


def bundle_files() -> dict[str, bytes]:
    bundle = BundleBuilder()
    root_turn = "turn-synthetic-root"
    child_turn = "turn-synthetic-child"
    followup_turn = "turn-synthetic-followup"

    root_metadata = bundle.payload(
        "session_metadata",
        {
            "model": "synthetic-model",
            "nickname": "Ariadne root",
            "session_source": "synthetic_demo",
        },
    )
    root_request = bundle.payload(
        "inference_request",
        {
            "input": [
                message(
                    "user",
                    "Demonstrate a deterministic delegated trace.",
                    "input_text",
                )
            ]
        },
    )
    root_response = bundle.payload(
        "inference_response",
        {
            "output_items": [
                message(
                    "assistant",
                    "I will delegate one synthetic task.",
                    "output_text",
                )
            ],
            "response_id": "resp-synthetic-root",
            "token_usage": None,
        },
    )
    spawn_invocation = bundle.payload(
        "tool_invocation",
        {
            "payload": {
                "arguments": (
                    '{"message":"Return one deterministic observation.",'
                    '"task_name":"synthetic_reviewer"}'
                ),
                "type": "function",
            },
            "tool_name": "spawn_agent",
        },
    )
    spawn_start = bundle.payload(
        "tool_runtime_event",
        {
            "call_id": "call-synthetic-spawn",
            "prompt": "Return one deterministic observation.",
            "sender_thread_id": ROOT_THREAD_ID,
        },
    )
    spawn_end = bundle.payload(
        "tool_runtime_event",
        {
            "call_id": "call-synthetic-spawn",
            "model": "synthetic-model",
            "new_thread_id": CHILD_THREAD_ID,
            "prompt": "Return one deterministic observation.",
            "reasoning_effort": "medium",
            "sender_thread_id": ROOT_THREAD_ID,
            "status": "running",
        },
    )
    spawn_result = bundle.payload(
        "tool_result",
        {
            "task_name": "/root/synthetic_reviewer",
            "thread_id": CHILD_THREAD_ID,
        },
    )
    child_metadata = bundle.payload(
        "session_metadata",
        {
            "agent_role": "reviewer",
            "model": "synthetic-model",
            "nickname": "Synthetic reviewer",
            "session_source": {
                "subagent": {
                    "thread_spawn": {
                        "agent_nickname": "Synthetic reviewer",
                        "agent_path": "/root/synthetic_reviewer",
                        "agent_role": "reviewer",
                        "parent_thread_id": ROOT_THREAD_ID,
                        "task_name": "synthetic_reviewer",
                    }
                }
            },
        },
    )
    child_request = bundle.payload(
        "inference_request",
        {
            "input": [
                message(
                    "assistant",
                    inter_agent_message(
                        "/root",
                        "/root/synthetic_reviewer",
                        "Return one deterministic observation.",
                        trigger_turn=True,
                    ),
                    "input_text",
                )
            ]
        },
    )
    child_response = bundle.payload(
        "inference_response",
        {
            "output_items": [
                message("assistant", RESULT_TEXT, "output_text")
            ],
            "response_id": "resp-synthetic-child",
            "token_usage": None,
        },
    )
    carried_result = bundle.payload(
        "agent_result",
        {
            "child_thread_id": CHILD_THREAD_ID,
            "message": RESULT_NOTIFICATION,
            "recipient_thread_id": ROOT_THREAD_ID,
            "status": {"completed": RESULT_TEXT},
        },
    )
    followup_request = bundle.payload(
        "inference_request",
        {
            "input": [
                message(
                    "assistant",
                    inter_agent_message(
                        "/root/synthetic_reviewer",
                        "/root",
                        RESULT_NOTIFICATION,
                        trigger_turn=False,
                    ),
                    "input_text",
                )
            ]
        },
    )
    followup_response = bundle.payload(
        "inference_response",
        {
            "output_items": [
                message(
                    "assistant",
                    "The synthetic result was received.",
                    "output_text",
                )
            ],
            "response_id": "resp-synthetic-followup",
            "token_usage": None,
        },
    )

    bundle.event(
        "rollout_started",
        trace_id=TRACE_ID,
        root_thread_id=ROOT_THREAD_ID,
    )
    bundle.event(
        "thread_started",
        thread_id=ROOT_THREAD_ID,
        agent_path="/root",
        metadata_payload=root_metadata,
    )
    bundle.event(
        "codex_turn_started",
        thread_id=ROOT_THREAD_ID,
        codex_turn_id=root_turn,
    )
    bundle.event(
        "inference_started",
        context_thread_id=ROOT_THREAD_ID,
        context_turn_id=root_turn,
        thread_id=ROOT_THREAD_ID,
        codex_turn_id=root_turn,
        inference_call_id="inference-synthetic-root",
        model="synthetic-model",
        provider_name="synthetic-provider",
        request_payload=root_request,
    )
    bundle.event(
        "inference_completed",
        context_thread_id=ROOT_THREAD_ID,
        context_turn_id=root_turn,
        inference_call_id="inference-synthetic-root",
        response_id="resp-synthetic-root",
        response_payload=root_response,
        upstream_request_id="req-synthetic-root",
    )
    tool_context = {
        "context_thread_id": ROOT_THREAD_ID,
        "context_turn_id": root_turn,
        "tool_call_id": "call-synthetic-spawn",
    }
    bundle.event(
        "tool_call_started",
        **tool_context,
        code_mode_runtime_tool_id=None,
        invocation_payload=spawn_invocation,
        kind={"type": "spawn_agent"},
        model_visible_call_id="call-synthetic-spawn",
        requester={"type": "model"},
        summary={
            "input_preview": "Return one deterministic observation.",
            "label": "spawn_agent",
            "output_preview": None,
            "type": "generic",
        },
    )
    bundle.event(
        "tool_call_runtime_started",
        **tool_context,
        runtime_payload=spawn_start,
    )
    bundle.event(
        "tool_call_runtime_ended",
        **tool_context,
        runtime_payload=spawn_end,
        status="completed",
    )
    bundle.event(
        "tool_call_ended",
        **tool_context,
        result_payload=spawn_result,
        status="completed",
    )
    bundle.event(
        "thread_started",
        thread_id=CHILD_THREAD_ID,
        agent_path="/root/synthetic_reviewer",
        metadata_payload=child_metadata,
    )
    bundle.event(
        "codex_turn_started",
        thread_id=CHILD_THREAD_ID,
        codex_turn_id=child_turn,
    )
    bundle.event(
        "inference_started",
        context_thread_id=CHILD_THREAD_ID,
        context_turn_id=child_turn,
        thread_id=CHILD_THREAD_ID,
        codex_turn_id=child_turn,
        inference_call_id="inference-synthetic-child",
        model="synthetic-model",
        provider_name="synthetic-provider",
        request_payload=child_request,
    )
    bundle.event(
        "inference_completed",
        context_thread_id=CHILD_THREAD_ID,
        context_turn_id=child_turn,
        inference_call_id="inference-synthetic-child",
        response_id="resp-synthetic-child",
        response_payload=child_response,
        upstream_request_id="req-synthetic-child",
    )
    bundle.event(
        "codex_turn_ended",
        context_thread_id=CHILD_THREAD_ID,
        context_turn_id=child_turn,
        codex_turn_id=child_turn,
        status="completed",
    )
    bundle.event(
        "thread_ended",
        thread_id=CHILD_THREAD_ID,
        status="completed",
    )
    bundle.event(
        "agent_result_observed",
        edge_id="edge-synthetic-result",
        child_thread_id=CHILD_THREAD_ID,
        child_codex_turn_id=child_turn,
        parent_thread_id=ROOT_THREAD_ID,
        message=RESULT_NOTIFICATION,
        carried_payload=carried_result,
    )
    bundle.event(
        "codex_turn_ended",
        context_thread_id=ROOT_THREAD_ID,
        context_turn_id=root_turn,
        codex_turn_id=root_turn,
        status="completed",
    )
    bundle.event(
        "codex_turn_started",
        thread_id=ROOT_THREAD_ID,
        codex_turn_id=followup_turn,
    )
    bundle.event(
        "inference_started",
        context_thread_id=ROOT_THREAD_ID,
        context_turn_id=followup_turn,
        thread_id=ROOT_THREAD_ID,
        codex_turn_id=followup_turn,
        inference_call_id="inference-synthetic-followup",
        model="synthetic-model",
        provider_name="synthetic-provider",
        request_payload=followup_request,
    )
    bundle.event(
        "inference_completed",
        context_thread_id=ROOT_THREAD_ID,
        context_turn_id=followup_turn,
        inference_call_id="inference-synthetic-followup",
        response_id="resp-synthetic-followup",
        response_payload=followup_response,
        upstream_request_id="req-synthetic-followup",
    )
    bundle.event(
        "codex_turn_ended",
        context_thread_id=ROOT_THREAD_ID,
        context_turn_id=followup_turn,
        codex_turn_id=followup_turn,
        status="completed",
    )
    bundle.event(
        "thread_ended",
        thread_id=ROOT_THREAD_ID,
        status="completed",
    )
    bundle.event("rollout_ended", status="completed")
    return bundle.files()


def tree_fingerprint(files: dict[str, bytes]) -> str:
    digest = hashlib.sha256()
    for relative_path, contents in sorted(files.items()):
        digest.update(relative_path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(contents)
        digest.update(b"\0")
    return digest.hexdigest()


def generate_bundle(output: Path) -> None:
    if output.exists() and any(output.iterdir()):
        raise SystemExit(f"refusing to overwrite non-empty directory: {output}")
    output.mkdir(parents=True, exist_ok=True)
    files = bundle_files()
    for relative_path, contents in files.items():
        destination = output.joinpath(*relative_path.split("/"))
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(contents)
    print(f"generated {output}")
    print(f"sha256 {tree_fingerprint(files)}")


def verify_bundle(bundle: Path) -> None:
    if not bundle.is_dir():
        raise SystemExit(f"bundle directory does not exist: {bundle}")
    expected = bundle_files()
    actual: dict[str, bytes] = {}
    for path in bundle.rglob("*"):
        if path.is_symlink():
            raise SystemExit(f"bundle contains a symlink: {path}")
        if path.is_file():
            actual[path.relative_to(bundle).as_posix()] = path.read_bytes()
    if actual != expected:
        missing = sorted(set(expected) - set(actual))
        extra = sorted(set(actual) - set(expected))
        changed = sorted(
            path for path in set(actual) & set(expected) if actual[path] != expected[path]
        )
        details = [
            *(f"missing: {path}" for path in missing),
            *(f"extra: {path}" for path in extra),
            *(f"changed: {path}" for path in changed),
        ]
        raise SystemExit("bundle differs from deterministic output\n" + "\n".join(details))
    print(f"verified {bundle}")
    print(f"sha256 {tree_fingerprint(actual)}")


def launch_demo(binary: str) -> None:
    with tempfile.TemporaryDirectory(prefix="ariadne-trace-demo-") as temp:
        bundle = Path(temp) / "synthetic-bundle"
        generate_bundle(bundle)
        subprocess.run([binary, "trace", "--bundle", os.fspath(bundle)], check=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    generate = commands.add_parser("generate", help="write a deterministic bundle")
    generate.add_argument("output", type=Path)
    verify = commands.add_parser("verify", help="verify deterministic bundle bytes")
    verify.add_argument("bundle", type=Path)
    run = commands.add_parser("run", help="generate and open the normal trace browser")
    run.add_argument("--binary", default="codex-ariadne")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if args.command == "generate":
        generate_bundle(args.output)
    elif args.command == "verify":
        verify_bundle(args.bundle)
    elif args.command == "run":
        launch_demo(args.binary)


if __name__ == "__main__":
    main()
