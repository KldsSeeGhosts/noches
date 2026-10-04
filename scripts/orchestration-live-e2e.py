#!/usr/bin/env python3
"""Opt-in billable real headless Codex↔Claude E2E; stdlib only, scratch data.

Never copies login material, rewrites harness configuration, or opens real
Noches data. Installed authenticated CLI configurations are used read-only.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import signal
import socket
import struct
import subprocess
import tempfile
import time


class Rpc:
    def __init__(self, port):
        self.sock = socket.create_connection(("127.0.0.1", port), timeout=30)
        key = base64.b64encode(os.urandom(16)).decode()
        self.sock.sendall(
            f"GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\n"
            f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
            "Sec-WebSocket-Version: 13\r\n\r\n".encode()
        )
        header = b""
        while b"\r\n\r\n" not in header:
            chunk = self.sock.recv(1)
            if not chunk:
                self.sock.close()
                raise RuntimeError("engine IPC closed during handshake")
            header += chunk
            if len(header) > 65536:
                self.sock.close()
                raise RuntimeError("engine IPC handshake exceeded 64 KiB")
        assert b" 101 " in header, header.decode()
        self.sequence = 0

    def send(self, body, opcode=1):
        data = body.encode() if isinstance(body, str) else body
        mask = os.urandom(4)
        count = len(data)
        header = bytes([0x80 | opcode])
        if count < 126:
            header += bytes([0x80 | count])
        elif count < 65536:
            header += bytes([0x80 | 126]) + struct.pack("!H", count)
        else:
            header += bytes([0x80 | 127]) + struct.pack("!Q", count)
        self.sock.sendall(header + mask + bytes(byte ^ mask[i % 4] for i, byte in enumerate(data)))

    def exact(self, count):
        result = b""
        while len(result) < count:
            chunk = self.sock.recv(count - len(result))
            if not chunk:
                raise RuntimeError("engine IPC closed")
            result += chunk
        return result

    def receive(self):
        while True:
            first, second = self.exact(2)
            count = second & 127
            if count == 126:
                count = struct.unpack("!H", self.exact(2))[0]
            elif count == 127:
                count = struct.unpack("!Q", self.exact(8))[0]
            assert count <= 64 * 1024 * 1024
            payload = self.exact(count)
            if first & 15 == 9:
                self.send(payload, 10)
            elif first & 15 == 1:
                return json.loads(payload)
            elif first & 15 == 8:
                raise RuntimeError("engine IPC closed")

    def call(self, method, params=None):
        self.sequence += 1
        request_id = self.sequence
        self.send(json.dumps({"id": request_id, "method": method, "params": params or {}}))
        while True:
            reply = self.receive()
            if reply["id"] != request_id:
                continue
            if "err" in reply:
                raise RuntimeError(f"{method}: {reply['err']}")
            if "ok" in reply:
                return reply["ok"]
            if "item" in reply:
                self.send(json.dumps({"id": request_id, "cancel": True}))
                return reply["item"]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--parent", choices=["codex", "claude-code"], default="codex")
    parser.add_argument("--parent-model")
    parser.add_argument("--child-model")
    args = parser.parse_args()
    scratch = Path(tempfile.mkdtemp(prefix="noches-orch-live-"))
    repo = scratch / "repo"
    repo.mkdir()
    subprocess.run(["git", "init", "-q", str(repo)], check=True)
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    env = dict(os.environ, ZERON_DATA_DIR=str(scratch / "data"), NOCHES_DATA_DIR=str(scratch / "data"), ZERON_IPC_PORT=str(port),
               ZERON_HARNESS=args.parent, ZERON_ORCHESTRATION="1",
               ZERON_WORKOS_CLIENT_ID="", ZERON_EDGE_URL="http://127.0.0.1:1",
               RUST_LOG="zeron_engine=info,zeron_harness=info")
    env.pop("ZERON_EDGE_TOKEN", None)
    log = (scratch / "engine.log").open("w")
    process = subprocess.Popen([args.binary, "headless"], env=env, cwd=repo, stdout=log, stderr=log)
    print(f"SCRATCH: {scratch}", flush=True)
    try:
        rpc = None
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                raise RuntimeError(f"headless exited {process.returncode}")
            try:
                rpc = Rpc(port)
                break
            except OSError:
                time.sleep(0.25)
        assert rpc, "headless IPC did not become ready"
        device = rpc.call("EngineInfo")["deviceId"]
        providers = rpc.call("ListProviderInstances")
        parent_driver = "codex" if args.parent == "codex" else "claudeAgent"
        child_driver = "claudeAgent" if args.parent == "codex" else "codex"
        parent = next(p for p in providers if p["driverKind"] == parent_driver)
        child = next(p for p in providers if p["driverKind"] == child_driver)
        print("PROVIDERS: " + json.dumps([{"id": p["providerInstanceId"], "authentication": p["authentication"],
              "models": [m["id"] for m in p["models"]]} for p in [parent, child]]), flush=True)
        parent_model = args.parent_model or next(
            (m["id"] for m in parent["models"] if m["id"] == "gpt-6.1-sol"), parent["models"][0]["id"])
        child_model = args.child_model or next(
            (m["id"] for m in child["models"] if m["id"] == "claude-haiku-4-5"), child["models"][0]["id"])
        target = {"providerInstanceId": child["providerInstanceId"], "model": child_model}
        prompt = (
            "This is a Noches orchestration integration test. Do not change files or run shell commands. "
            "First call the injected t3-code orchestrator_capabilities tool. Then call delegate_task "
            f"exactly once with target {json.dumps(target)}, mode=\"async\", "
            "clientRequestId=\"headless-live-round-1\", task=\"Do not modify files or run commands. "
            "Reply with exactly the word PONG.\". Retain taskId. Say WAITING_FOR_CHILD and finish this "
            "turn if needed. When the app sends 'Delegated task completion available', call task_status "
            "with that taskId to acknowledge its result. Then report exactly 'LIVE_E2E_PASS: child "
            "completed; parent received wake; result=PONG' if the child summary is PONG. "
            "Do not use native subagents, delegate more tasks, poll before the wake, or create threads."
        )
        chat = "headless-live-parent"
        rpc.call("Mutate", {"op": "createChat", "chatId": chat, "deviceId": device, "cwd": str(repo),
                           "config": {"harness": args.parent, "model": parent_model, "sandbox": "workspace-write",
                                      "runtimeMode": "full-access", "interactionMode": "default"}})
        rpc.call("QueueCommand", {"chatId": chat, "command": {"kind": "run", "messageId": "headless-live-input",
                 "request": {"prompt": prompt, "model": parent_model, "reasoning": None, "cwd": str(repo),
                             "sandbox": "danger-full-access", "runtimeMode": "full-access",
                             "interactionMode": "default", "resume": None}}})
        deadline = time.monotonic() + 300
        previous = None
        while time.monotonic() < deadline:
            state = rpc.call("GetOrchestrationState", {"chatId": chat})
            encoded = json.dumps(state, sort_keys=True)
            if encoded != previous:
                print("STATE: " + encoded, flush=True)
                previous = encoded
            tasks = state.get("tasks", []) if isinstance(state, dict) else []
            if tasks and tasks[0]["status"] == "completed" and \
                    tasks[0]["completionDelivery"]["state"] == "acknowledged" and \
                    "LIVE_E2E_PASS" in (state.get("latestResult") or ""):
                transcript = rpc.call("WatchDocMessages", {"chatId": chat})
                (scratch / "parent-transcript.json").write_text(json.dumps(transcript, indent=2))
                (scratch / "state.json").write_text(json.dumps(state, indent=2))
                print("LIVE E2E PASS: Codex/Claude child completed; parent received wake and acknowledged PONG.", flush=True)
                print(state["latestResult"], flush=True)
                return
            if tasks and tasks[0]["status"] in ["failed", "interrupted", "cancelled"]:
                raise RuntimeError("child terminal failure: " + json.dumps(tasks[0]))
            if not tasks and state and state.get("workState") == "result_available" and state.get("latestResult"):
                transcript = rpc.call("WatchDocMessages", {"chatId": chat})
                (scratch / "parent-transcript.json").write_text(json.dumps(transcript, indent=2))
                raise RuntimeError("parent finished without a delegated child: " + state["latestResult"])
            time.sleep(0.5)
        transcript = rpc.call("WatchDocMessages", {"chatId": chat})
        (scratch / "parent-transcript.json").write_text(json.dumps(transcript, indent=2))
        raise RuntimeError("timed out awaiting child completion, wake and parent report; final state=" + str(previous))
    except Exception as error:
        print("LIVE E2E FAIL: " + str(error), flush=True)
        raise
    finally:
        process.send_signal(signal.SIGINT)
        try:
            process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        log.close()
        # Scratch evidence is deliberately retained; no real profile was opened.


if __name__ == "__main__":
    main()
