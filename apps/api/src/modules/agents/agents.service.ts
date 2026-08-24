import { Injectable, NotFoundException } from "@nestjs/common";
import { PrismaService } from "../../common/prisma.service.js";
import type { Agent } from "@prisma/client";
import type { AgentDTO } from "@buling/shared";

@Injectable()
export class AgentsService {
  constructor(private readonly prisma: PrismaService) {}

  async list(): Promise<AgentDTO[]> {
    const rows = await this.prisma.agent.findMany({ orderBy: { createdAt: "desc" } });
    return rows.map(toDTO);
  }

  async get(id: string): Promise<AgentDTO> {
    const a = await this.prisma.agent.findUnique({ where: { id } });
    if (!a) throw new NotFoundException("agent not found");
    return toDTO(a);
  }
}

function toDTO(a: Agent): AgentDTO {
  return {
    id: a.id,
    name: a.name,
    avatar: a.avatar,
    role: a.role,
    status: a.status as AgentDTO["status"],
    bio: a.bio,
    skills: a.skills,
    modules: a.modules,
    collabPref: a.collabPref as AgentDTO["collabPref"],
    modelKey: a.modelKey,
    effort: a.effort as AgentDTO["effort"],
    permissions: (a.permissions as unknown) as AgentDTO["permissions"],
    traceDelta: a.traceDelta,
    traceLabel: a.traceLabel,
    recentRejects: a.recentRejects,
    stats: (a.stats as unknown) as AgentDTO["stats"],
  };
}
