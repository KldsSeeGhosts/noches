#!/usr/bin/env node
// Isolated headless-engine idle benchmark. No UI or model turn is started.
// Usage: node scripts/perf-orchestration-idle.mjs BINARY NEW_OUTPUT_DIR [ROWS=50000] [SECONDS=15]
// Requires Node >=22, sqlite3, and Xcode CLI tools on macOS. CPU: 100% per core.
import { spawn, execFileSync } from 'node:child_process';
import { createServer } from 'node:net';
import { createHash } from 'node:crypto';
import { mkdirSync, existsSync, copyFileSync, openSync, readFileSync, writeFileSync, createReadStream } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { setTimeout as sleep } from 'node:timers/promises';

const [binaryArg, outputArg, rowsArg = '50000', secondsArg = '15'] = process.argv.slice(2);
if (!binaryArg || !outputArg) throw Error('Usage: perf-orchestration-idle.mjs BINARY NEW_OUTPUT_DIR [ROWS] [SECONDS]');
const rows = Number(rowsArg), seconds = Number(secondsArg);
if (!Number.isSafeInteger(rows) || rows < 0 || rows > 1_000_000) throw Error('ROWS must be 0..1000000');
if (!Number.isFinite(seconds) || seconds < 5 || seconds > 120) throw Error('SECONDS must be 5..120');
const output = resolve(outputArg), binary = resolve(binaryArg);
if (existsSync(output)) throw Error('Use a new output directory; never reuse a user profile');
mkdirSync(output, { recursive: true });
const executable = `${output}/zeron-profiled`;
copyFileSync(binary, executable);
const hash = createHash('sha256');
for await (const chunk of createReadStream(executable)) hash.update(chunk);
const binarySha256 = hash.digest('hex');
const nativeStat = `${output}/macos-resource-stat`;
if (process.platform === 'darwin') {
  execFileSync('xcrun', ['clang', '-O2', '-Wall', '-Wextra',
    fileURLToPath(new URL('./macos-resource-stat.c', import.meta.url)), '-o', nativeStat]);
}
const hz = Number(execFileSync('getconf', ['CLK_TCK'], { encoding: 'utf8' }).trim());
function stat(pid) {
  if (process.platform === 'darwin') {
    const value = JSON.parse(execFileSync(nativeStat, [String(pid)], { encoding: 'utf8' }))[0];
    if (!value) throw Error('Engine process disappeared');
    return value;
  }
  if (process.platform !== 'linux') throw Error('This benchmark supports macOS and Linux');
  const fields = readFileSync(`/proc/${pid}/stat`, 'utf8').split(') ')[1].split(' ');
  const status = readFileSync(`/proc/${pid}/status`, 'utf8');
  return { pid, cpuSeconds: (Number(fields[11]) + Number(fields[12])) / hz,
    rssMiB: Number(status.match(/VmRSS:\s+(\d+)/)?.[1] ?? 0) / 1024 };
}
function assertNoCompetingBuild() {
  const commands = execFileSync('ps', ['-axo', 'comm='], { encoding: 'utf8' });
  if (commands.split('\n').some(command =>
    /(?:^|\/)(?:cargo|rustc|clang(?:\+\+)?|ld|lld)(?:$|\s)/.test(command.trim()))) {
    throw Error('Competing build/test process detected; discard timings and rerun when idle');
  }
}
const server = createServer();
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const port = server.address().port;
await new Promise(resolve => server.close(resolve));
const env = { ...process.env, ZERON_DATA_DIR: `${output}/profile`,
  ZERON_IPC_PORT: String(port), ZERON_HARNESS: 'mock', ZERON_ORCHESTRATION: '1', RUST_LOG: 'warn' };
