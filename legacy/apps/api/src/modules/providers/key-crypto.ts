import { createCipheriv, createDecipheriv, randomBytes } from "node:crypto";

/**
 * Provider API Key 加密（AES-256-GCM）：密钥永不明文落库。
 * 主密钥来源：本地 env（PROVIDER_MASTER_KEY_BASE64），生产由 KMS / Vault 注入。
 * DB 里保存 {enc, iv, tag, last4}——last4 供界面 sk-****last4 脱敏展示，
 * 明文只在外发上游 API 时在内存中解密使用。
 */

const ALGO = "aes-256-gcm";
const IV_BYTES = 12;

function masterKey(): Buffer {
  const b64 = process.env.PROVIDER_MASTER_KEY_BASE64;
  if (!b64) throw new Error("PROVIDER_MASTER_KEY_BASE64 not set");
  const key = Buffer.from(b64, "base64");
  if (key.length !== 32) throw new Error("PROVIDER_MASTER_KEY_BASE64 must decode to 32 bytes");
  return key;
}

export interface EncryptedApiKey {
  enc: Buffer;
  iv: Buffer;
  tag: Buffer;
  last4: string;
}

export function encryptApiKey(plaintext: string): EncryptedApiKey {
  const iv = randomBytes(IV_BYTES);
  const cipher = createCipheriv(ALGO, masterKey(), iv);
  const enc = Buffer.concat([cipher.update(plaintext, "utf8"), cipher.final()]);
  const tag = cipher.getAuthTag();
  return { enc, iv, tag, last4: plaintext.slice(-4) };
}

export function decryptApiKey(rec: { enc: Buffer; iv: Buffer; tag: Buffer }): string {
  const decipher = createDecipheriv(ALGO, masterKey(), rec.iv);
  decipher.setAuthTag(rec.tag);
  const buf = Buffer.concat([decipher.update(rec.enc), decipher.final()]);
  return buf.toString("utf8");
}
