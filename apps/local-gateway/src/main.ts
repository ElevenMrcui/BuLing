import Fastify from "fastify";
import cors from "@fastify/cors";
import { platform } from "node:os";
import { MAC_CLI_REGISTRY } from "@buling/cli-registry";
import { scan } from "./scan.js";
import { ensureToken } from "./token.js";

/**
 * 不令 · 本地网关（Local Gateway）
 *
 * 只做一件事：**在用户 Mac 上枚举已安装的 AI 厂商 CLI**，供不令 Web / 原型在
 * "创建 Agent · 技能与模块 · 本机发现" 里选择。
 *
 * 强不变量：
 *  - 只监听 127.0.0.1（回环，网卡上不可达）
 *  - 所有接口需带 X-Gateway-Token 头，Token 首次启动生成，落到 ~/.buling/gateway.token（chmod 600）
 *  - 只读：只 `command -v` / `fs.access` / 可选 `<cli> --version`，**不 spawn 用户代码、不写任何文件（Token 除外）**
 *  - 不与任何远端服务通信；不发遥测；不加载插件
 *  - CORS 白名单：`null`（file:// 打开原型）+ `http(s)://claude.ai`（Artifact）+ `http://localhost:*`
 */

const PORT = Number(process.env.PORT ?? 17817);
const app = Fastify({ logger: { level: process.env.LOG_LEVEL ?? "info" } });

const { token, tokenPath, isNew } = await ensureToken();

await app.register(cors, {
  origin: (origin, cb) => {
    // file:// 与 sandboxed 场景下浏览器发的是字符串 "null"（不是缺省），也放行
    if (!origin || origin === "null") return cb(null, true);
    try {
      const u = new URL(origin);
      const ok =
        u.hostname === "claude.ai" ||
        u.hostname.endsWith(".claude.ai") ||
        u.hostname === "localhost" ||
        u.hostname === "127.0.0.1";
      return cb(null, ok);
    } catch {
      return cb(null, false);
    }
  },
  credentials: false,
  allowedHeaders: ["X-Gateway-Token", "Content-Type"],
  // Chrome Private Network Access：https 页面访问本机 http 需要此头
  exposedHeaders: ["Access-Control-Allow-Private-Network"],
});

// 全局鉴权（除 CORS 预检）
app.addHook("preHandler", async (req, rep) => {
  if (req.method === "OPTIONS") return;
  const t = req.headers["x-gateway-token"];
  if (t !== token) {
    rep.code(401);
    throw new Error("invalid or missing X-Gateway-Token");
  }
});

// 探活：不返回 Token；只返回可读的元数据
app.get("/health", async () => ({
  data: {
    status: "ok",
    platform: platform(),
    node: process.versions.node,
    supported: platform() === "darwin",
    registrySize: MAC_CLI_REGISTRY.length,
  },
}));

// 主接口：扫描并返回所有 CLI 签名的探测结果
app.get("/discover", async () => {
  const t0 = Date.now();
  const items = await scan(MAC_CLI_REGISTRY);
  return {
    data: {
      items,
      scannedAt: new Date().toISOString(),
      durationMs: Date.now() - t0,
      platform: platform(),
    },
  };
});

await app.listen({ port: PORT, host: "127.0.0.1" });

printBanner({ port: PORT, token, tokenPath, isNew });

function printBanner(x: { port: number; token: string; tokenPath: string; isNew: boolean }) {
  const line = "─".repeat(58);
  const supported = platform() === "darwin";
  process.stdout.write(
    `\n${line}\n` +
      `不令 · 本地网关已就绪  http://127.0.0.1:${x.port}\n` +
      `${line}\n` +
      `Token ${x.isNew ? "（首次生成）" : "（沿用）"}: ${x.token}\n` +
      `Token 存放: ${x.tokenPath}\n` +
      `\n打开原型 / Web，在"创建 Agent · 技能与模块 · 本机发现"里粘贴 Token 完成配对。\n` +
      (supported
        ? ""
        : `\n[注意] 当前平台 ${platform()}；本地网关目前只对 macOS 的 CLI 签名做过校准，其它平台仅"能启动"，扫描结果多为 not-found。\n`) +
      `${line}\n\n`,
  );
}
