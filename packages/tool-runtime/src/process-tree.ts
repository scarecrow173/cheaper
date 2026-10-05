import { spawn } from "node:child_process";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";

const HELPER_TIMEOUT_MS = 2000;
const HELPER_OUTPUT_BYTES = 1048576;
const systemRoot = () => process.env.SystemRoot ?? "C:\\Windows";

/** 固定 OS helper の出力・実行時間を制限し、close まで回収する。 */
async function helper(executable: string, args: string[]): Promise<string> {
  const child = spawn(executable, args, {
    shell: false,
    windowsHide: true,
    stdio: ["ignore", "pipe", "ignore"],
    env: { SystemRoot: systemRoot(), PATH: "", NODE_V8_COVERAGE: undefined },
  });
  const chunks: Buffer[] = [];
  let size = 0;
  let failure: Error | undefined;
  const stop = (error: Error) => {
    failure ??= error;
    child.kill("SIGKILL");
    child.stdout.destroy();
  };
  const timer = setTimeout(
    () => stop(new Error("process tree helper deadline exceeded")),
    HELPER_TIMEOUT_MS,
  );
  const onData = (chunk: Buffer) => {
    size += chunk.length;
    if (size > HELPER_OUTPUT_BYTES) stop(new Error("process tree helper output exceeded"));
    else chunks.push(chunk);
  };
  child.stdout.on("data", onData);
  try {
    const code = await new Promise<number | null>((resolve) => {
      child.once("error", (error) => {
        failure ??= error;
      });
      child.once("close", resolve);
    });
    if (failure) throw failure;
    if (code !== 0) throw new Error(`process tree helper failed (${code})`);
    return Buffer.concat(chunks).toString("utf8");
  } finally {
    clearTimeout(timer);
    child.stdout.removeListener("data", onData);
    child.stdout.destroy();
    child.removeAllListeners();
  }
}

interface Identity {
  pid: number;
  parent: number;
  created: string;
}
const QUERY =
  "$ErrorActionPreference='Stop'; @(Get-CimInstance Win32_Process | Select-Object @{n='pid';e={[int]$_.ProcessId}},@{n='parent';e={[int]$_.ParentProcessId}},@{n='created';e={$_.CreationDate.ToUniversalTime().ToString('o')}}) | ConvertTo-Json -Compress";
async function snapshot(): Promise<Identity[]> {
  const executable = path.join(
    systemRoot(),
    "System32",
    "WindowsPowerShell",
    "v1.0",
    "powershell.exe",
  );
  const data: unknown = JSON.parse(
    await helper(executable, ["-NoProfile", "-NonInteractive", "-Command", QUERY]),
  );
  if (
    !Array.isArray(data) ||
    !data.every(
      (p) =>
        p &&
        Number.isSafeInteger(p.pid) &&
        Number.isSafeInteger(p.parent) &&
        typeof p.created === "string",
    )
  )
    throw new Error("invalid process identity snapshot");
  return data;
}

export interface ProcessTree {
  terminate(graceMs: number): Promise<void>;
}

/** OS containment は提供しない。Windows の snapshot 前の ancestry-loss は W1 limitation。 */
export function createProcessTree(pid: number): ProcessTree {
  if (process.platform !== "win32") {
    const send = (signal: NodeJS.Signals | 0): boolean => {
      try {
        process.kill(-pid, signal);
        return true;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ESRCH") return false;
        throw error;
      }
    };
    return {
      async terminate(graceMs) {
        if (!send("SIGTERM")) return;
        const deadline = performance.now() + graceMs;
        while (send(0) && performance.now() < deadline) {
          await delay(Math.min(25, Math.max(1, deadline - performance.now())));
        }
        if (send(0)) send("SIGKILL");
      },
    };
  }
  const known = new Map<number, string>();
  const bornBefore = Date.now();
  const collect = (all: Identity[]): Identity[] => {
    const root = all.find((p) => p.pid === pid);
    // 初回 snapshot 時点で root PID が再利用済みなら、その新 tree を対象にしない。
    const rootValid =
      root &&
      (known.has(pid) ? known.get(pid) === root.created : Date.parse(root.created) <= bornBefore);
    const selected = new Set<number>();
    if (rootValid) selected.add(pid);
    for (const p of all) if (known.get(p.pid) === p.created) selected.add(p.pid);
    let changed = true;
    while (changed) {
      changed = false;
      for (const p of all) {
        if (!selected.has(p.pid) && (selected.has(p.parent) || (!root && p.parent === pid))) {
          selected.add(p.pid);
          changed = true;
        }
      }
    }
    const live = all.filter((p) => selected.has(p.pid));
    for (const p of live) known.set(p.pid, p.created);
    return live;
  };
  // rejection は直ちに捕捉し、cleanup で明示的に伝播する。
  const first = snapshot()
    .then(collect)
    .then(
      () => undefined,
      (error) => error as Error,
    );
  const kill = async (live: Identity[], force: boolean) => {
    if (live.length === 0) return;
    const executable = path.join(systemRoot(), "System32", "taskkill.exe");
    try {
      await helper(executable, [
        ...live.flatMap((p) => ["/PID", String(p.pid)]),
        "/T",
        ...(force ? ["/F"] : []),
      ]);
    } catch (error) {
      // taskkill と自然終了が競合した場合だけ成功として扱う。
      if (collect(await snapshot()).length > 0) throw error;
    }
  };
  return {
    async terminate(graceMs) {
      const initialError = await first;
      if (initialError) throw initialError;
      let live = collect(await snapshot());
      if (live.length === 0) return;
      // console app の非 /F termination は best effort。失敗しても force escalation を行う。
      try {
        await kill(live, false);
      } catch {
        /* 残存 identity を force phase で再検証する。 */
      }
      const deadline = performance.now() + graceMs;
      do {
        live = collect(await snapshot());
        if (live.length === 0) return;
        if (performance.now() < deadline) await delay(Math.min(50, deadline - performance.now()));
      } while (performance.now() < deadline);
      await kill(live, true);
      if (collect(await snapshot()).length > 0)
        throw new Error("managed process descendants survived force termination");
    },
  };
}
