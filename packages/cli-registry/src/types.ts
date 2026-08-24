/** 一种探测方式；一个 CLI 可有多种（PATH 中 which、macOS /Applications、Homebrew formula）。
 *  一命中即视为已安装；不同 kind 反映不同来源（便于展示"是通过什么装的"）。 */
export type Probe =
  | { kind: "which"; bin: string }
  | { kind: "brew"; formula: string }
  | { kind: "appBundle"; path: string }
  | { kind: "file"; path: string };

export type CliCategory = "agent-cli" | "chat-cli" | "runtime" | "ide-companion" | "sdk";

/** 一个已知 AI 厂商 CLI / 工具的稳定签名——只用来"发现"，不用来"执行"。
 *  执行沙箱是另一层（execution 模块），需要用户在 Agent 上显式授权。 */
export interface CliSignature {
  id: string;             // 稳定 id（永不变）
  name: string;           // 中文展示名
  vendor: string;         // 厂商 / 组织
  category: CliCategory;
  homepage: string;
  description: string;    // 一句话说明它是什么
  agentUsage: string;     // 挂载到 Agent 后能拿它做什么（一句话，供选择时判断）
  probes: Probe[];        // 探测方式（顺序尝试，一命中即视为找到）
  versionCmd?: string[];  // 可选：找到后跑一次以获取版本，如 ['claude', '--version']
  suggestInstall?: string;// 未安装时的一键安装提示（`brew install ...` / `npm i -g ...`）
}

/** 网关扫描结果 —— 一个签名对应一条，无论是否找到都返回，前端据此展示 install/not-found 两种态。 */
export interface DiscoveredCli {
  id: string;
  name: string;
  vendor: string;
  category: CliCategory;
  description: string;
  agentUsage: string;
  homepage: string;
  status: "installed" | "not-found" | "error";
  installedVia: Probe["kind"] | null;
  installedPath: string | null;
  version: string | null;
  suggestInstall?: string;
  error?: string;         // status='error' 时说明原因（例如权限不足）
}
