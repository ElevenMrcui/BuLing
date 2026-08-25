import { BadRequestException, Injectable, NotFoundException } from "@nestjs/common";
import { IsArray, IsOptional, IsString, MaxLength, MinLength } from "class-validator";
import { PrismaService } from "../../common/prisma.service.js";
import type { TeamDTO } from "@buling/shared";
import type { Team } from "@prisma/client";

export class UpsertTeamDto {
  @IsString() @MinLength(1) @MaxLength(100) name!: string;
  @IsOptional() @IsString() @MaxLength(500) description?: string;
  @IsString() ownerAgentId!: string;
  @IsArray() @IsString({ each: true }) memberAgentIds!: string[];
}

/**
 * 团队服务：与自主认领并列的第二种任务组织方式的载体。
 * 团队负责人可给团队内任务的步骤指派人 / 汇总提交评审——但那些接口挂在 tasks 模块下
 * （POST /tasks/:id/team/*，R2 展开），本模块只负责团队本身的 CRUD。
 */
@Injectable()
export class TeamsService {
  constructor(private readonly prisma: PrismaService) {}

  async list(): Promise<TeamDTO[]> {
    const rows = await this.prisma.team.findMany({ orderBy: { createdAt: "desc" } });
    return rows.map(toDTO);
  }

  async get(id: string): Promise<TeamDTO> {
    const t = await this.prisma.team.findUnique({ where: { id } });
    if (!t) throw new NotFoundException("team not found");
    return toDTO(t);
  }

  async create(input: UpsertTeamDto): Promise<TeamDTO> {
    // 负责人必须在成员里；不满足则自动补入，保持"负责人也是成员"的不变量
    const members = input.memberAgentIds.includes(input.ownerAgentId)
      ? input.memberAgentIds
      : [input.ownerAgentId, ...input.memberAgentIds];
    // 引用完整性：所有成员都必须是真实 Agent
    const found = await this.prisma.agent.findMany({ where: { id: { in: members } } });
    if (found.length !== members.length)
      throw new BadRequestException("some memberAgentIds do not exist");
    const created = await this.prisma.team.create({
      data: {
        name: input.name,
        description: input.description ?? "",
        ownerAgentId: input.ownerAgentId,
        memberAgentIds: members,
      },
    });
    return toDTO(created);
  }

  async remove(id: string): Promise<{ ok: true }> {
    // 有进行中的任务在使用此团队时拒绝删除（前端已有提示，这里做二次防线）
    const inFlight = await this.prisma.task.count({
      where: { teamId: id, status: { in: ["planning", "progress", "review"] } },
    });
    if (inFlight > 0)
      throw new BadRequestException(`team is used by ${inFlight} in-flight tasks`);
    await this.prisma.team.delete({ where: { id } });
    return { ok: true };
  }
}

function toDTO(t: Team): TeamDTO {
  return {
    id: t.id,
    name: t.name,
    description: t.description,
    ownerAgentId: t.ownerAgentId,
    memberAgentIds: t.memberAgentIds,
  };
}
