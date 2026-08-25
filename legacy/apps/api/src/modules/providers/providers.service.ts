import { Injectable, NotFoundException } from "@nestjs/common";
import { IsIn, IsOptional, IsString, MaxLength } from "class-validator";
import { PrismaService } from "../../common/prisma.service.js";
import { encryptApiKey } from "./key-crypto.js";
import type { ProviderDTO, ProviderModel, ProviderType } from "@buling/shared";
import type { Provider } from "@prisma/client";

export class UpsertProviderDto {
  @IsString() @MaxLength(100) name!: string;
  @IsIn(["anthropic", "openai", "compatible"]) type!: ProviderType;
  @IsString() baseUrl!: string;
  @IsOptional() @IsString() apiKey?: string; // 明文入参，落库前加密
}

/**
 * 演示态"测试连接"：按类型返回一批示意模型；与原型 §模型接入 一致的边界——
 * 不发起真实上游请求，避免误消耗额度 / 泄露 key。R2 接真实拉取。
 */
const SAMPLE_DISCOVERY: Record<ProviderType, ProviderModel[]> = {
  anthropic: [
    { id: "haiku", label: "Claude Haiku 4.5", hint: "轻量快速", ctx: "200K" },
    { id: "sonnet", label: "Claude Sonnet 5", hint: "均衡通用", ctx: "200K" },
    { id: "opus", label: "Claude Opus 5", hint: "深度推理", ctx: "200K" },
  ],
  openai: [
    { id: "gpt-4o", label: "GPT-4o", hint: "多模态通用", ctx: "128K" },
    { id: "gpt-4o-mini", label: "GPT-4o mini", hint: "低成本高频", ctx: "128K" },
  ],
  compatible: [
    { id: "custom-1", label: "（连接后由服务返回）", hint: "实际模型以服务响应为准", ctx: "—" },
  ],
};

@Injectable()
export class ProvidersService {
  constructor(private readonly prisma: PrismaService) {}

  async list(): Promise<ProviderDTO[]> {
    const rows = await this.prisma.provider.findMany({ orderBy: { createdAt: "desc" } });
    return rows.map(toDTO);
  }

  async create(input: UpsertProviderDto): Promise<ProviderDTO> {
    const key = input.apiKey ? encryptApiKey(input.apiKey) : null;
    const created = await this.prisma.provider.create({
      data: {
        name: input.name,
        type: input.type,
        baseUrl: input.baseUrl,
        apiKeyEnc: key?.enc ?? null,
        apiKeyIv: key?.iv ?? null,
        apiKeyTag: key?.tag ?? null,
        apiKeyLast4: key?.last4 ?? null,
        enabled: true,
        status: "untested",
        models: [],
      },
    });
    return toDTO(created);
  }

  async test(id: string): Promise<ProviderDTO> {
    const p = await this.prisma.provider.findUnique({ where: { id } });
    if (!p) throw new NotFoundException("provider not found");
    const models = SAMPLE_DISCOVERY[p.type as ProviderType];
    const updated = await this.prisma.provider.update({
      where: { id },
      data: { status: "connected", models: models as unknown as object },
    });
    return toDTO(updated);
  }

  async setEnabled(id: string, enabled: boolean): Promise<ProviderDTO> {
    const updated = await this.prisma.provider.update({ where: { id }, data: { enabled } });
    return toDTO(updated);
  }

  async remove(id: string): Promise<{ ok: true }> {
    await this.prisma.provider.delete({ where: { id } });
    return { ok: true };
  }
}

function toDTO(p: Provider): ProviderDTO {
  return {
    id: p.id,
    name: p.name,
    type: p.type as ProviderType,
    baseUrl: p.baseUrl,
    apiKeyLast4: p.apiKeyLast4,
    enabled: p.enabled,
    status: p.status as ProviderDTO["status"],
    models: (p.models as unknown as ProviderModel[]) ?? [],
  };
}
