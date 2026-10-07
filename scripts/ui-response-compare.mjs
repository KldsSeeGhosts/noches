#!/usr/bin/env node
// Run already-built native fixtures sequentially, rejecting build contention.
// No OS input automation; the fixture dispatches production GPUI list events.
import { spawn, execFileSync } from "node:child_process";
import { mkdirSync, createWriteStream, writeFileSync, readFileSync } from "node:fs";
import { resolve, join } from "node:path";

const [baseline, after, destination, ...turnArgs] = process.argv.slice(2);
if (!destination) throw new Error(
  "Usage: node scripts/ui-response-compare.mjs BASELINE_BINARY AFTER_BINARY OUTPUT_DIR [TURNS ...]"
);
const builds = () => execFileSync("ps", ["-axo", "pid=,comm="], {encoding:"utf8"})
  .split("\n").filter(line => /\/rustc$/.test(line.trim()));
const output = resolve(destination);
mkdirSync(output, {recursive:true});
const turns = turnArgs.length ? turnArgs.map(Number) : [600, 1800];
const runs = Number(process.env.NOCHES_RESPONSE_RUNS ?? 3);
if (!turns.every(n => Number.isInteger(n) && n > 0) || !Number.isInteger(runs) || runs < 1)
  throw new Error("Turn counts and run count must be positive integers");

for (const count of turns) {
  for (let run = 1; run <= runs; run++) {
    for (const [label, executable] of [["baseline",baseline], ["after",after]]) {
      if (builds().length) throw new Error("A Rust compiler is active; retry after builds finish");
      const name = `${label}-${count}-${run}`;
      const directory = join(output, name);
      mkdirSync(directory, {recursive:true});
      const log = createWriteStream(join(output, `${name}.log`));
      const env = {...process.env, NOCHES_PERF_TRACE:"1", NOCHES_RESPONSE_TURNS:String(count),
        NOCHES_RESPONSE_PANEL_MS:"200"};
      // Interactive/verification readbacks are deliberately excluded from
      // matched timings. Run NOCHES_RESPONSE_VERIFY separately afterwards.
      delete env.NOCHES_RESPONSE_INTERACTIVE;
      delete env.NOCHES_RESPONSE_VERIFY;
      delete env.NOCHES_RESPONSE_REDUCED;
      delete env.NOCHES_RESPONSE_LIGHT;
      const child = spawn(resolve(executable), [directory], {env, stdio:["ignore","pipe","pipe"]});
      child.stdout.pipe(log, {end:false});
      child.stderr.pipe(log, {end:false});
      const checks = [];
      const check = () => checks.push({at:new Date().toISOString(), compilers:builds()});
      check();
      const interval = setInterval(check, 1000);
      const code = await new Promise((resolve, reject) => {
        child.once("error", reject);
        child.once("close", resolve);
      }).finally(() => { clearInterval(interval); });
      await new Promise(resolve => log.end(resolve));
      check();
      const contaminated = checks.some(c => c.compilers.length);
      const runtimeErrors = /\bERROR\b|native fixture FAILED/.test(
        readFileSync(join(output, `${name}.log`), "utf8")
      );
      writeFileSync(join(directory, "build-contention.json"),
        JSON.stringify({code, contaminated, runtimeErrors, checks}, null, 2));
      if (code !== 0) throw new Error(`${name} failed (${code}); see its log`);
      if (runtimeErrors) throw new Error(`${name} logged a runtime error; see its log`);
      if (contaminated) throw new Error(`${name} overlapped a compiler; exclude this run and retry`);
      console.log(`${name}: clean run -> ${directory}/response.json`);
    }
  }
}
