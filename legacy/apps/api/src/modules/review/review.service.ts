import { ForbiddenException, Injectable, NotFoundException } from "@nestjs/common";
import { Prisma } from "@prisma/client";
import { PrismaService } from "../../common/prisma.service.js";
import type { Actor } from "../../common/actor.js";
import { REJECT_DOWNWEIGHT_THRESHOLD, REVIEW_ROLE } from "@buling/shared";

/**
 * 评审红线（评审环节不可自动化）落地：
 * - ActorGuard 已限定 review 接口只允许 actor.type === 'user' 调用；
 * - 本 service 再检查 actor 拥有 'reviewer' 角色（双重）；
 * - 只有本服务能把 Task.status 从 'review' 推到 'done'——tasks 模块无此路径。
 */
@Injectable()
export class ReviewService {
  constructor(private readonly prisma: PrismaService) {}

  async approve(docId: string, actor: Actor) {
    this.assertHumanReviewer(actor);
    const doc = await this.prisma.doc.findUnique({ where: { id: docId } });
    if (!doc) throw new NotFoundException("doc not found");
    if (doc.reviewStatus === "approved") return { alreadyApproved: true };

    await this.prisma.$transaction(async (tx: Prisma.TransactionClient) => {
      await tx.doc.update({
        where: { id: docId },
        data: {
          reviewStatus: "approved",
          approvedByUserId: actor.id,
          approvedAt: new Date(),
        },
      });
      // 协作痕迹机制：批准可让被打回过的 Agent 权重自然恢复
      const agent = await tx.agent.findUnique({ where: { id: doc.agentId } });
      if (agent && agent.recentRejects > 0) {
        await tx.agent.update({
          where: { id: agent.id },
          data: { recentRejects: agent.recentRejects - 1 },
        });
      }
      // 若属于某任务，检查该任务全部产出是否都已批准 → 转 done
      if (doc.taskId) {
        const task = await tx.task.findUnique({
          where: { id: doc.taskId },
          include: { docs: true },
        });
        if (task && task.status === "review") {
          const allApproved =
            task.docs.length > 0 && task.docs.every((d) => d.id === docId || d.reviewStatus === "approved");
          if (allApproved) {
            await tx.task.update({
              where: { id: task.id },
              data: { status: "done", progressPct: 100 },
            });
          }
        }
      }
      await tx.auditLog.create({
        data: {
          actorType: actor.type,
          actorId: actor.id,
          action: "review.approve",
          entity: "doc",
          entityId: docId,
          after: { reviewStatus: "approved" },
        },
      });
    });
    return { ok: true };
  }

  async reject(docId: string, actor: Actor, reason?: string) {
    this.assertHumanReviewer(actor);
    const doc = await this.prisma.doc.findUnique({ where: { id: docId } });
    if (!doc) throw new NotFoundException("doc not found");

    await this.prisma.$transaction(async (tx: Prisma.TransactionClient) => {
      await tx.doc.update({
        where: { id: docId },
        data: { reviewStatus: "rejected" },
      });
      // 协作痕迹机制：临时降权（非永久拉黑）
      const agent = await tx.agent.findUnique({ where: { id: doc.agentId } });
      if (agent) {
        await tx.agent.update({
          where: { id: agent.id },
          data: { recentRejects: agent.recentRejects + 1 },
        });
        if (agent.recentRejects + 1 === REJECT_DOWNWEIGHT_THRESHOLD && doc.taskId) {
          await tx.collaborationEvent.create({
            data: {
              taskId: doc.taskId,
              type: "system",
              text: `${agent.name} 近期多次被打回，匹配权重已临时下调（非永久，产出回升后自动恢复）`,
            },
          });
        }
      }
      await tx.auditLog.create({
        data: {
          actorType: actor.type,
          actorId: actor.id,
          action: "review.reject",
          entity: "doc",
          entityId: docId,
          after: { reviewStatus: "rejected", reason: reason ?? null },
        },
      });
    });
    return { ok: true };
  }

  private assertHumanReviewer(actor: Actor) {
    if (actor.type !== "user")
      throw new ForbiddenException("评审红线：非人工用户不得批准或打回");
    if (!actor.roles.includes(REVIEW_ROLE))
      throw new ForbiddenException(`评审红线：角色需包含 '${REVIEW_ROLE}'，当前 roles=[${actor.roles.join(",")}]`);
  }
}
