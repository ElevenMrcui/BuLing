import { promises as fs } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { randomBytes } from "node:crypto";

/**
 * 网关 Token 引导：首次启动生成 32B 随机 Token，落到 `~/.buling/gateway.token`（chmod 600）。
 * 打印到 stdout 供用户复制粘贴到 Web / 原型的"配对"输入框。
 * Token 不会通过任何网络下发；本地网关也不与任何远端服务通信。
 */

export interface GatewayIdentity {
  token: string;
  tokenPath: string;
  isNew: boolean;
}

export async function ensureToken(): Promise<GatewayIdentity> {
  const dir = join(homedir(), ".buling");
  const path = join(dir, "gateway.token");
  try {
    const buf = await fs.readFile(path, "utf8");
    const token = buf.trim();
    if (token.length >= 32) return { token, tokenPath: path, isNew: false };
  } catch {}
  await fs.mkdir(dir, { recursive: true, mode: 0o700 });
  const token = randomBytes(32).toString("base64url");
  await fs.writeFile(path, token, { mode: 0o600 });
  return { token, tokenPath: path, isNew: true };
}
