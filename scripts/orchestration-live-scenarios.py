#!/usr/bin/env python3
"""Opt-in billable live scenarios for Orchestrator V2 (real Claude/Codex CLIs).

Boots one headless `zeron` on a scratch data dir (never a real Noches profile),
then drives the same RPC surface the desktop uses: Mutate / QueueCommand /
QueueMessage / MutateQueuedRun / ForkThread / StopThreadWork /
ResetThreadSession. Each scenario prints one `PASS|FAIL|UNSUPPORTED|SKIP` line
and writes `evidence/<id>.json` under the scratch directory. Reuses the RPC
client from orchestration-live-e2e.py. Stdlib only.

  python3 scripts/orchestration-live-scenarios.py --binary target/debug/zeron
  python3 scripts/orchestration-live-scenarios.py --binary ... --only 1,2

Billing is capped by --max-turns (default 60 provider turns).
"""
import argparse
import base64
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import sqlite3
import struct
import subprocess
import tempfile
import time
import zlib

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("live_e2e", HERE / "orchestration-live-e2e.py")
live_e2e = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(live_e2e)

CLAUDE = {"instance": "claudeAgent", "harness": "claude-code", "model": "claude-haiku-4-5"}
LUNA = {"instance": "codex", "harness": "codex", "model": "gpt-6-luna"}
GEMINI = {"instance": "codex", "harness": "codex", "model": "gemini-3.8-flash"}
# Pi's catalog ids are exact `provider/model` slugs from the owner's CPA provider.
PI = {"instance": "pi", "harness": "pi", "model": "cpa/gemini-3.8-flash"}
PI_LUNA = {"instance": "pi", "harness": "pi", "model": "cpa/gpt-6-luna"}


class Skip(Exception):
    pass


class Unsupported(Exception):
    pass


class Check:
    """Accumulates named assertions so one FAIL line names every broken one."""

    def __init__(self):
        self.items = []

    def ok(self, name, condition, detail=None):
        self.items.append({"check": name, "ok": bool(condition), "detail": detail})
        return bool(condition)

    @property
    def failed(self):
        return [i for i in self.items if not i["ok"]]


