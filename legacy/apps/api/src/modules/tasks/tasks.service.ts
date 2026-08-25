import { BadRequestException, ForbiddenException, Injectable, NotFoundException } from "@nestjs/common";
import { PrismaService } from "../../common/prisma.service.js";
import {
  IntentBreakdown,
  REJECT_DOWNWEIGHT_THRESHOLD,
  TASK_TRANSITIONS,
  TaskDTO,
  TaskStatus,
  TaskStep,
} from "@buling/shared";
import { PublishTaskDto, ClaimTaskDto } from "./tasks.dto.js";
import { Task, Agent } from "@prisma/client";

@Injectable()
export class TasksService {
  constructor(private readonly prisma: PrismaService) {}

  // -------- 查询 --------

  async list(status?: TaskStatus[]): Promise<TaskDTO[]> {
    const where = status?.length ? { status: { in: status } } : {};
    const rows = await this.prisma.task.findMany({
      where,
      // 待认领列的排序即"人工置顶永远盖过算法"约束：pinned 优先，其次未认领时长
      orderBy: [{ pinned: "desc" }, { openHours: "desc" }, { createdAt: "desc" }],
    });
    return rows.map(toTaskDTO);
  }

  async get(id: string): Promise<TaskDTO> {
    const t = await this.prisma.task.findUnique({ where: { id } });
    if (!t) throw new NotFoundException("task not found");
    return toTaskDTO(t);
  }

  // -------- 发布 --------

  async publish(input: PublishTaskDto): Promise<TaskDTO> {
    const priority = input.priority ?? "mid";
    const initialStatus: TaskStatus = input.assignMode === "auto" ? "open" : "planning";

    // 团队模式：校验团队 + 决定 teamOwnerAgentId（不填就取团队默认 owner），
    // 并生成标准三步 plan：需求拆解 / 团队成员分工执行（待指派）/ 汇总提交前自查
    let teamId: string | null = null;
    let teamOwnerAgentId: string | null = null;
    if (input.assignMode === "team") {
      if (!input.teamId) throw new BadRequestException("team assign requires teamId");
      const team = await this.prisma.team.findUnique({ where: { id: input.teamId } });
      if (!team) throw new NotFoundException("team not found");
      teamId = team.id;
      teamOwnerAgentId = input.teamOwnerAgentId ?? team.ownerAgentId;
      if (!team.memberAgentIds.includes(teamOwnerAgentId))
        throw new BadRequestException("teamOwnerAgentId must be a member of the team");
    }

    const assignees: string[] =
      input.assignMode === "manual" && input.assigneeAgentId ? [input.assigneeAgentId] :
      input.assignMode === "team" && teamOwnerAgentId ? [teamOwnerAgentId] :
      [];

    const plan: TaskStep[] =
      input.assignMode === "manual" && input.assigneeAgentId
        ? [{ title: "任务需求拆解", status: "active", agentId: input.assigneeAgentId }]
        : input.assignMode === "team" && teamOwnerAgentId
          ? [
              { title: "需求拆解与关键信息收集", status: "active", agentId: teamOwnerAgentId },
              { title: "团队成员分工执行", status: "pending", agentId: null },
              { title: "汇总与联合评审前自查", status: "pending", agentId: teamOwnerAgentId },
            ]
          : [];

    const created = await this.prisma.task.create({
      data: {
        title: input.title,
        description: input.description ?? "暂无详细描述。",
        status: initialStatus,
        priority,
        requiredSkills: input.requiredSkills.length ? input.requiredSkills : ["通用协作"],
        assignees,
        due: (input.due ?? "待定") + "截止",
        plan: plan as unknown as object,
        assignMode: input.assignMode,
        teamId,
        teamOwnerAgentId,
      },
    });
    return toTaskDTO(created);
  }

  // -------- 认领 --------

  async claim(taskId: string, input: ClaimTaskDto): Promise<TaskDTO> {
    const t = await this.prisma.task.findUnique({ where: { id: taskId } });
    if (!t) throw new NotFoundException("task not found");
    if (t.status !== "open") throw new BadRequestException("task is not open for claim");

    const agent = await this.prisma.agent.findUnique({ where: { id: input.agentId } });
    if (!agent) throw new NotFoundException("agent not found");

    // 服务端二次校验意愿分：客户端传的 agentId 必须真的能算出正分
    const intent = this.computeIntent(t, agent);
    if (intent.score <= 0)
      throw new BadRequestException("agent does not satisfy the skill contract");

    const plan: TaskStep[] = [
      { title: "任务需求拆解", status: "active", agentId: agent.id },
      { title: "资料收集与执行", status: "pending", agentId: agent.id },
      { title: "产出与自查", status: "pending", agentId: null },
    ];
    const updated = await this.prisma.task.update({
      where: { id: taskId },
      data: {
        status: "planning",
        assignees: [agent.id],
        plan: plan as unknown as object,
        progressPct: 5,
      },
    });
    await this.prisma.collaborationEvent.create({
      data: {
        taskId,
        type: "system",
        text: `${agent.name} 以 ${intent.score}% 意愿分自主领取该任务`,
      },
    });
    return toTaskDTO(updated);
  }

