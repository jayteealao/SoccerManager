// Starts the release engine for one test, with its own data folder, and reads what it wrote.
import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', '..');
export const CONTENT = path.join(REPO, 'content');
export const WEB = path.join(REPO, 'web');
export const BINARY = path.join(
  REPO,
  'target',
  'release',
  process.platform === 'win32' ? 'engine-cli.exe' : 'engine-cli'
);

function requireBinary() {
  if (!existsSync(BINARY)) {
    throw new Error(`The engine is not built: ${BINARY} is missing. Run "cargo build --release".`);
  }
}

/// A new, empty folder under the system temporary folder.
export function tempDir(prefix) {
  return mkdtempSync(path.join(tmpdir(), `touchline-${prefix}-`));
}

/// Ends a process and every process it started. The launcher runs the engine as a child, and
/// a child left running would hold its port and its data folder.
export function killTree(pid) {
  if (!pid) {
    return;
  }
  if (process.platform === 'win32') {
    spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
  } else {
    try {
      process.kill(pid, 'SIGKILL');
    } catch {
      // Already gone.
    }
  }
}

/// Runs `engine-cli <command> <args>` with a new data folder and waits for the page address
/// it prints. `serve` prints the socket port and then the address; `launch` and `replay
/// --web` print the address. Returns the address, the data folder, and the process.
export async function startEngine({ command = 'serve', args = [], env = {}, dataDir } = {}) {
  requireBinary();
  const data = dataDir ?? tempDir('data');
  const child = spawn(BINARY, [command, ...args], {
    cwd: REPO,
    env: { ...process.env, SM_DATA_DIR: data, SM_CONTENT_DIR: CONTENT, SM_ENGINE_PATH: '', ...env },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let stdout = '';
  let stderr = '';
  child.stderr.on('data', (d) => {
    stderr += d.toString();
  });
  const exited = new Promise((resolve) => child.on('exit', (code) => resolve(code)));
  const url = await new Promise((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`No page address within 10 s.\nstdout:\n${stdout}\nstderr:\n${stderr}`)),
      10_000
    );
    child.stdout.on('data', (d) => {
      stdout += d.toString();
      const found = stdout.match(/http:\/\/127\.0\.0\.1:\d+\/?/);
      if (found) {
        clearTimeout(timer);
        resolve(found[0].endsWith('/') ? found[0] : `${found[0]}/`);
      }
    });
    exited.then((code) => {
      clearTimeout(timer);
      reject(new Error(`The engine exited with ${code} before it printed an address.\n${stderr}`));
    });
  });
  return {
    url,
    dataDir: data,
    process: child,
    exited,
    log: () => stderr,
    kill: () => killTree(child.pid),
    /// Removes the data folder once the engine has stopped.
    cleanUp: () => {
      killTree(child.pid);
      try {
        rmSync(data, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
      } catch {
        // A file still held open by an exiting process; the system temporary folder keeps it.
      }
    },
  };
}

/// Runs one engine command to completion and returns its exit code and output.
export function runEngine(args, { env = {}, dataDir } = {}) {
  requireBinary();
  const result = spawnSync(BINARY, args, {
    cwd: REPO,
    env: { ...process.env, SM_CONTENT_DIR: CONTENT, ...(dataDir ? { SM_DATA_DIR: dataDir } : {}), ...env },
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  return { code: result.status, stdout: result.stdout, stderr: result.stderr };
}

/// Generates two clubs from `seed` and returns their team-file paths, home first.
export function generateLeague(seed) {
  const out = tempDir(`league-${seed}`);
  const run = runEngine(['generate', '--seed', String(seed), '--clubs', '2', '--out', out]);
  if (run.code !== 0) {
    throw new Error(`generate failed with ${run.code}: ${run.stderr}`);
  }
  const files = readdirSync(out)
    .filter((f) => f.endsWith('.json'))
    .sort()
    .map((f) => path.join(out, f));
  if (files.length !== 2) {
    throw new Error(`generate wrote ${files.length} team files, not 2, in ${out}`);
  }
  return files;
}

/// The match folders the engine wrote under a data folder.
export function matchIds(dataDir) {
  const root = path.join(dataDir, 'matches');
  return existsSync(root) ? readdirSync(root) : [];
}

/// The statistics record and the event rows of one match, parsed and untransformed. A file
/// not yet written reads as null.
export function readRecords(dataDir, matchId) {
  const folder = path.join(dataDir, 'matches', matchId);
  const statsPath = path.join(folder, 'stats.json');
  const eventsPath = path.join(folder, 'events.jsonl');
  const stats = existsSync(statsPath) ? JSON.parse(readFileSync(statsPath, 'utf8')) : null;
  const events = existsSync(eventsPath)
    ? readFileSync(eventsPath, 'utf8')
        .split('\n')
        .filter((line) => line.trim())
        .map((line) => JSON.parse(line))
    : null;
  return { folder, stats, events };
}