def png_solid(width, height, rgb):
    """A valid RGB PNG of one flat color, stdlib only."""

    def chunk(kind, data):
        body = kind + data
        return struct.pack("!I", len(data)) + body + struct.pack("!I", zlib.crc32(body) & 0xFFFFFFFF)

    row = b"\x00" + bytes(rgb) * width
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack("!IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(row * height)) + chunk(b"IEND", b""))


class Engine:
    def __init__(self, binary, scratch, max_turns):
        self.binary = binary
        self.scratch = scratch
        self.data = scratch / "data"
        self.repo = scratch / "repo"
        self.max_turns = max_turns
        self.turns = 0
        self.process = None
        self.rpc = None
        self.seq = 0

    # -- lifecycle ---------------------------------------------------------
    def start(self):
        self.repo.mkdir(parents=True, exist_ok=True)
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        subprocess.run(["git", "-C", str(self.repo), "-c", "user.name=live", "-c", "user.email=live@example.invalid",
                        "commit", "-q", "--allow-empty", "-m", "init"], check=True)
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            port = listener.getsockname()[1]
        env = dict(os.environ, ZERON_DATA_DIR=str(self.data), NOCHES_DATA_DIR=str(self.data),
                   ZERON_IPC_PORT=str(port), ZERON_HARNESS="claude-code", ZERON_ORCHESTRATION="1",
                   ZERON_WORKOS_CLIENT_ID="", ZERON_EDGE_URL="http://127.0.0.1:1",
                   RUST_LOG="zeron_engine=info,zeron_harness=info")
        env.pop("ZERON_EDGE_TOKEN", None)
        self.log = (self.scratch / "engine.log").open("w")
        self.process = subprocess.Popen([self.binary, "headless"], env=env, cwd=self.repo,
                                        stdout=self.log, stderr=self.log)
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError(f"headless exited {self.process.returncode}")
            try:
                self.rpc = live_e2e.Rpc(port)
                break
            except OSError:
                time.sleep(0.25)
        if not self.rpc:
            raise RuntimeError("headless IPC did not become ready")
        self.device = self.call("EngineInfo")["deviceId"]
        self.providers = {p["providerInstanceId"]: p for p in self.call("ListProviderInstances")}

    def stop(self):
        if self.process:
            self.process.send_signal(signal.SIGINT)
            try:
                self.process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
            self.log.close()
        # The engine snapshots installed CLI logins into agent-accounts/ of its
        # scratch profile. Never leave those behind in retained evidence.
        shutil.rmtree(self.data / "agent-accounts", ignore_errors=True)

    def call(self, method, params=None):
        return self.rpc.call(method, params or {})

    # -- chats and turns ---------------------------------------------------
    def require_model(self, who):
        provider = self.providers.get(who["instance"])
        if not provider or provider["authentication"] != "authenticated":
            raise Skip(f"{who['instance']} is not authenticated")
        if who["model"] not in [m["id"] for m in provider["models"]]:
            raise Skip(f"{who['instance']} does not advertise {who['model']}")

    def config(self, who, reasoning=None, options=None):
        return {"instanceId": who["instance"], "harness": who["harness"], "model": who["model"],
                "reasoning": reasoning, "modelOptions": options or {}, "sandbox": "danger-full-access",
                "runtimeMode": "full-access", "interactionMode": "default"}

    def create_chat(self, chat, who, reasoning=None, options=None, cwd=None):
        self.call("Mutate", {"op": "createChat", "chatId": chat, "deviceId": self.device,
                             "cwd": str(cwd or self.repo), "config": self.config(who, reasoning, options)})

    def set_config(self, chat, who, reasoning=None, options=None):
        self.call("Mutate", {"op": "setChatConfig", "chatId": chat, "config": self.config(who, reasoning, options)})

    def spend(self, n=1):
        self.turns += n
        if self.turns > self.max_turns:
            raise RuntimeError(f"provider turn budget {self.max_turns} exhausted")

    def run(self, chat, prompt, who, reasoning=None, options=None, message_id=None, attachments=None):
        self.spend()
        self.seq += 1
        message_id = message_id or f"{chat}-m{self.seq}"
        request = {"instanceId": who["instance"], "harness": who["harness"], "prompt": prompt,
                   "model": who["model"], "reasoning": reasoning, "modelOptions": options or {},
                   "cwd": str(self.repo), "sandbox": "danger-full-access", "runtimeMode": "full-access",
                   "interactionMode": "default", "resume": None, "attachments": attachments or []}
        self.call("QueueCommand", {"chatId": chat, "command": {"kind": "run", "messageId": message_id,
                                                              "request": request}})
        # A provider switch can take seconds to dispatch. Return only once the
        # host has accepted the dispatch, so wait_idle() can never observe the
        # gap before the run exists (and a following command can't supersede it).
        self.wait_for(lambda: self.sql("select 1 from orchestration_command_receipts where command_id=? "
                                       "and status='accepted'", (f"session-run:{message_id}",)),
                      timeout=120, interval=0.1, what=f"dispatch of {message_id}")
        return message_id

    def queue_message(self, chat, text, hold=True):
        return self.call("QueueMessage", {"chatId": chat, "text": text, "holdForTurnEnd": hold})["id"]

    def upload(self, name, data):
        upload_id = f"live-{os.urandom(8).hex()}"  # the engine jails ids to [A-Za-z0-9_-]
        self.call("UploadChunk", {"uploadId": upload_id, "data": base64.b64encode(data).decode(), "seq": 0})
        return self.call("UploadCommit", {"uploadId": upload_id, "fileName": name})["path"]

    def descendants(self):
        """(pid, command) of every process under the engine; the leaf `lsof` probe is noise."""
        out = subprocess.run(["ps", "-A", "-o", "pid=,ppid=,command="], capture_output=True, text=True).stdout
        table = {}
        for line in out.splitlines():
            parts = line.split(None, 2)
            if len(parts) == 3 and parts[0].isdigit() and parts[1].isdigit():
                table[int(parts[0])] = (int(parts[1]), parts[2])
        found, frontier = [], [self.process.pid]
        while frontier:
            parent = frontier.pop()
            for pid, (ppid, command) in table.items():
                if ppid == parent:
                    found.append((pid, command))
                    frontier.append(pid)
        return [(pid, command[:120]) for pid, command in found if "lsof" not in command]

    # -- observation -------------------------------------------------------
    def state(self, chat):
        return self.call("GetOrchestrationState", {"chatId": chat})

    def queue_state(self, chat):
        return self.call("GetQueueState", {"chatId": chat})

    def transfer_state(self, chat):
        return self.call("GetThreadTransferState", {"chatId": chat})

    def transcript(self, chat):
        reply = self.call("WatchDocMessages", {"chatId": chat})
        return reply.get("reset", reply) if isinstance(reply, dict) else reply

    def journal(self, chat):
        # The engine names a journal after the thread id with every other character mapped to '_'.
        name = "".join(c if c.isalnum() or c in "-_" else "_" for c in chat)
        path = self.data / "orgs" / "dev-org" / "dev-user" / "journals" / f"{name}.jsonl"
        if not path.exists():
            return []
        events = []
        for line in path.read_text().splitlines():
            try:
                events.append(json.loads(line))
            except ValueError:
                pass  # a half-written tail line while the engine appends
        return events

    def sql(self, query, params=()):
        path = self.data / "orgs" / "dev-org" / "dev-user" / "docs.sqlite3"
        db = sqlite3.connect(f"file:{path}?mode=ro", uri=True, timeout=10)
        db.row_factory = sqlite3.Row
        try:
            return [dict(row) for row in db.execute(query, params)]
        finally:
            db.close()

    def is_active(self, chat):
        queue = self.queue_state(chat)
        return bool(queue.get("activeRunId")) or self.state(chat).get("workState") == "working"

    def wait_idle(self, chat, timeout=240, settle=2.0):
        deadline = time.monotonic() + timeout
        quiet_since = None
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError("engine exited")
            if self.is_active(chat):
                quiet_since = None
            else:
                quiet_since = quiet_since or time.monotonic()
                if time.monotonic() - quiet_since >= settle:
                    return
            time.sleep(0.5)
        raise RuntimeError(f"{chat} did not become idle in {timeout}s")

    def wait_for(self, predicate, timeout=120, interval=0.15, what="condition"):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            value = predicate()
            if value:
                return value
            time.sleep(interval)
        raise RuntimeError(f"timed out waiting for {what}")


# -- transcript helpers -----------------------------------------------------
def message_text(message):
    return "".join(p.get("text", "") for p in message.get("parts", []) if p.get("kind") == "text")


def assistant_text(transcript):
    return "\n".join(message_text(m) for m in transcript if m["role"] == "assistant")


def last_assistant_text(transcript):
    texts = [message_text(m) for m in transcript if m["role"] == "assistant" and message_text(m).strip()]
    return texts[-1] if texts else ""


def user_texts(transcript):
    return [message_text(m) for m in transcript if m["role"] == "user"]


def events_of(journal, kind):
    return [e["event"] for e in journal if e["event"].get("type") == kind]


def claude_session_file(cwd, session_id):
    """~/.claude/projects/<cwd with non-alphanumerics as '-'>/<session>.jsonl (read-only)."""
    resolved = str(Path(cwd).resolve())
    encoded = "".join(c if c.isalnum() else "-" for c in resolved)
    return Path.home() / ".claude" / "projects" / encoded / f"{session_id}.jsonl"


def sha(path):
    import hashlib
    return hashlib.sha256(path.read_bytes()).hexdigest() if path.exists() else None


STOP_PARENT = "claude"  # scenario 8 parent (see --stop-parent)
RESET_PAUSE_SECONDS = 4  # scenario 9: --reset-pause 0 reproduces the immediate-send race
SCENARIOS = []


def scenario(number, title):
    def register(function):
        SCENARIOS.append((number, title, function))
        return function
    return register


def promote_steer(engine, chat, queued_run_id, tag):
    """The desktop's Steer gesture on a held queue row (canonical queue)."""
    queue = engine.queue_state(chat)
    return engine.call("MutateQueuedRun", {
        "chatId": chat, "queuedRunId": queued_run_id, "clientRequestId": f"live-{tag}",
        "action": {"type": "promoteToSteer", "targetRunId": queue["activeRunId"],
                   "expectedSelection": queue.get("promotionSelection")}}), queue


def queue_entry(engine, chat, message_id, timeout=20):
    def find():
        for entry in engine.queue_state(chat).get("queue", []):
            if entry["messageId"] == message_id:
                return entry
    return engine.wait_for(find, timeout=timeout, interval=0.1, what="queued row")


def steer_once(engine, chat, text, tag, evidence):
    message_id = engine.queue_message(chat, text, hold=True)
    entry = queue_entry(engine, chat, message_id)
    reply, queue = promote_steer(engine, chat, entry["queuedRunId"], tag)
    evidence.setdefault("promotions", []).append({"tag": tag, "messageId": message_id, "reply": reply,
                                                   "mode": queue.get("promotionMode")})
    return message_id, reply


def steer_delivery(engine, chat, message_id):
    """Everything that proves one native delivery of a steer message."""
    journal = engine.journal(chat)
    return {
        "inputAcceptedFor": [e for e in events_of(journal, "inputAcceptedFor") if e.get("messageId") == message_id],
        "acceptances": engine.sql("select effect_id, run_attempt_id, provider_session_id from "
                                  "orchestration_steering_acceptances where message_id=? and thread_id=?",
                                  (message_id, chat)),
        "transcriptCount": sum(1 for m in engine.transcript(chat) if m["id"] == message_id),
    }


@scenario(1, "Claude receipts: steer mid-tool and right after the last tool call")
def claude_receipts(engine, checks, evidence):
    engine.require_model(CLAUDE)
    chat = "s1"
    engine.create_chat(chat, CLAUDE)
    # A: steer while the tool is still running.
    engine.run(chat, "Run `sleep 10` with your shell tool, then reply with exactly: ALPHA", CLAUDE)
    engine.wait_for(lambda: events_of(engine.journal(chat), "toolCall"), what="first tool call")
    token_a = "STEER-A-7731"
    id_a, reply_a = steer_once(engine, chat, f"{token_a}: when you finish, also append the word BRAVO.", "a", evidence)
    checks.ok("A promotion accepted", reply_a.get("refusal") is None, reply_a)
    # Long settle: a steer that lands after the final text is answered by a
    # trailing CLI turn that finishes ~1-2 s after the first `done`.
    engine.wait_idle(chat, settle=8)
    # B: steer immediately after the last tool result, before the final answer.
    # The turn can finish before the steer lands; retry (bounded) so the
    # scenario measures delivery, not a lost race.
    token_b = id_b = None
    for attempt in range(3):
        seen = len(events_of(engine.journal(chat), "toolResult"))
        engine.run(chat, "Run `ls` with your shell tool, then reply with exactly: GAMMA", CLAUDE)
        engine.wait_for(lambda: len(events_of(engine.journal(chat), "toolResult")) > seen, timeout=60,
                        interval=0.03, what="tool result")
        if not engine.queue_state(chat).get("activeRunId"):
            engine.wait_idle(chat, settle=3)
            evidence.setdefault("bLostRace", 0)
            evidence["bLostRace"] += 1
            continue
        token_b = "STEER-B-4492"
        id_b, reply_b = steer_once(engine, chat, f"{token_b}: also append the word DELTA.", "b", evidence)
        break
    if not checks.ok("B: steer landed while a run was active", id_b is not None, evidence.get("bLostRace")):
        return
    # Sample the idle gap: how long the thread reports no active run while the
    # trailing steer reply is still being produced.
    samples = []
    gap_started = None
    deadline = time.monotonic() + 40
    while time.monotonic() < deadline:
        active = engine.is_active(chat)
        dones = len(events_of(engine.journal(chat), "done"))
        samples.append({"t": round(time.monotonic(), 2), "active": active, "dones": dones})
        if not active:
            gap_started = gap_started or time.monotonic()
            if time.monotonic() - gap_started > 8:
                break
        else:
            gap_started = None
        time.sleep(0.1)
    evidence["stateSamples"] = samples[-60:]
    transcript = engine.transcript(chat)
    journal = engine.journal(chat)
    evidence["delivery"] = {"a": steer_delivery(engine, chat, id_a), "b": steer_delivery(engine, chat, id_b)}
    evidence["userMessages"] = user_texts(transcript)
    evidence["assistantTexts"] = [message_text(m) for m in transcript if m["role"] == "assistant"]
    evidence["eventTypes"] = [e["event"]["type"] for e in journal
                              if e["event"]["type"] not in ("reasoningDelta", "textDelta")]
    evidence["effects"] = engine.sql("select effect_type, status, count(*) n from orchestration_effect_outbox "
                                     "where thread_id=? group by 1,2", (chat,))
    evidence["runs"] = engine.sql("select run_id, status from orchestration_projection_runs where thread_id=? "
                                  "order by ordinal", (chat,))
    for tag, token, mid in (("A", token_a, id_a), ("B", token_b, id_b)):
        delivery = evidence["delivery"][tag.lower()]
        checks.ok(f"{tag}: exactly one user message carries the steer",
                  sum(token in t for t in user_texts(transcript)) == 1, user_texts(transcript))
        checks.ok(f"{tag}: transcript has the message id once", delivery["transcriptCount"] == 1, delivery)
        checks.ok(f"{tag}: native receipt recorded once", len(delivery["inputAcceptedFor"]) == 1, delivery)
        checks.ok(f"{tag}: steering acceptance row recorded once", len(delivery["acceptances"]) == 1, delivery)
    steers = sum(e["n"] for e in evidence["effects"] if e["effect_type"] == "provider-turn.steer")
    checks.ok("two steer effects, both succeeded", steers == 2 and not [
        e for e in evidence["effects"] if e["effect_type"] == "provider-turn.steer" and e["status"] != "succeeded"],
        evidence["effects"])
    # An orphan redispatch would re-send a steer text as its own provider turn.
    redispatch = engine.sql("select effect_type, count(*) n from orchestration_effect_outbox where thread_id=? "
                            "and effect_type != 'provider-turn.steer' and (payload_json like ? or payload_json like ?) "
                            "group by 1", (chat, f"%{token_a}%", f"%{token_b}%"))
    checks.ok("no orphan redispatch of either steer text", not redispatch, redispatch)
    bad = [e for e in evidence["effects"] if e["status"] in ("failed", "uncertain", "pending", "running")]
    checks.ok("no failed/uncertain/pending effects", not bad, bad)
    checks.ok("no failed runs", not [r for r in evidence["runs"] if r["status"] == "failed"], evidence["runs"])
    final = "\n".join(evidence["assistantTexts"])
    checks.ok("A steer honored (BRAVO)", "BRAVO" in final, evidence["assistantTexts"])
    checks.ok("B steer honored (DELTA)", "DELTA" in final, evidence["assistantTexts"])


def fork_thread(engine, parent, child, point, tag):
    reply = engine.call("ForkThread", {"chatId": parent, "commandId": f"live-fork-{tag}", "targetChatId": child,
                                       "sourcePoint": point, "title": f"fork {tag}"})
    if reply.get("refusal"):
        raise RuntimeError(f"fork {tag} refused: {reply['refusal']}")
    return reply


def grep_sql(engine, needle):
    """Every (table, column) whose text mentions needle: where an id is recorded."""
    hits = []
    for table in [r["name"] for r in engine.sql("select name from sqlite_master where type='table'")]:
        columns = [r["name"] for r in engine.sql(f"pragma table_info({table})")]
        for column in columns:
            rows = engine.sql(f'select substr("{column}", 1, 400) v from {table} where cast("{column}" as text) like ? limit 3',
                              (f"%{needle}%",))
            if rows:
                hits.append({"table": table, "column": column, "sample": rows[0]["v"]})
    return hits


def fork_token(engine, child):
    """The deferred native id the host minted at fork time (claude-fork:v1:<child>:<parent>:<turn>)."""
    import re
    for row in engine.sql("select envelope_json from orchestration_events where stream_id=? "
                          "and envelope_json like '%claude-fork:v1:%' order by sequence limit 1", (child,)):
        found = re.search(r"claude-fork:v1:([0-9a-f-]+):([0-9a-f-]+):([0-9a-f-]+)", row["envelope_json"])
        if found:
            return {"child": found.group(1), "parent": found.group(2), "turn": found.group(3)}
    return None


@scenario(2, "Claude lazy native fork: turn-1 checkpoint vs head")
def claude_lazy_fork(engine, checks, evidence):
    engine.require_model(CLAUDE)
    parent = "s2p"
    engine.create_chat(parent, CLAUDE)
    engine.run(parent, "Remember: X=1. Reply with exactly: OK1", CLAUDE)
    engine.wait_idle(parent)
    engine.run(parent, "Remember: Y=2. Reply with exactly: OK2", CLAUDE)
    engine.wait_idle(parent)
    parent_session = events_of(engine.journal(parent), "sessionStarted")[0]["sessionId"]
    parent_file = claude_session_file(engine.repo, parent_session)
    before = sha(parent_file)
    evidence["parentSession"] = parent_session
    checks.ok("parent session file exists", before is not None, str(parent_file))
    state = engine.transfer_state(parent)
    turn1 = next(c["checkpoint"]["id"] for c in state["checkpoints"]
                 if c["checkpoint"]["runId"].endswith(":1") and c["phase"] == "completed")
    ask = ("What do you remember from this conversation? Answer in one line exactly as "
           "`X=<value or unknown>; Y=<value or unknown>`. Use `unknown` for anything you were not told.")
    forks = {}
    for name, point in (("turn1", {"type": "checkpoint", "checkpointId": turn1}), ("head", {"type": "latest_stable"})):
        child = f"s2-{name}"
        fork_thread(engine, parent, child, point, name)
        started = engine.transfer_state(child)
        fork_transfer = (started.get("transfers") or [{}])[0]
        evidence[f"{name}Transfer"] = fork_transfer
        # Lazy: creating the fork must not have started a provider session.
        checks.ok(f"{name}: forking starts no provider process", not engine.journal(child),
                  len(engine.journal(child)))
        engine.run(child, ask, CLAUDE)
        engine.wait_idle(child)
        forks[name] = child
    for name, child in forks.items():
        transcript = engine.transcript(child)
        answer = last_assistant_text(transcript)
        evidence[f"{name}Answer"] = answer
        started = events_of(engine.journal(child), "sessionStarted")
        session = started[0]["sessionId"] if started else None
        evidence[f"{name}Session"] = session
        evidence[f"{name}SessionFile"] = str(claude_session_file(engine.repo, session)) if session else None
        checks.ok(f"{name}: child got its own session id", bool(session) and session != parent_session, session)
        checks.ok(f"{name}: child session file exists", bool(session) and claude_session_file(engine.repo, session).exists())
        token = fork_token(engine, child)
        evidence[f"{name}Token"] = token
        checks.ok(f"{name}: child session id equals the host-assigned fork id",
                  bool(token) and token["child"] == session, {"token": token, "session": session})
        checks.ok(f"{name}: fork token names the parent session", bool(token) and token["parent"] == parent_session,
                  token)
        turn_refs = [e["turnId"] for e in events_of(engine.journal(parent), "nativeReference")]
        expected_turn = turn_refs[0] if name == "turn1" else turn_refs[-1]
        checks.ok(f"{name}: fork cut at the expected parent turn", bool(token) and token["turn"] == expected_turn,
                  {"token": token, "turnRefs": turn_refs})
    turn1_answer = evidence["turn1Answer"].replace(" ", "")
    head_answer = evidence["headAnswer"].replace(" ", "")
    checks.ok("turn-1 fork knows X only", "X=1" in turn1_answer and "Y=2" not in turn1_answer, evidence["turn1Answer"])
    checks.ok("head fork knows X and Y", "X=1" in head_answer and "Y=2" in head_answer, evidence["headAnswer"])
    checks.ok("parent session file unchanged by forking", sha(parent_file) == before, parent_file.name)
    checks.ok("children are distinct native sessions", evidence["turn1Session"] != evidence["headSession"])


def opt(level):
    return {"reasoningEffort": level}


def codex_rollout(thread_id):
    """The Codex CLI's own record of a native thread (read-only), or None."""
    root = Path.home() / ".codex" / "sessions"
    found = sorted(root.glob(f"**/rollout-*{thread_id}.jsonl")) if root.exists() else []
    return found[0] if found else None


def codex_turn_contexts(thread_id):
    """[(model, effort)] per turn as the provider itself recorded them."""
    path = codex_rollout(thread_id)
    contexts = []
    for line in (path.read_text().splitlines() if path else []):
        try:
            record = json.loads(line)
        except ValueError:
            continue
        if record.get("type") == "turn_context":
            payload = record.get("payload", {})
            contexts.append({"model": payload.get("model"), "effort": payload.get("effort"),
                             "turn": payload.get("turn_id")})
    return contexts


@scenario(3, "Codex selection transitions: effort and model change keep the native thread")
def codex_selection(engine, checks, evidence):
    engine.require_model(LUNA)
    engine.require_model(GEMINI)
    chat = "s3"
    engine.create_chat(chat, LUNA, reasoning="medium", options=opt("medium"))
    plan = [(LUNA, "medium", "Remember K=17. Reply with exactly: READY"),
            (LUNA, "high", "What is K? Reply with only the number."),
            (GEMINI, "medium", "What is K? Reply with only the number.")]
    for index, (who, effort, prompt) in enumerate(plan):
        if index:
            engine.set_config(chat, who, reasoning=effort, options=opt(effort))
        engine.run(chat, prompt, who, reasoning=effort, options=opt(effort))
        engine.wait_idle(chat, timeout=240)
    transcript = engine.transcript(chat)
    answers = [message_text(m).strip() for m in transcript if m["role"] == "assistant" and message_text(m).strip()]
    evidence["answers"] = answers
    started = events_of(engine.journal(chat), "sessionStarted")
    evidence["sessionStarted"] = [{k: e.get(k) for k in ("instanceId", "model", "sessionId")} for e in started]
    threads = {e["sessionId"] for e in started}
    checks.ok("all three turns ran on the SAME native thread", len(threads) == 1 and len(started) == 3,
              evidence["sessionStarted"])
    state = engine.transfer_state(chat)
    evidence["transfers"], evidence["handoffs"] = state["transfers"], state["handoffs"]
    checks.ok("no portable handoff or transfer was created", not state["transfers"] and not state["handoffs"],
              {"transfers": state["transfers"], "handoffs": state["handoffs"]})
    runs = engine.sql("select run_id, payload_json from orchestration_projection_runs where thread_id=? order by ordinal",
                      (chat,))
    selections = [json.loads(r["payload_json"]) for r in runs]
    evidence["runSelections"] = [{"run": r["id"], "modelSelection": r.get("modelSelection"),
                                  "contextHandoffId": r.get("contextHandoffId")} for r in selections]
    checks.ok("no run carries a context handoff", all(not r.get("contextHandoffId") for r in selections),
              evidence["runSelections"])
    checks.ok("later turns still remember K (native resume, not a fresh thread)",
              len(answers) == 3 and "17" in answers[1] and "17" in answers[2], answers)
    thread_id = next(iter(threads))
    contexts = codex_turn_contexts(thread_id)
    evidence["providerTurnContexts"] = contexts
    checks.ok("the provider recorded the requested model/effort for each turn",
              [(c["model"], c["effort"]) for c in contexts] ==
              [("gpt-6-luna", "medium"), ("gpt-6-luna", "high"), ("gemini-3.8-flash", "medium")], contexts)


@scenario(4, "Codex queue promotion: model change + interrupt/restart on the native thread")
def codex_queue_promotion(engine, checks, evidence):
    engine.require_model(LUNA)
    engine.require_model(GEMINI)
    chat = "s4"
    engine.create_chat(chat, LUNA, reasoning="medium", options=opt("medium"))
    engine.run(chat, "Run `sleep 25` with your shell tool, then reply with exactly: DONE1", LUNA,
               reasoning="medium", options=opt("medium"))
    engine.wait_for(lambda: events_of(engine.journal(chat), "toolCall"), timeout=90, what="long tool call")
    before = engine.descendants()
    evidence["processesBefore"] = before
    first = engine.queue_message(chat, "Follow-up one: reply with exactly: F1")
    second = engine.queue_message(chat, "Follow-up two: reply with exactly: F2")
    # The composer changes the model while the turn runs.
    engine.set_config(chat, GEMINI, reasoning="medium", options=opt("medium"))
    def restart_ready():
        state = engine.queue_state(chat)
        if state.get("promotionMode") == "interrupt_restart" and len(state.get("queue", [])) == 2:
            return state
    queue = engine.wait_for(restart_ready, timeout=20, what="restart promotion mode")
    evidence["queueBefore"] = queue
    selection = queue.get("promotionSelection") or {}
    checks.ok("promotion mode follows the composer model change",
              selection.get("model") == "gemini-3.8-flash" and not queue.get("canPromoteToSteer"), selection)
    entry = next(e for e in queue["queue"] if e["messageId"] == first)
    reply = engine.call("MutateQueuedRun", {
        "chatId": chat, "queuedRunId": entry["queuedRunId"], "clientRequestId": "live-promote-restart",
        "action": {"type": "promoteToRestart", "targetRunId": queue["activeRunId"], "handoff": False,
                   "expectedSelection": queue.get("promotionSelection")}})
    evidence["promotion"] = reply
    checks.ok("promotion accepted", reply.get("refusal") is None, reply)
    engine.spend(2)  # the restarted turn and the second follow-up are provider turns too
    # Untouched follow-up: still queued, same id and text, right after the promotion.
    still = engine.wait_for(lambda: next((e for e in engine.queue_state(chat)["queue"]
                                           if e["messageId"] == second), None), timeout=10, what="second row")
    evidence["secondRowRightAfterPromotion"] = still
    checks.ok("second queued follow-up untouched (same id and text) right after the promotion",
              still["text"] == "Follow-up two: reply with exactly: F2")
    time.sleep(6)
    after = engine.descendants()
    evidence["processesAfter"] = after
    checks.ok("old turn's tool process released (no leftover `sleep 25`)",
              not [p for p in after if "sleep 25" in p[1]], after)
    checks.ok("single Codex app-server (no leaked second process)",
              len([p for p in after if "codex" in p[1] and "app-server" in p[1]]) == 1, after)
    engine.wait_idle(chat, timeout=240)
    started = events_of(engine.journal(chat), "sessionStarted")
    dones = events_of(engine.journal(chat), "done")
    evidence["sessionStarted"] = [{k: e.get(k) for k in ("model", "sessionId")} for e in started]
    evidence["dones"] = [{k: e.get(k) for k in ("status", "sessionId")} for e in dones]
    threads = {e["sessionId"] for e in started}
    checks.ok("restart resumed the SAME native thread", len(threads) == 1 and len(started) >= 2, evidence["sessionStarted"])
    checks.ok("first turn ended interrupted, restart on gemini-3.8-flash",
              dones and dones[0]["status"] == "interrupted" and started[1]["model"] == "gemini-3.8-flash",
              {"dones": evidence["dones"], "started": evidence["sessionStarted"]})
    transcript = engine.transcript(chat)
    texts = user_texts(transcript)
    evidence["userMessages"] = texts
    evidence["assistantTexts"] = [message_text(m).strip() for m in transcript if m["role"] == "assistant"]
    checks.ok("each follow-up delivered exactly once, in order",
              [t for t in texts if t.startswith("Follow-up")] == [
                  "Follow-up one: reply with exactly: F1", "Follow-up two: reply with exactly: F2"], texts)
    checks.ok("both follow-ups answered", "F1" in evidence["assistantTexts"] and "F2" in evidence["assistantTexts"],
              evidence["assistantTexts"])
    state = engine.transfer_state(chat)
    checks.ok("no portable handoff for a same-thread restart", not state["handoffs"] and not state["transfers"],
              {"handoffs": state["handoffs"], "transfers": state["transfers"]})
    contexts = codex_turn_contexts(next(iter(threads)))
    evidence["providerTurnContexts"] = contexts
    checks.ok("provider ran the restart and the follow-up on gemini-3.8-flash",
              [c["model"] for c in contexts][:2] == ["gpt-6-luna", "gemini-3.8-flash"]
              and all(c["model"] == "gemini-3.8-flash" for c in contexts[1:]), contexts)
    checks.ok("queue drained", not engine.queue_state(chat)["queue"])


@scenario(5, "Cross-provider A->B->A: Haiku -> gpt-6-luna -> Haiku with portable context then a delta")
def cross_provider(engine, checks, evidence):
    engine.require_model(CLAUDE)
    engine.require_model(LUNA)
    chat = "s5"
    engine.create_chat(chat, CLAUDE)
    engine.run(chat, "Remember: the project name is AURORA. Reply with exactly: OK-A1", CLAUDE)
    engine.wait_idle(chat)
    engine.set_config(chat, LUNA, reasoning="medium", options=opt("medium"))
    engine.run(chat, "Earlier in this conversation you were given a project name. State it. "
                     "Also remember: the build number is 4242. Reply as `name=<project name>; ok`.", LUNA,
               reasoning="medium", options=opt("medium"))
    engine.wait_idle(chat, timeout=240)
    engine.set_config(chat, CLAUDE)
    engine.run(chat, "Without tools, what is the project name and what is the build number? "
                     "Reply as `name=<n>; build=<b>`.",
               CLAUDE)
    engine.wait_idle(chat)
    transcript = engine.transcript(chat)
    answers = [message_text(m).strip() for m in transcript if m["role"] == "assistant" and message_text(m).strip()]
    evidence["answers"] = answers
    state = engine.transfer_state(chat)
    evidence["handoffs"] = state["handoffs"]
    evidence["transfers"] = state["transfers"]
    runs = [json.loads(r["payload_json"]) for r in engine.sql(
        "select payload_json from orchestration_projection_runs where thread_id=? order by ordinal", (chat,))]
    evidence["runs"] = [{"run": r["id"], "instance": r["providerInstanceId"], "contextHandoffId": r.get("contextHandoffId"),
                         "thread": r.get("providerThreadId")} for r in runs]
    started = events_of(engine.journal(chat), "sessionStarted")
    evidence["sessionStarted"] = [{k: e.get(k) for k in ("instanceId", "model", "sessionId")} for e in started]
    claude_sessions = [e["sessionId"] for e in started if e["instanceId"] == "claudeAgent"]
    codex_sessions = [e["sessionId"] for e in started if e["instanceId"] == "codex"]
    checks.ok("B (codex) answered with A's project name via portable context", len(answers) >= 2 and "AURORA" in answers[1].upper(),
              answers)
    checks.ok("A resumed its OWN native session (same Claude session id on turn 3)",
              len(claude_sessions) == 2 and claude_sessions[0] == claude_sessions[1], evidence["sessionStarted"])
    checks.ok("A recalls the project name (native) and the build number told only to B (delta handoff)",
              answers and "AURORA" in answers[-1].upper() and "4242" in answers[-1], answers[-1:])
    checks.ok("codex ran on a distinct native thread", len(codex_sessions) == 1 and codex_sessions[0] not in claude_sessions,
              evidence["sessionStarted"])
    with_handoff = [r for r in evidence["runs"] if r["contextHandoffId"]]
    checks.ok("both provider switches carried a context handoff (turn 2 portable, turn 3 delta)",
              len(with_handoff) == 2 and with_handoff[0]["instance"] == "codex" and with_handoff[1]["instance"] == "claudeAgent",
              evidence["runs"])
    checks.ok("two handoffs recorded", len(state["handoffs"]) == 2, state["handoffs"])


def multimodal_steer(engine, checks, evidence, who, label, effort=None):
    chat = f"s6-{label}"
    options = opt(effort) if effort else None
    engine.create_chat(chat, who, reasoning=effort, options=options)
    engine.run(chat, "Run `sleep 14` with your shell tool, then reply with exactly: WAITED", who,
               reasoning=effort, options=options)
    engine.wait_for(lambda: events_of(engine.journal(chat), "toolCall"), timeout=90, what="long tool call")
    image = engine.upload("red.png", png_solid(64, 64, (255, 0, 0)))
    evidence[f"{label}Image"] = image
    message_id = engine.call("QueueMessage", {
        "chatId": chat, "attachments": [image], "holdForTurnEnd": True,
        "text": "Look at the attached image. What is its single dominant color? "
                "Answer with exactly one lowercase word, after you finish waiting."})["id"]
    entry = queue_entry(engine, chat, message_id)
    evidence[f"{label}QueueEntry"] = entry
    reply, queue = promote_steer(engine, chat, entry["queuedRunId"], f"img-{label}")
    evidence[f"{label}Promotion"] = {"reply": reply, "mode": queue.get("promotionMode")}
    checks.ok(f"{label}: image steer promotion accepted", reply.get("refusal") is None, reply)
    engine.wait_idle(chat, timeout=240, settle=6)
    transcript = engine.transcript(chat)
    answers = [message_text(m).strip() for m in transcript if m["role"] == "assistant" and message_text(m).strip()]
    evidence[f"{label}Answers"] = answers
    delivery = steer_delivery(engine, chat, message_id)
    evidence[f"{label}Delivery"] = delivery
    checks.ok(f"{label}: steer delivered natively once (receipt + acceptance row)",
              len(delivery["inputAcceptedFor"]) == 1 and len(delivery["acceptances"]) == 1
              and delivery["transcriptCount"] == 1, delivery)
    checks.ok(f"{label}: model answered red", any("red" in a.lower() for a in answers), answers)


@scenario(6, "Multimodal steer: red PNG into a running Codex turn and a Claude turn")
def multimodal(engine, checks, evidence):
    engine.require_model(LUNA)
    engine.require_model(CLAUDE)
    multimodal_steer(engine, checks, evidence, LUNA, "codex", "medium")
    multimodal_steer(engine, checks, evidence, CLAUDE, "claude")


@scenario(7, "Codex native fork at a turn boundary; legacy revert primitives on the installed app-server")
def codex_fork_revert(engine, checks, evidence):
    engine.require_model(LUNA)
    parent = "s7p"
    engine.create_chat(parent, LUNA, reasoning="medium", options=opt("medium"))
    engine.run(parent, "Remember: X=1. Reply with exactly: OK1", LUNA, reasoning="medium", options=opt("medium"))
    engine.wait_idle(parent, timeout=240)
    engine.run(parent, "Remember: Y=2. Reply with exactly: OK2", LUNA, reasoning="medium", options=opt("medium"))
    engine.wait_idle(parent, timeout=240)
    state = engine.transfer_state(parent)
    turn1 = next(c["checkpoint"]["id"] for c in state["checkpoints"]
                 if c["checkpoint"]["runId"].endswith(":1") and c["phase"] == "completed")
    parent_thread = events_of(engine.journal(parent), "sessionStarted")[0]["sessionId"]
    evidence["parentThread"] = parent_thread
    parent_before = sha(codex_rollout(parent_thread))
    child = "s7-turn1"
    fork_thread(engine, parent, child, {"type": "checkpoint", "checkpointId": turn1}, "codex-turn1")
    transfer = (engine.transfer_state(child).get("transfers") or [{}])[0]
    evidence["forkTransfer"] = transfer
    engine.run(child, "What do you remember from this conversation? Answer in one line exactly as "
                      "`X=<value or unknown>; Y=<value or unknown>`.", LUNA, reasoning="medium", options=opt("medium"))
    engine.wait_idle(child, timeout=240)
    answer = last_assistant_text(engine.transcript(child))
    evidence["turn1ForkAnswer"] = answer
    started = events_of(engine.journal(child), "sessionStarted")
    child_thread = started[0]["sessionId"] if started else None
    evidence["childThread"] = child_thread
    checks.ok("turn-1 fork knows X only", "X=1" in answer.replace(" ", "") and "Y=2" not in answer.replace(" ", ""), answer)
    checks.ok("child is a distinct native Codex thread", bool(child_thread) and child_thread != parent_thread, child_thread)
    rollout = codex_rollout(child_thread) if child_thread else None
    evidence["childRolloutForkedFrom"] = None
    if rollout:
        meta = json.loads(rollout.read_text().splitlines()[0]).get("payload", {})
        evidence["childRolloutForkedFrom"] = meta.get("forked_from_id") or meta.get("parent_thread_id")
    checks.ok("child rollout records the parent thread (native thread/fork)",
              evidence["childRolloutForkedFrom"] == parent_thread, evidence["childRolloutForkedFrom"])
    evidence["parentRolloutHashBefore"] = parent_before
    probe = probe_codex_revert(engine.repo, parent_thread)
    evidence["legacyRevertProbe"] = probe
    if probe.get("unsupported"):
        raise Unsupported("legacy history revert: " + probe["unsupported"])
    checks.ok("installed app-server uses paginated history (legacy revert primitives apply)",
              probe.get("historyMode") == "paginated", probe)
    checks.ok("fork + thread/turns/list + thread/revert leave exactly the first turn",
              probe.get("turnsAfterRevert") == 1 and probe.get("turnsBefore") == 2, probe)


class CodexAppServer:
    """Minimal stdio JSON-RPC client for the installed `codex app-server` (no model turns)."""

    def __init__(self, cwd):
        self.process = subprocess.Popen(["codex", "app-server"], cwd=cwd, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        self.count = 0
        self.request("initialize", {"clientInfo": {"name": "noches-live", "version": "0"},
                                    "capabilities": {"experimentalApi": True}})
        self.send({"method": "initialized", "params": {}})

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def request(self, method, params):
        self.count += 1
        self.send({"id": self.count, "method": method, "params": params})
        while True:
            line = self.process.stdout.readline()
            if not line:
                raise RuntimeError("codex app-server exited")
            reply = json.loads(line)
            if reply.get("id") == self.count:
                if "error" in reply:
                    raise RuntimeError(f"{method}: {reply['error']}")
                return reply["result"]

    def close(self):
        self.process.terminate()
        self.process.wait(timeout=10)


def probe_codex_revert(cwd, parent_thread):
    """The exact calls revert_forked_turns() makes, against a fork of the live parent thread."""
    server = CodexAppServer(cwd)
    probe = {}
    try:
        forked = server.request("thread/fork", {"threadId": parent_thread, "cwd": str(cwd)})
        fork_id = forked["thread"]["id"]
        probe["forkId"] = fork_id
        read = server.request("thread/read", {"threadId": fork_id, "includeTurns": False})
        probe["historyMode"] = read["thread"].get("historyMode")
        if probe["historyMode"] != "paginated":
            probe["unsupported"] = f"fork uses historyMode={probe['historyMode']!r}"
            return probe
        page = server.request("thread/turns/list", {"threadId": fork_id, "cursor": None, "limit": 10,
                                                    "sortDirection": "desc", "itemsView": "summary"})
        turns = page.get("data", [])
        probe["turnsBefore"] = len(turns)
        newest = turns[0]["id"]  # sortDirection desc: the newest turn is the revert boundary
        server.request("thread/revert", {"threadId": fork_id, "beforeTurnId": newest})
        after = server.request("thread/turns/list", {"threadId": fork_id, "cursor": None, "limit": 10,
                                                     "sortDirection": "desc", "itemsView": "summary"})
        probe["turnsAfterRevert"] = len(after.get("data", []))
    except Exception as error:
        probe["error"] = str(error)
        if "unknown variant" in str(error) or "not found" in str(error).lower() and "method" in str(error).lower():
            probe["unsupported"] = str(error)
    finally:
        server.close()
    return probe


@scenario(8, "Stop all: parent delegating a slow Claude child; queued message stays queued")
def stop_all(engine, checks, engine_evidence):
    evidence = engine_evidence
    parent = {"codex": LUNA, "codex-gemini": GEMINI, "claude": CLAUDE, "pi": PI}[STOP_PARENT]
    stop_all_with(engine, checks, engine_evidence, parent)


@scenario(16, "Stop all with a Pi parent delegating a slow Claude child; queued message stays queued")
def stop_all_pi(engine, checks, evidence):
    stop_all_with(engine, checks, evidence, PI)


def stop_all_with(engine, checks, evidence, parent):
    effort = "medium" if parent["harness"] == "codex" else None
    evidence["parent"] = parent
    engine.require_model(parent)
    engine.require_model(CLAUDE)
    chat = "s8"
    engine.create_chat(chat, parent, reasoning=effort, options=opt(effort) if effort else None)
    target = {"providerInstanceId": CLAUDE["instance"], "model": CLAUDE["model"]}
    prompt = ("Noches orchestration integration test. First call the injected t3-code delegate_task tool exactly once "
              f"with target {json.dumps(target)}, mode=\"async\", clientRequestId=\"live-stop-1\", "
              "task=\"Run `sleep 90` with your shell tool in the foreground (never in the background), then reply "
              "DONE.\". Do not wait for the child and do not poll. Immediately afterwards run `sleep 90` yourself "
              "with your shell tool in the foreground (never in the background). No other tools.")
    engine.run(chat, prompt, parent, reasoning=effort, options=opt(effort) if effort else None)

    def child_running():
        state = engine.state(chat)
        for task in state.get("tasks", []):
            child = task.get("childThreadId")
            if child and events_of(engine.journal(child), "toolCall"):
                return task
    task = None
    try:
        task = engine.wait_for(child_running, timeout=150, interval=0.5, what="child tool call")
    except RuntimeError:
        evidence["stateAtTimeout"] = engine.state(chat)
        raise
    engine.spend()  # the delegated child's turn
    evidence["taskBefore"] = task
    evidence["parentToolCalls"] = [e.get("call") for e in events_of(engine.journal(chat), "toolCall")]
    queued = engine.queue_message(chat, "Queued follow-up: reply with exactly: QUEUED", hold=True)
    queue_entry(engine, chat, queued)
    sleepers_before = [p for p in engine.descendants() if "sleep 90" in p[1]]
    evidence["sleepersBefore"] = sleepers_before
    result = engine.call("StopThreadWork", {"chatId": chat, "clientRequestId": "live-stop-all"})
    evidence["stopResult"] = result
    checks.ok("Stop all accepted without refusal", result.get("refusal") is None, result)
    checks.ok("Stop all admitted the parent run and the child run", result.get("stoppedRuns", 0) >= 2, result)
    time.sleep(8)
    state = engine.state(chat)
    evidence["stateAfter"] = state
    statuses = [t.get("status") for t in state.get("tasks", [])]
    checks.ok("delegated child task is no longer running", statuses and all(
        s in ("cancelled", "interrupted", "failed", "completed") for s in statuses), statuses)
    checks.ok("parent has no active run", not engine.queue_state(chat).get("activeRunId"), engine.queue_state(chat))
    after = engine.descendants()
    evidence["processesAfter"] = after
    checks.ok("no `sleep 90` processes survive the stop", not [p for p in after if "sleep 90" in p[1]], after)
    queue = engine.queue_state(chat)
    evidence["queueAfter"] = queue
    checks.ok("queued message stays queued (not dispatched)",
              [e["messageId"] for e in queue["queue"]] == [queued], queue["queue"])
    checks.ok("queued message was not delivered to the provider", queued not in [m["id"] for m in engine.transcript(chat)])


@scenario(9, "Reset agent session on Claude: fresh native session seeded with portable context")
def reset_session(engine, checks, evidence):
    engine.require_model(CLAUDE)
    chat = "s9"
    engine.create_chat(chat, CLAUDE)
    engine.run(chat, "Remember: the passphrase is MERIDIAN-77. Reply with exactly: OK", CLAUDE)
    engine.wait_idle(chat)
    first_session = events_of(engine.journal(chat), "sessionStarted")[0]["sessionId"]
    state = engine.transfer_state(chat)
    reply = engine.call("ResetThreadSession", {"chatId": chat, "clientRequestId": "live-reset-1",
                                               "observedRunId": state["latestStartedRunId"],
                                               "providerSessions": state["attachedProviderSessions"]})
    evidence["reset"] = reply
    checks.ok("reset accepted", reply.get("refusal") is None, reply)
    # The reset RPC returns once the receipt is committed; its disconnect effect
    # then closes the old Claude process. A person sends their next message
    # seconds later, so wait for the effect plus a short human pause.
    engine.wait_for(lambda: engine.sql("select 1 from orchestration_effect_outbox where "
                                       "effect_type='provider-session.disconnect' and status='succeeded'"),
                    timeout=30, what="disconnect effect")
    time.sleep(RESET_PAUSE_SECONDS)
    engine.run(chat, "What is the passphrase? Reply with exactly the passphrase.", CLAUDE)
    seen_args = []

    def sample():
        for _pid, command in engine.descendants():
            if "claude" in command and "--output-format" in command:
                seen_args.append(command)
        return engine.is_active(chat) is False

    deadline = time.monotonic() + 120
    while time.monotonic() < deadline and not sample():
        time.sleep(0.2)
    engine.wait_idle(chat)
    started = events_of(engine.journal(chat), "sessionStarted")
    second_session = started[-1]["sessionId"]
    evidence["sessions"] = [e["sessionId"] for e in started]
    evidence["claudeCommandLines"] = sorted(set(seen_args))[:3]
    checks.ok("next turn started a NEW native session", second_session != first_session, evidence["sessions"])
    checks.ok("fresh start: no --resume on the Claude command line",
              seen_args and not any("--resume" in a for a in seen_args), evidence["claudeCommandLines"])
    runs = [json.loads(r["payload_json"]) for r in engine.sql(
        "select payload_json from orchestration_projection_runs where thread_id=? order by ordinal", (chat,))]
    evidence["runs"] = [{"run": r["id"], "contextHandoffId": r.get("contextHandoffId")} for r in runs]
    checks.ok("fresh turn carries a portable context handoff", len(runs) == 2 and bool(runs[1].get("contextHandoffId")),
              evidence["runs"])
    seed_file = claude_session_file(engine.repo, second_session)
    seed_text = seed_file.read_text() if seed_file.exists() else ""
    checks.ok("the new native session was seeded with the earlier history (portable context in its prompt)",
              "MERIDIAN-77" in seed_text and "Context handoff" in seed_text, str(seed_file))
    answer = last_assistant_text(engine.transcript(chat))
    evidence["answer"] = answer
    checks.ok("answer is correct via portable context", "MERIDIAN-77" in answer, answer)


# -- Pi scenarios ------------------------------------------------------------
def pi_session_file(engine, chat):
    started = events_of(engine.journal(chat), "sessionStarted")
    return Path(started[0]["sessionId"]) if started else None


def sha_file(path):
    return sha(path) if path else None


def delegation_prompt(target, token, client_request_id):
    return ("This is a Noches orchestration integration test. Do not change files or run shell commands. "
            "First call the injected t3-code orchestrator_capabilities tool. Then call the injected t3-code "
            f"delegate_task tool exactly once with target {json.dumps(target)}, mode=\"async\", "
            f"clientRequestId=\"{client_request_id}\", task=\"Do not modify files or run commands. Reply with "
            f"exactly the word {token}.\". Retain taskId. Say WAITING_FOR_CHILD and finish this turn. When the "
            "app sends 'Delegated task completion available', call task_status with that taskId to "
            f"acknowledge its result. Then report exactly 'LIVE_PASS: result={token}' if the child summary is "
            f"{token}. Do not use native subagents, delegate more tasks, poll before the wake, or create threads.")


def delegate_and_wake(engine, checks, evidence, parent, child, token, chat):
    engine.require_model(parent)
    engine.require_model(child)
    effort = "medium" if parent["harness"] == "codex" else None
    engine.create_chat(chat, parent, reasoning=effort, options=opt(effort) if effort else None)
    target = {"providerInstanceId": child["instance"], "model": child["model"]}
    engine.run(chat, delegation_prompt(target, token, f"live-{chat}"), parent, reasoning=effort,
               options=opt(effort) if effort else None)
    engine.spend(2)  # the delegated child's turn and the parent's wake continuation
    final = {}

    def settled():
        state = engine.state(chat)
        final["state"] = state
        tasks = state.get("tasks", [])
        if tasks and tasks[0]["status"] in ("failed", "interrupted", "cancelled"):
            raise RuntimeError("child terminal failure: " + json.dumps(tasks[0]))
        return bool(tasks) and tasks[0]["status"] == "completed" and \
            tasks[0].get("completionDelivery", {}).get("state") == "acknowledged" and \
            "LIVE_PASS" in (state.get("latestResult") or "")
    try:
        engine.wait_for(settled, timeout=300, interval=1.0, what="child completion, wake and parent report")
    finally:
        evidence["state"] = final.get("state")
    engine.wait_idle(chat, timeout=120)
    state = engine.state(chat)
    task = state["tasks"][0]
    evidence["task"] = task
    evidence["parentTranscript"] = [message_text(m) for m in engine.transcript(chat) if m["role"] == "assistant"]
    child_thread = task["childThreadId"]
    child_started = events_of(engine.journal(child_thread), "sessionStarted")
    evidence["childSessions"] = [{k: e.get(k) for k in ("harness", "model", "sessionId")} for e in child_started]
    checks.ok("child ran on the requested provider instance and model",
              task["providerInstanceId"] == child["instance"] and task["model"] == child["model"], task)
    checks.ok("child completed with its result", token in (task.get("result") or ""), task.get("result"))
    checks.ok("child process is the requested harness",
              bool(child_started) and child_started[0].get("harness") == child["harness"], evidence["childSessions"])
    checks.ok("parent received the wake and acknowledged it",
              task["completionDelivery"]["state"] == "acknowledged" and bool(task["completionDelivery"].get("observedByRunId")),
              task["completionDelivery"])
    checks.ok("parent reported the child's result after the wake", f"LIVE_PASS: result={token}" in (state.get("latestResult") or ""),
              state.get("latestResult"))
    calls = [e.get("call") for e in events_of(engine.journal(chat), "toolCall")]
    evidence["parentTools"] = [str(c) for c in calls]
    delegations = [c for c in calls if isinstance(c, dict) and c.get("tool") == "delegate_task"]
    checks.ok("parent delegated exactly once", len(delegations) == 1, evidence["parentTools"])


@scenario(10, "Pi parent delegates to a Haiku child and receives the wake")
def pi_parent_haiku_child(engine, checks, evidence):
    delegate_and_wake(engine, checks, evidence, PI, CLAUDE, "PONGHAIKU", "s10")


@scenario(11, "Haiku parent delegates to a Pi child and receives the wake")
def haiku_parent_pi_child(engine, checks, evidence):
    delegate_and_wake(engine, checks, evidence, CLAUDE, PI, "PONGPI", "s11")


@scenario(12, "Haiku -> Pi -> Haiku: Pi gets portable context, Haiku resumes natively with the delta")
def haiku_pi_haiku(engine, checks, evidence):
    engine.require_model(CLAUDE)
    engine.require_model(PI)
    chat = "s12"
    engine.create_chat(chat, CLAUDE)
    engine.run(chat, "Remember: the project name is AURORA. Reply with exactly: OK-A1", CLAUDE)
    engine.wait_idle(chat)
    engine.set_config(chat, PI)
    engine.run(chat, "Earlier in this conversation you were given a project name. State it. "
                     "Also remember: the build number is 4242. Reply as `name=<project name>; ok`.", PI)
    engine.wait_idle(chat, timeout=240)
    engine.set_config(chat, CLAUDE)
    engine.run(chat, "Without tools, what is the project name and what is the build number? "
                     "Reply as `name=<n>; build=<b>`.", CLAUDE)
    engine.wait_idle(chat)
    answers = [message_text(m).strip() for m in engine.transcript(chat)
               if m["role"] == "assistant" and message_text(m).strip()]
    evidence["answers"] = answers
    state = engine.transfer_state(chat)
    evidence["handoffs"] = state["handoffs"]
    evidence["transfers"] = state["transfers"]
    runs = [json.loads(r["payload_json"]) for r in engine.sql(
        "select payload_json from orchestration_projection_runs where thread_id=? order by ordinal", (chat,))]
    evidence["runs"] = [{"run": r["id"], "instance": r["providerInstanceId"],
                         "contextHandoffId": r.get("contextHandoffId")} for r in runs]
    started = events_of(engine.journal(chat), "sessionStarted")
    evidence["sessionStarted"] = [{k: e.get(k) for k in ("instanceId", "model", "sessionId")} for e in started]
    claude_sessions = [e["sessionId"] for e in started if e["instanceId"] == "claudeAgent"]
    pi_sessions = [e["sessionId"] for e in started if e["instanceId"] == "pi"]
    checks.ok("Pi answered with Haiku's project name via portable context",
              len(answers) >= 2 and "AURORA" in answers[1].upper(), answers)
    checks.ok("Pi's native id is a session file under Pi's own store",
              len(pi_sessions) == 1 and Path(pi_sessions[0]).is_file() and pi_sessions[0].endswith(".jsonl"), pi_sessions)
    checks.ok("Haiku resumed its OWN native session (same Claude session id on turn 3)",
              len(claude_sessions) == 2 and claude_sessions[0] == claude_sessions[1], evidence["sessionStarted"])
    checks.ok("Haiku recalls the name (native) and the build number told only to Pi (delta handoff)",
              bool(answers) and "AURORA" in answers[-1].upper() and "4242" in answers[-1], answers[-1:])
    with_handoff = [r for r in evidence["runs"] if r["contextHandoffId"]]
    checks.ok("both provider switches carried a handoff (Pi portable, Haiku delta)",
              len(with_handoff) == 2 and with_handoff[0]["instance"] == "pi"
              and with_handoff[1]["instance"] == "claudeAgent", evidence["runs"])
    strategies = [h.get("strategy") for h in state["handoffs"]]
    checks.ok("Pi received the full summary, Haiku only the delta",
              strategies == ["full_thread_summary", "delta_since_target_last_seen"], strategies)


@scenario(13, "Pi lazy native fork: turn-1 checkpoint vs head through ForkThread")
def pi_lazy_fork(engine, checks, evidence):
    engine.require_model(PI)
    parent = "s13p"
    engine.create_chat(parent, PI)
    engine.run(parent, "Remember: X=1. Reply with exactly: OK1", PI)
    engine.wait_idle(parent, timeout=240)
    engine.run(parent, "Remember: Y=2. Reply with exactly: OK2", PI)
    engine.wait_idle(parent, timeout=240)
    parent_file = pi_session_file(engine, parent)
    before = sha_file(parent_file)
    evidence["parentSession"] = str(parent_file)
    checks.ok("parent session file exists", bool(parent_file) and parent_file.is_file(), str(parent_file))
    turn_refs = [e.get("turnId") for e in events_of(engine.journal(parent), "nativeReference")]
    evidence["turnRefs"] = turn_refs
    checks.ok("each parent turn recorded a native user-entry ref", len(turn_refs) == 2 and all(turn_refs), turn_refs)
    state = engine.transfer_state(parent)
    turn1 = next(c["checkpoint"]["id"] for c in state["checkpoints"]
                 if c["checkpoint"]["runId"].endswith(":1") and c["phase"] == "completed")
    ask = ("What do you remember from this conversation? Answer in one line exactly as "
           "`X=<value or unknown>; Y=<value or unknown>`. Use `unknown` for anything you were not told.")
    for name, point in (("turn1", {"type": "checkpoint", "checkpointId": turn1}), ("head", {"type": "latest_stable"})):
        child = f"s13-{name}"
        fork_thread(engine, parent, child, point, f"pi-{name}")
        checks.ok(f"{name}: forking starts no provider process yet", not engine.journal(child), len(engine.journal(child)))
        engine.run(child, ask, PI)
        engine.wait_idle(child, timeout=240)
        answer = last_assistant_text(engine.transcript(child))
        child_file = pi_session_file(engine, child)
        transfer = (engine.transfer_state(child).get("transfers") or [{}])[0]
        evidence[f"{name}Answer"] = answer
        evidence[f"{name}Session"] = str(child_file)
        evidence[f"{name}Transfer"] = transfer
        text = child_file.read_text() if child_file and child_file.is_file() else ""
        checks.ok(f"{name}: child has its own Pi session file", bool(child_file) and child_file != parent_file
                  and child_file.is_file(), str(child_file))
        checks.ok(f"{name}: delivered by a native Pi fork, not portable context",
                  (transfer.get("resolution") or {}).get("strategy") == "native_fork"
                  and "Context handoff" not in text, transfer.get("resolution"))
    t1, head = evidence["turn1Answer"].replace(" ", ""), evidence["headAnswer"].replace(" ", "")
    checks.ok("turn-1 fork knows X only", "X=1" in t1 and "Y=2" not in t1, evidence["turn1Answer"])
    checks.ok("head fork knows X and Y", "X=1" in head and "Y=2" in head, evidence["headAnswer"])
    checks.ok("parent session file unchanged by forking", sha_file(parent_file) == before, parent_file.name)
    checks.ok("children are distinct native sessions", evidence["turn1Session"] != evidence["headSession"])


def pi_models_used(path):
    """The model each assistant message in a Pi session file was produced by."""
    models = []
    for line in (path.read_text().splitlines() if path and path.is_file() else []):
        try:
            entry = json.loads(line)
        except ValueError:
            continue
        message = entry.get("message") or {}
        if entry.get("type") == "message" and message.get("role") == "assistant" and message.get("model"):
            models.append(f"{message.get('provider')}/{message['model']}")
    return models


@scenario(14, "Pi queue promotion with a model change: steer now, new model on the next turn of the same native session")
def pi_queue_promotion(engine, checks, evidence):
    # Pi cannot be steered by interrupt/restart (T3's PiAdapterV2 says
    # supportsSteeringByInterruptRestart=false), so with a composer model change
    # the promotion is a DEFERRED active steer: the running turn takes the
    # message on its current model and the selection applies to the next turn
    # (selection_transition = ApplyOnNextTurn, via RPC set_model on the resumed
    # session). Nothing is interrupted and nothing is handed off.
    engine.require_model(PI)
    engine.require_model(PI_LUNA)
    chat = "s14"
    engine.create_chat(chat, PI)
    engine.run(chat, "Run `sleep 20` with your shell tool, then reply with exactly: DONE1", PI)
    engine.wait_for(lambda: events_of(engine.journal(chat), "toolCall"), timeout=120, what="long tool call")
    first = engine.queue_message(chat, "Follow-up one: also append the word F1 to your reply.")
    second = engine.queue_message(chat, "Follow-up two: reply with exactly: F2")
    engine.set_config(chat, PI_LUNA)

    def deferred_ready():
        state = engine.queue_state(chat)
        if state.get("promotionSelectionDeferred") and len(state.get("queue", [])) == 2:
            return state
    queue = engine.wait_for(deferred_ready, timeout=20, what="deferred promotion hint")
    evidence["queueBefore"] = queue
    selection = queue.get("promotionSelection") or {}
    checks.ok("a model change on the same Pi instance promotes as a deferred active steer",
              queue.get("promotionMode") == "active_steering" and queue.get("promotionSelectionDeferred")
              and selection.get("model") == PI_LUNA["model"], {"mode": queue.get("promotionMode"),
                                                              "selection": selection})
    entry = next(e for e in queue["queue"] if e["messageId"] == first)
    reply = engine.call("MutateQueuedRun", {
        "chatId": chat, "queuedRunId": entry["queuedRunId"], "clientRequestId": "live-pi-promote-deferred",
        "action": {"type": "promoteToSteer", "targetRunId": queue["activeRunId"],
                   "expectedSelection": queue.get("promotionSelection")}})
    evidence["promotion"] = reply
    checks.ok("promotion accepted", reply.get("refusal") is None, reply)
    engine.spend(2)  # the steered reply is part of turn 1; F2 is the second provider turn
    engine.wait_idle(chat, timeout=300, settle=6)
    started = events_of(engine.journal(chat), "sessionStarted")
    dones = events_of(engine.journal(chat), "done")
    evidence["sessionStarted"] = [{k: e.get(k) for k in ("model", "sessionId")} for e in started]
    evidence["dones"] = [{k: e.get(k) for k in ("status", "sessionId")} for e in dones]
    sessions = {e["sessionId"] for e in started}
    runs = engine.sql("select run_id, status from orchestration_projection_runs where thread_id=? order by ordinal",
                      (chat,))
    evidence["runs"] = runs
    # The promoted queue row is the cancelled middle run (its message rode the
    # steer, as for Claude in scenario 1). The warm Pi process is retired idle
    # when the next turn needs a different model, which journals one trailing
    # `interrupted` done by design; the turns themselves must complete.
    checks.ok("the turn was steered, not interrupted: turn 1 and the next turn completed",
              [r["status"] for r in runs] == ["completed", "cancelled", "completed"]
              and dones[0]["status"] == "completed" and dones[-1]["status"] == "completed",
              {"runs": runs, "dones": evidence["dones"]})
    checks.ok("the next turn ran on the SAME native Pi session with the new model",
              len(sessions) == 1 and len(started) == 2 and started[0]["model"] == PI["model"]
              and started[1]["model"] == PI_LUNA["model"], evidence["sessionStarted"])
    transcript = engine.transcript(chat)
    texts = [t for t in user_texts(transcript) if t.startswith("Follow-up")]
    answers = [message_text(m).strip() for m in transcript if m["role"] == "assistant"]
    evidence["userMessages"], evidence["assistantTexts"] = texts, answers
    checks.ok("each follow-up delivered exactly once, in order",
              texts == ["Follow-up one: also append the word F1 to your reply.",
                        "Follow-up two: reply with exactly: F2"], texts)
    checks.ok("steer honored in turn 1 and F2 answered in turn 2",
              any("F1" in a for a in answers) and "F2" in answers, answers)
    delivery = steer_delivery(engine, chat, first)
    evidence["delivery"] = delivery
    checks.ok("steer delivered natively once (receipt + acceptance row)",
              len(delivery["inputAcceptedFor"]) == 1 and len(delivery["acceptances"]) == 1
              and delivery["transcriptCount"] == 1, delivery)
    state = engine.transfer_state(chat)
    checks.ok("no portable handoff for a same-session model change", not state["handoffs"] and not state["transfers"],
              {"handoffs": state["handoffs"], "transfers": state["transfers"]})
    models = pi_models_used(Path(next(iter(sessions))))
    evidence["sessionFileModels"] = models
    checks.ok("Pi's own session file shows gemini for turn 1 (steer included) and luna for turn 2",
              len(models) >= 2 and models[0] == PI["model"] and models[-1] == PI_LUNA["model"]
              and PI_LUNA["model"] not in models[:-1], models)
    checks.ok("queue drained", not engine.queue_state(chat)["queue"])
    checks.ok("no leftover `sleep 20`", not [p for p in engine.descendants() if "sleep 20" in p[1]])


@scenario(15, "Pi steer receipts: mid-tool and right after the last tool result, each exactly once")
def pi_receipts(engine, checks, evidence):
    engine.require_model(PI)
    chat = "s15"
    engine.create_chat(chat, PI)
    engine.run(chat, "Run `sleep 10` with your shell tool, then reply with exactly: ALPHA", PI)
    engine.wait_for(lambda: events_of(engine.journal(chat), "toolCall"), timeout=120, what="first tool call")
    token_a = "STEER-A-7731"
    id_a, reply_a = steer_once(engine, chat, f"{token_a}: when you finish, also append the word BRAVO.", "pi-a", evidence)
    checks.ok("A promotion accepted", reply_a.get("refusal") is None, reply_a)
    engine.wait_idle(chat, timeout=240, settle=8)
    id_b = token_b = None
    for attempt in range(2):
        seen = len(events_of(engine.journal(chat), "toolResult"))
        engine.run(chat, "Run `ls` with your shell tool, then reply with exactly: GAMMA", PI)
        engine.wait_for(lambda: len(events_of(engine.journal(chat), "toolResult")) > seen, timeout=90,
                        interval=0.03, what="tool result")
        if not engine.queue_state(chat).get("activeRunId"):
            engine.wait_idle(chat, settle=3)
            evidence["bLostRace"] = evidence.get("bLostRace", 0) + 1
            continue
        token_b = "STEER-B-4492"
        id_b, reply_b = steer_once(engine, chat, f"{token_b}: also append the word DELTA.", "pi-b", evidence)
        break
    if not checks.ok("B: steer landed while a run was active", id_b is not None, evidence.get("bLostRace")):
        return
    engine.wait_idle(chat, timeout=240, settle=8)
    transcript = engine.transcript(chat)
    evidence["delivery"] = {"a": steer_delivery(engine, chat, id_a), "b": steer_delivery(engine, chat, id_b)}
    evidence["userMessages"] = user_texts(transcript)
    evidence["assistantTexts"] = [message_text(m) for m in transcript if m["role"] == "assistant"]
    evidence["effects"] = engine.sql("select effect_type, status, count(*) n from orchestration_effect_outbox "
                                     "where thread_id=? group by 1,2", (chat,))
    evidence["runs"] = engine.sql("select run_id, status from orchestration_projection_runs where thread_id=? "
                                  "order by ordinal", (chat,))
    for tag, token in (("A", token_a), ("B", token_b)):
        delivery = evidence["delivery"][tag.lower()]
        checks.ok(f"{tag}: exactly one user message carries the steer",
                  sum(token in t for t in user_texts(transcript)) == 1, user_texts(transcript))
        checks.ok(f"{tag}: transcript has the message id once", delivery["transcriptCount"] == 1, delivery)
        checks.ok(f"{tag}: native receipt recorded once", len(delivery["inputAcceptedFor"]) == 1, delivery)
        checks.ok(f"{tag}: steering acceptance row recorded once", len(delivery["acceptances"]) == 1, delivery)
    steers = sum(e["n"] for e in evidence["effects"] if e["effect_type"] == "provider-turn.steer")
    checks.ok("two steer effects, all succeeded", steers == 2 and not [
        e for e in evidence["effects"] if e["effect_type"] == "provider-turn.steer" and e["status"] != "succeeded"],
        evidence["effects"])
    redispatch = engine.sql("select effect_type, count(*) n from orchestration_effect_outbox where thread_id=? "
                            "and effect_type != 'provider-turn.steer' and (payload_json like ? or payload_json like ?) "
                            "group by 1", (chat, f"%{token_a}%", f"%{token_b}%"))
    checks.ok("no orphan redispatch of either steer text", not redispatch, redispatch)
    bad = [e for e in evidence["effects"] if e["status"] in ("failed", "uncertain", "pending", "running")]
    checks.ok("no failed/uncertain/pending effects", not bad, bad)
    checks.ok("no failed runs", not [r for r in evidence["runs"] if r["status"] == "failed"], evidence["runs"])
    final = "\n".join(evidence["assistantTexts"])
    checks.ok("A steer honored (BRAVO)", "BRAVO" in final, evidence["assistantTexts"])
    checks.ok("B steer honored (DELTA)", "DELTA" in final, evidence["assistantTexts"])


def main():
    global RESET_PAUSE_SECONDS, STOP_PARENT
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--binary", required=True)
    parser.add_argument("--only", help="comma separated scenario numbers")
    parser.add_argument("--max-turns", type=int, default=60)
    parser.add_argument("--scratch", help="scratch directory (default: a fresh mkdtemp)")
    parser.add_argument("--stop-parent", choices=["claude", "codex", "codex-gemini", "pi"], default=STOP_PARENT,
                        help="provider of the scenario-8 parent (codex = gpt-6-luna, codex-gemini = "
                             "gemini-3.8-flash on Codex, pi = Pi on gemini-3.8-flash)")
    parser.add_argument("--reset-pause", type=float, default=RESET_PAUSE_SECONDS,
                        help="seconds to wait after the reset's disconnect effect before the next turn (scenario 9)")
    args = parser.parse_args()
    RESET_PAUSE_SECONDS = args.reset_pause
    STOP_PARENT = args.stop_parent
    wanted = {int(n) for n in args.only.split(",")} if args.only else None
    scratch = Path(args.scratch) if args.scratch else Path(tempfile.mkdtemp(prefix="noches-orch-scenarios-"))
    scratch.mkdir(parents=True, exist_ok=True)
    os.chmod(scratch, 0o700)
    (scratch / "evidence").mkdir(exist_ok=True)
    print(f"SCRATCH: {scratch}", flush=True)
    results = []
    failures = 0
    for number, title, function in SCENARIOS:
        if wanted and number not in wanted:
            continue
        # One engine per scenario: no cross-scenario state, a crash is contained.
        run_dir = scratch / f"s{number}"
        run_dir.mkdir(exist_ok=True)
        engine = Engine(args.binary, run_dir, args.max_turns - sum(r.get("turns", 0) for r in results))
        checks = Check()
        evidence = {"scenario": number, "title": title}
        status = "FAIL"
        started = time.monotonic()
        try:
            engine.start()
            function(engine, checks, evidence)
            status = "PASS" if not checks.failed else "FAIL"
        except Skip as skip:
            status, evidence["reason"] = "SKIP", str(skip)
        except Unsupported as unsupported:
            status, evidence["reason"] = "UNSUPPORTED", str(unsupported)
        except Exception as error:  # a driver or product failure is a FAIL with the trace
            import traceback
            status, evidence["error"] = "FAIL", f"{type(error).__name__}: {error}"
            evidence["trace"] = traceback.format_exc()
        finally:
            evidence["checks"] = checks.items
            evidence["status"] = status
            evidence["turns"] = engine.turns
            engine.stop()
        evidence["seconds"] = round(time.monotonic() - started, 1)
        (scratch / "evidence" / f"{number}.json").write_text(json.dumps(evidence, indent=2, default=str))
        failures += status == "FAIL"
        results.append({"scenario": number, "status": status, "turns": engine.turns})
        detail = "; ".join(f"{c['check']}" for c in checks.failed) or evidence.get("error") or evidence.get("reason") or ""
        print(f"{status}: [{number}] {title} ({engine.turns} turns, {evidence['seconds']}s) {detail}", flush=True)
    (scratch / "evidence" / "summary.json").write_text(json.dumps(results, indent=2))
    print(f"TOTAL provider turns: {sum(r['turns'] for r in results)}; evidence: {scratch / 'evidence'}", flush=True)
    raise SystemExit(1 if failures else 0)


if __name__ == "__main__":
    main()
