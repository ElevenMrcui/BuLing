import { CanActivate, ExecutionContext, ForbiddenException, Injectable, SetMetadata } from "@nestjs/common";
import { Reflector } from "@nestjs/core";
import type { FastifyRequest } from "fastify";

/**
 * 请求 Actor 抽象：三类身份。评审红线依赖它——review 接口只接受 `user`。
 * R1 骨架用请求头 X-Actor-Type / X-Actor-Id 手工带；R2 接入真实 JWT 后从 token 提取。
 */
export type ActorType = "user" | "system" | "agent";
export interface Actor { type: ActorType; id: string; roles: string[]; }

export function extractActor(req: FastifyRequest): Actor {
  const type = (req.headers["x-actor-type"] as ActorType) ?? "user";
  const id = (req.headers["x-actor-id"] as string) ?? "anonymous";
  const roles = ((req.headers["x-actor-roles"] as string) ?? "").split(",").filter(Boolean);
  return { type, id, roles };
}

/** 声明"此接口只允许某些 ActorType 调用" */
export const ALLOWED_ACTORS_KEY = "allowedActors";
export const AllowedActors = (...types: ActorType[]) => SetMetadata(ALLOWED_ACTORS_KEY, types);

@Injectable()
export class ActorGuard implements CanActivate {
  constructor(private readonly reflector: Reflector) {}
  canActivate(ctx: ExecutionContext): boolean {
    const allowed = this.reflector.getAllAndOverride<ActorType[] | undefined>(
      ALLOWED_ACTORS_KEY,
      [ctx.getHandler(), ctx.getClass()],
    );
    if (!allowed || allowed.length === 0) return true;
    const req = ctx.switchToHttp().getRequest<FastifyRequest>();
    const actor = extractActor(req);
    if (!allowed.includes(actor.type)) {
      throw new ForbiddenException(
        `此接口只允许 [${allowed.join(",")}] 调用，当前 actor.type=${actor.type}——评审红线由此拦截`,
      );
    }
    (req as any).actor = actor;
    return true;
  }
}