  // -------- 人工置顶（唯一算法之外的排序入口） --------

  async pin(taskId: string): Promise<TaskDTO> {
    const updated = await this.prisma.task.update({
      where: { id: taskId },
      data: { pinned: true },
    });
    await this.prisma.collaborationEvent.create({
      data: { taskId, type: "system", text: "任务已人工置顶——人工优先级永远盖过算法排序" },
    });
    return toTaskDTO(updated);
  }

  // -------- 完成步骤 --------

  async completeStep(taskId: string, idx: number): Promise<TaskDTO> {
    const t = await this.prisma.task.findUnique({ where: { id: taskId } });
    if (!t) throw new NotFoundException("task not found");
    const plan = (t.plan as unknown as TaskStep[]).slice();
    const step = plan[idx];
    if (!step) throw new BadRequestException("step index out of range");
    if (step.status !== "active") throw new BadRequestException("step is not active");
    step.status = "done";
    const next = plan[idx + 1];
    if (next && next.status === "pending") next.status = "active";
    const doneCount = plan.filter((s) => s.status === "done").length;
    const progressPct = Math.round((doneCount / plan.length) * 100);
    const allDone = doneCount === plan.length;

    // 评审红线：全部步骤完成也只能进入 review，绝不直接 done
    const nextStatus: TaskStatus = allDone ? "review" : "progress";
    this.validateTransition(t.status as TaskStatus, nextStatus);

    const updated = await this.prisma.task.update({
      where: { id: taskId },
      data: { plan: plan as unknown as object, progressPct, status: nextStatus },
    });
    await this.prisma.collaborationEvent.create({
      data: {
        taskId,
        type: step.agentId ? "agent_msg" : "system",
        agentId: step.agentId ?? null,
        text: allDone ? `已完成全部步骤，任务转入待评审` : `已完成「${step.title}」`,
      },
    });
    return toTaskDTO(updated);
  }

  // -------- 意愿分计算（自主认领机制的核心，暴露给列表页取候选人） --------

  async intentsFor(taskId: string): Promise<IntentBreakdown[]> {
    const t = await this.prisma.task.findUnique({ where: { id: taskId } });
    if (!t) throw new NotFoundException("task not found");
    const agents = await this.prisma.agent.findMany({ where: { status: { not: "offline" } } });
    return agents
      .map((a) => this.computeIntent(t, a))
      .filter((b) => b.base > 0)
      .sort((a, b) => b.score - a.score)
      .slice(0, 5);
  }

  computeIntent(task: Task, agent: Agent): IntentBreakdown {
    const required = task.requiredSkills;
    const matched = required.filter((rs) =>
      agent.skills.some((s) => s.includes(rs) || rs.includes(s)),
    );
    const base = required.length === 0 ? 0 : Math.round((matched.length / required.length) * 100);
    const trace = agent.traceDelta;
    const downweight = agent.recentRejects >= REJECT_DOWNWEIGHT_THRESHOLD ? -8 : 0;
    const score = Math.max(0, Math.min(99, base + trace + downweight));
    return { agentId: agent.id, score, base, trace, downweight, matched };
  }

  // -------- 状态机 --------

  private validateTransition(from: TaskStatus, to: TaskStatus) {
    if (from === to) return;
    if (!TASK_TRANSITIONS[from].includes(to)) {
      throw new ForbiddenException(`illegal task transition: ${from} -> ${to}`);
    }
  }
}

function toTaskDTO(t: Task): TaskDTO {
  return {
    id: t.id,
    title: t.title,
    description: t.description,
    status: t.status as TaskStatus,
    priority: t.priority as TaskDTO["priority"],
    requiredSkills: t.requiredSkills,
    assignees: t.assignees,
    progressPct: t.progressPct,
    openHours: t.openHours,
    pinned: t.pinned,
    due: t.due,
    plan: t.plan as unknown as TaskStep[],
    assignMode: t.assignMode as TaskDTO["assignMode"],
    teamId: t.teamId,
    teamOwnerAgentId: t.teamOwnerAgentId,
    createdAt: t.createdAt.toISOString(),
    updatedAt: t.updatedAt.toISOString(),
  };
}
