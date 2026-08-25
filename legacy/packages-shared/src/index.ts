/**
 * 不令 · 前后端共享契约（唯一真源）
 * 任何 API 契约、枚举值、DTO 形状都在这里，Web 与 API 都从这里 import，
 * 避免 "后端改字段前端不知道" 的漂移。
 */

// -------- 枚举 --------

export type TaskStatus = "open" | "planning" | "progress" | "review" | "done";
export type Priority = "high" | "mid" | "low";
export type AssignMode = "auto" | "manual" | "team";
export type AgentStatus = "working" | "idle" | "offline";
export type CollabPref = "auto" | "manual";
export type Effort = "low" | "medium" | "high";
export type DocType = "doc" | "pdf" | "sheet" | "image";
export type ReviewStatus = "pending" | "approved" | "rejected";
export type ProviderType = "anthropic" | "openai" | "compatible";
export type ProviderStatus = "connected" | "untested" | "error";

// -------- 领域对象 --------

export interface AgentDTO {
  id: string;
  name: string;
  avatar: string;
  role: string;
  status: AgentStatus;
  bio: string;
  skills: string[];
  modules: string[];
  collabPref: CollabPref;
  modelKey: string | null;
  effort: Effort;
  permissions: {
    confirmCode: boolean;
    confirmExternal: boolean;
    budget: string | null;
  };
  traceDelta: number;
  traceLabel: string | null;
  recentRejects: number;
  stats: { tasksDone: number; avgScore: number; collabs: number };
}

export interface TaskStep {
  title: string;
  status: "pending" | "active" | "done";
  agentId?: string | null;
  squad?: string[];
  blocked?: boolean;
  light?: boolean;
}

export interface TaskDTO {
  id: string;
  title: string;
  description: string;
  status: TaskStatus;
  priority: Priority;
  requiredSkills: string[];
  assignees: string[];
  progressPct: number;
  openHours: number;
  pinned: boolean;
  due: string;
  plan: TaskStep[];
  assignMode: AssignMode;
  teamId: string | null;
  teamOwnerAgentId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface TeamDTO {
  id: string;
  name: string;
  description: string;
  ownerAgentId: string;
  memberAgentIds: string[];
}

export interface DocDTO {
  id: string;
  title: string;
  type: DocType;
  taskId: string | null;
  agentId: string;
  preview: string;
  reviewStatus: ReviewStatus;
  approvedByUserId: string | null;
  approvedAt: string | null;
  createdAt: string;
}

export interface ProviderModel {
  id: string;
  label: string;
  hint: string;
  ctx: string;
}

export interface ProviderDTO {
  id: string;
  name: string;
  type: ProviderType;
  baseUrl: string;
  apiKeyLast4: string | null;
  enabled: boolean;
  status: ProviderStatus;
  models: ProviderModel[];
}

// -------- 请求体 --------

export interface PublishTaskInput {
  title: string;
  description?: string;
  requiredSkills: string[];
  priority?: Priority;
  due?: string;
  assignMode: AssignMode;
  /** assignMode='manual' 时必填 */
  assigneeAgentId?: string;
  /** assignMode='team' 时必填 */
  teamId?: string;
  /** assignMode='team' 时必填；不填默认取 team.ownerAgentId */
  teamOwnerAgentId?: string;
}

/** 团队负责人给团队内某步骤指派人（R2 落地） */
export interface TeamAssignStepInput {
  agentId: string;
}

export interface ClaimTaskInput {
  agentId: string;
}

// -------- 意愿分（自主认领机制服务端计算） --------

export interface IntentBreakdown {
  agentId: string;
  score: number;
  base: number;
  trace: number;
  downweight: number;
  matched: string[];
}

// -------- 统一响应 --------

export type ApiOk<T> = { data: T };
export type ApiErr = { error: { code: string; message: string; details?: unknown } };
export type ApiResp<T> = ApiOk<T> | ApiErr;

// -------- 常量 --------

export const REJECT_DOWNWEIGHT_THRESHOLD = 2;
export const REVIEW_ROLE = "reviewer" as const;

/** 允许的任务状态转移；用于服务端 validateTransition() 与前端 UI 兜底 */
export const TASK_TRANSITIONS: Record<TaskStatus, TaskStatus[]> = {
  open: ["planning"],
  planning: ["progress"],
  progress: ["review"],
  review: ["done"],
  done: [],
};
