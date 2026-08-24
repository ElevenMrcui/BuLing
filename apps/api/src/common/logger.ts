import pino from "pino";

/**
 * 结构化日志：JSON 单行输出，便于生产环境 Loki 采集。
 * 关键字段过 redact 白名单，防止 API Key / Authorization 头误落。
 */
export const logger = pino({
  level: process.env.LOG_LEVEL ?? "info",
  redact: {
    paths: [
      "req.headers.authorization",
      "req.headers.cookie",
      "*.apiKey",
      "*.apiKeyEnc",
      "*.password",
      "*.passwordHash",
    ],
    censor: "[REDACTED]",
  },
});
