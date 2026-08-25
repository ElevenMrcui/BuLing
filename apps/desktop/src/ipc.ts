// 不令 OPC · Tauri IPC 命令的类型化封装。
// 每个函数对应 src-tauri/src/lib.rs 里的一个 #[tauri::command]——薄封装，
// 只做参数改名（camelCase → Rust 端期望的参数名）和返回值类型标注，不加
// 业务逻辑。

import { invoke } from "@tauri-apps/api/core";
import type {
  AgentInfo,
  ClaimScore,
  GateInfo,
  OpcStatus,
  ProjectInfo,
  ProviderInfo,
  TaskInfo,
  TaskRunInfo,
  WorkflowSummary,
} from "./types";

export const ipc = {
  status: () => invoke<OpcStatus>("opc_status"),
  providers: () => invoke<ProviderInfo[]>("opc_providers"),
  agents: () => invoke<AgentInfo[]>("opc_agents"),

  createProject: (input: { slug: string; displayName: string; rootPath: string; goal: string | null; templateId: string | null }) =>
    invoke<ProjectInfo>("opc_create_project", input),
  listProjects: () => invoke<ProjectInfo[]>("opc_list_projects"),
  projectWorkflow: (projectId: string) => invoke<WorkflowSummary | null>("opc_project_workflow", { projectId }),

  workflowTasks: (projectId: string, workflowId: string) => invoke<TaskInfo[]>("opc_workflow_tasks", { projectId, workflowId }),
  workflowReadyTasks: (projectId: string, workflowId: string) =>
    invoke<TaskInfo[]>("opc_workflow_ready_tasks", { projectId, workflowId }),
  workflowRunTask: (projectId: string, workflowId: string, nodeKey: string) =>
    invoke<TaskRunInfo>("opc_workflow_run_task", { projectId, workflowId, nodeKey }),
  workflowGates: (projectId: string, workflowId: string) => invoke<GateInfo[]>("opc_workflow_gates", { projectId, workflowId }),
  workflowApproveGate: (projectId: string, workflowId: string, gateId: string, comment: string | null) =>
    invoke<void>("opc_workflow_approve_gate", { projectId, workflowId, gateId, comment }),
  workflowRejectGate: (projectId: string, workflowId: string, gateId: string, comment: string | null) =>
    invoke<void>("opc_workflow_reject_gate", { projectId, workflowId, gateId, comment }),

  taskClaimableTasks: (projectId: string, workflowId: string) =>
    invoke<TaskInfo[]>("opc_task_claimable_tasks", { projectId, workflowId }),
  taskManualTasks: (projectId: string, workflowId: string) =>
    invoke<TaskInfo[]>("opc_task_manual_tasks", { projectId, workflowId }),
  taskClaim: (projectId: string, workflowId: string, nodeKey: string) =>
    invoke<ClaimScore>("opc_task_claim", { projectId, workflowId, nodeKey }),
  taskAssignManually: (projectId: string, workflowId: string, nodeKey: string, templateAgentId: string) =>
    invoke<void>("opc_task_assign_manually", { projectId, workflowId, nodeKey, templateAgentId }),
  taskRun: (projectId: string, workflowId: string, nodeKey: string) =>
    invoke<TaskRunInfo>("opc_task_run", { projectId, workflowId, nodeKey }),
};

/** Tauri invoke() 的 reject 值不保证是 Error 实例，统一拍平成字符串给 UI 显示。 */
export function errorMessage(err: unknown): string {
  if (err instanceof Error) return err.message;
  return String(err);
}
