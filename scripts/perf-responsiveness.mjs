#!/usr/bin/env node
// Run immutable, already-built optimized-test benchmarks in matched order.
// Usage: node scripts/perf-responsiveness.mjs BASE_ENGINE BASE_UI NEW_ENGINE NEW_UI NEW_OUTPUT_DIR
import { spawn, execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, createReadStream, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const args = process.argv.slice(2);
if (args.length !== 5) {
  throw Error('Usage: perf-responsiveness.mjs BASE_ENGINE BASE_UI NEW_ENGINE NEW_UI NEW_OUTPUT_DIR');
}
if (competingBuild()) throw Error('Wait for builds/tests to finish before benchmarking');
const output = resolve(args[4]);
if (existsSync(output)) throw Error('Use a new output directory');
mkdirSync(output, { recursive: true });
const artifacts = {};
for (const [index, name] of ['baseline-engine', 'baseline-ui', 'candidate-engine', 'candidate-ui'].entries()) {
  const path = `${output}/${name}`;
  copyFileSync(resolve(args[index]), path);
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  artifacts[name] = { path, sha256: hash.digest('hex') };
}
function competingBuild() {
  return execFileSync('ps', ['-axo', 'comm='], { encoding: 'utf8' })
    .split('\n')
    .some(command => /(?:^|\/)(?:cargo|rustc|clang(?:\+\+)?|ld|lld)(?:$|\s)/.test(command.trim()));
}
const order = ['baseline', 'candidate', 'candidate', 'baseline', 'baseline', 'candidate', 'candidate', 'baseline'];
const testArgs = ['profile_', '--ignored', '--nocapture', '--test-threads=1'];
const runs = [];
let active;
process.once('exit', () => { if (active) active.kill('SIGTERM'); });
process.once('SIGINT', () => process.exit(130));
process.once('SIGTERM', () => process.exit(143));
for (const variant of order) {
  if (competingBuild()) throw Error('Competing build/test detected; discard this comparison and rerun');
  let contaminated = false;
  const probe = setInterval(() => { contaminated ||= competingBuild(); }, 250);
  let text = '';
  try {
    for (const crate of ['engine', 'ui']) {
      let crateOutput = '';
      active = spawn(artifacts[`${variant}-${crate}`].path, testArgs);
      active.stdout.on('data', chunk => { crateOutput += chunk; });
      active.stderr.on('data', chunk => { crateOutput += chunk; });
      const exit = await new Promise((resolve, reject) => {
        active.once('exit', resolve);
        active.once('error', reject);
      });
      active = undefined;
      if (exit !== 0) throw Error(`Benchmark ${variant}/${crate} failed`);
      const requiredMarkers = crate === 'engine'
        ? ['outbox_profile history=50000 empty_claims=1000']
        : ['stream_projection_profile agents=false', 'stream_projection_profile agents=true'];
      if (requiredMarkers.some(marker => !crateOutput.includes(marker))) {
        throw Error(`Missing benchmark fixture in ${variant}/${crate}; an empty test run is not a result`);
      }
      text += crateOutput;
    }
    if (contaminated || competingBuild()) throw Error('Competing build/test detected; discard this comparison and rerun');
    runs.push({ variant, output: text });
    console.log(text);
  } finally {
    clearInterval(probe);
  }
}
writeFileSync(`${output}/measurements.json`, JSON.stringify({
  platform: process.platform, arch: process.arch, node: process.version,
  mode: 'optimized-test-microbenchmarks', artifacts, order, testArgs, runs,
}, null, 2));