delete env.ZERON_EDGE_TOKEN;
delete env.ZERON_ORG_ID;
delete env.ZERON_ADAPTERS_DIR;
const log = openSync(`${output}/engine.log`, 'w');
const engine = spawn(executable, ['headless'], {
  env, detached: true, stdio: ['ignore', log, log],
});
function stopEngine() {
  try { process.kill(-engine.pid, 'SIGTERM'); } catch { /* already exited */ }
}
process.once('exit', stopEngine);
process.once('SIGINT', () => process.exit(130));
process.once('SIGTERM', () => process.exit(143));
let ws;
try {
  for (let attempt = 0; attempt < 120; attempt++) {
    if (engine.exitCode != null) throw Error('Engine exited during startup; inspect engine.log');
    try {
      ws = new WebSocket(`ws://127.0.0.1:${port}`);
      await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
      break;
    } catch {
      ws?.close();
      await sleep(250);
    }
  }
  if (ws?.readyState !== WebSocket.OPEN) throw Error('Engine startup timed out');
  await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(Error('EngineReady timed out')), 30000);
    ws.onmessage = ({ data }) => {
      const frame = JSON.parse(data);
      if (frame.id !== 1) return;
      if ('ok' in frame) { clearTimeout(timer); resolve(); }
      if ('err' in frame) { clearTimeout(timer); reject(Error(JSON.stringify(frame.err))); }
    };
    ws.send(JSON.stringify({ id: 1, method: 'EngineReady', params: {} }));
  });
  ws.close();
  const database = `${output}/profile/profiles/local/docs.sqlite3`;
  if (!existsSync(database)) throw Error('Expected the isolated local profile');
  // Terminal payloads are never decoded or dispatched. This adds retained
  // outbox history only, not runnable work or synthetic provider activity.
  if (rows > 0) {
    execFileSync('sqlite3', ['-batch', '-bail', '-cmd', '.timeout 5000', database], { input: `
      BEGIN IMMEDIATE;
      WITH RECURSIVE ids(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM ids WHERE n<${rows})
      INSERT INTO orchestration_effect_outbox
        (effect_id,command_id,thread_id,effect_type,lane,payload_json,process_bound,status,available_at,created_at)
        SELECT 'retained-'||n,'retained-'||n,'history','terminal.cleanup','provider','{}',0,'succeeded',0,0 FROM ids;
      WITH RECURSIVE ids(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM ids WHERE n<${rows})
      INSERT INTO orchestration_publication_batches
        (batch_id,host_id,host_epoch,through_sequence,status,payload_json)
        SELECT 'retained-'||n,'history',1,0,'published','{}' FROM ids;
      COMMIT;
    ` });
  }
  await sleep(10000); // exclude setup, migration, initial discovery and seeding
  const samples = [];
  const started = performance.now();
  do {
    if (engine.exitCode != null) throw Error('Engine exited during measurement');
    assertNoCompetingBuild();
    samples.push({ atMs: performance.now(), ...stat(engine.pid) });
    await sleep(500);
  } while (performance.now() - started <= seconds * 1000);
  assertNoCompetingBuild();
  samples.push({ atMs: performance.now(), ...stat(engine.pid) });
  const first = samples[0], last = samples.at(-1);
  const summary = {
    binarySha256, platform: process.platform, arch: process.arch,
    mode: 'headless-idle-terminal-outbox-history', rowsPerOutbox: rows,
    durationSeconds: (last.atMs - first.atMs) / 1000,
    cpuPercent: 100000 * (last.cpuSeconds - first.cpuSeconds) / (last.atMs - first.atMs),
    peakRssMiB: Math.max(...samples.map(s => s.rssMiB)),
    ...(process.platform === 'darwin' ? {
      peakFootprintMiB: Math.max(...samples.map(s => s.footprintMiB)),
      idleWakeups: last.idleWakeups - first.idleWakeups,
    } : {}),
  };
  writeFileSync(`${output}/samples.json`, JSON.stringify(samples, null, 2));
  writeFileSync(`${output}/summary.json`, JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary, null, 2));
} finally {
  ws?.close();
  stopEngine();
}
