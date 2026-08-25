import type { ApiResp, TaskDTO, AgentDTO, ProviderDTO, IntentBreakdown } from "@buling/shared";

const BASE = process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:13500";

/**
 * 极简 API 客户端。actor 头当前手工带；R2 接入真实 JWT 后从 cookie / session 取。
 * 评审接口必须带 actor.type=user + roles 含 reviewer——否则服务端 ActorGuard 直接 403。
 */
async function req<T>(
  path: string,
  init: RequestInit & { actor?: { type: "user" | "system" | "agent"; id: string; roles?: string[] } } = {},
): Promise<T> {
  const { actor, headers, ...rest } = init;
  const h = new Headers(headers);
  h.set("content-type", "application/json");
  if (actor) {
    h.set("x-actor-type", actor.type);
    h.set("x-actor-id", actor.id);
    if (actor.roles) h.set("x-actor-roles", actor.roles.join(","));
  }
  const res = await fetch(`${BASE}/api${path}`, { ...rest, headers: h, cache: "no-store" });
  const body = (await res.json()) as ApiResp<T>;
  if (!res.ok || "error" in body) {
    const err = "error" in body ? body.error : { message: res.statusText, code: String(res.status) };
    throw new Error(`${err.code}: ${err.message}`);
  }
  return body.data;
}

export const api = {
  health: () => req<{ status: string; ts: string }>("/health"),
  tasks: {
    list: (status?: string) => req<TaskDTO[]>(`/tasks${status ? "?status=" + status : ""}`),
    get: (id: string) => req<TaskDTO>(`/tasks/${id}`),
    intents: (id: string) => req<IntentBreakdown[]>(`/tasks/${id}/intents`),
    publish: (body: unknown) =>
      req<TaskDTO>(`/tasks`, {
        method: "POST",
        body: JSON.stringify(body),
        actor: { type: "user", id: "u-demo", roles: ["operator"] },
      }),
    claim: (id: string, agentId: string) =>
      req<TaskDTO>(`/tasks/${id}/claim`, {
        method: "POST",
        body: JSON.stringify({ agentId }),
        actor: { type: "user", id: "u-demo", roles: ["operator"] },
      }),
    completeStep: (id: string, idx: number) =>
      req<TaskDTO>(`/tasks/${id}/plan/${idx}/complete`, {
        method: "POST",
        actor: { type: "user", id: "u-demo", roles: ["operator"] },
      }),
    pin: (id: string) =>
      req<TaskDTO>(`/tasks/${id}/pin`, {
        method: "POST",
        actor: { type: "user", id: "u-demo", roles: ["operator"] },
      }),
  },
  agents: {
    list: () => req<AgentDTO[]>("/agents"),
  },
  providers: {
    list: () => req<ProviderDTO[]>("/providers"),
  },
  review: {
    approve: (docId: string) =>
      req<{ ok: true }>(`/review/docs/${docId}/approve`, {
        method: "POST",
        actor: { type: "user", id: "u-reviewer", roles: ["reviewer"] },
      }),
    reject: (docId: string, reason?: string) =>
      req<{ ok: true }>(`/review/docs/${docId}/reject`, {
        method: "POST",
        body: JSON.stringify({ reason }),
        actor: { type: "user", id: "u-reviewer", roles: ["reviewer"] },
      }),
  },
};
