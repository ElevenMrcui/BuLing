import { promisify } from "node:util";
import { execFile as _execFile } from "node:child_process";
import { promises as fs } from "node:fs";
import type { CliSignature, DiscoveredCli, Probe } from "@buling/cli-registry";

const execFile = promisify(_execFile);

/** 单个 CLI 签名的完整探测：顺序试所有 probe，一命中即视为已安装。 */
export async function scan(sigs: readonly CliSignature[]): Promise<DiscoveredCli[]> {
  return Promise.all(sigs.map(runOne));
}

async function runOne(sig: CliSignature): Promise<DiscoveredCli> {
  const base: Omit<DiscoveredCli, "status" | "installedVia" | "installedPath" | "version"> = {
    id: sig.id,
    name: sig.name,
    vendor: sig.vendor,
    category: sig.category,
    description: sig.description,
    agentUsage: sig.agentUsage,
    homepage: sig.homepage,
    suggestInstall: sig.suggestInstall,
  };
  try {
    for (const p of sig.probes) {
      const hitPath = await probe(p);
      if (hitPath) {
        const version = sig.versionCmd ? await tryVersion(sig.versionCmd) : null;
        return {
          ...base,
          status: "installed",
          installedVia: p.kind,
          installedPath: hitPath,
          version,
        };
      }
    }
    return { ...base, status: "not-found", installedVia: null, installedPath: null, version: null };
  } catch (err) {
    return {
      ...base,
      status: "error",
      installedVia: null,
      installedPath: null,
      version: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

/** 一次 probe：命中返回文件绝对路径，未命中返回 null。 */
async function probe(p: Probe): Promise<string | null> {
  switch (p.kind) {
    case "which": {
      // 尽量走 POSIX `command -v` 而不是 `which`——前者是 shell 内置且移植性更好
      try {
        const { stdout } = await execFile("/bin/sh", ["-c", `command -v ${shellQuote(p.bin)}`], {
          timeout: 3000,
        });
        const path = stdout.trim();
        return path.length ? path : null;
      } catch {
        return null;
      }
    }
    case "appBundle":
    case "file": {
      try {
        await fs.access(p.path);
        return p.path;
      } catch {
        return null;
      }
    }
    case "brew": {
      try {
        const { stdout } = await execFile("brew", ["--prefix", p.formula], { timeout: 5000 });
        const prefix = stdout.trim();
        if (!prefix.length) return null;
        await fs.access(prefix);
        return prefix;
      } catch {
        return null;
      }
    }
  }
}

/** `--version` 探测：带 3s 超时；失败返回 null 而不是抛错——只是"更好看的元数据"，不阻塞主流程。 */
async function tryVersion(cmd: readonly string[]): Promise<string | null> {
  const [bin, ...args] = cmd;
  if (!bin) return null;
  try {
    const { stdout, stderr } = await execFile(bin, args, { timeout: 3000 });
    const out = (stdout || stderr).split("\n")[0]?.trim() ?? "";
    return out.length ? out.slice(0, 120) : null;
  } catch {
    return null;
  }
}

function shellQuote(s: string): string {
  // 只允许字母数字下划线连字符点号——CLI 名早在 registry 里由我们写死，不接受用户输入
  if (!/^[a-zA-Z0-9_.\-]+$/.test(s)) throw new Error(`invalid bin name: ${s}`);
  return s;
}
