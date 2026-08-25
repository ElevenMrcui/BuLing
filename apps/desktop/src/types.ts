// 不令 OPC · 跟 Tauri IPC 命令返回值一一对应的 TS 类型。
// 字段名保持跟 Rust 端 serde 输出（snake_case）一致，不做驼峰转换——
// 减少一层容易漂移的映射。

export interface OpcStatus {
  app_db_version: number;
  app_db_path: string;
  ready: boolean;
  agents_seeded: number;
}

export interface ProviderInfo {
  id: string;
  display_name: string;
  vendor: string;
  kind: "cli" | "api" | "local";
  wire_format: string | null;
  available: boolean;
  detail: string | null;
}

export interface AgentInfo {
  id: string;
  kind: "preset" | "user";
  role: string;
  display_name: string;
  avatar: string | null;
  sensitivity: "low" | "medium" | "high";
  provider_priority: string[];
  version: number;
}

export interface ProjectInfo {
  id: string;
  slug: string;
  display_name: string;
  root_path: string;
  status: string;
  starred: boolean;
  last_opened_at: string | null;
  active_workflow_id: string | null;
}

export interface WorkflowSummary {
  id: string;
  template_id: string;
  name: string;
  status: string;
}

export interface TaskInfo {
  node_key: string;
  kind: string;
  role: string | null;
  assignment_mode: string;
  status: string;
}

export interface GateInfo {
  id: string;
  status: string;
}

export interface ClaimScore {
  agent_instance_id: string;
  template_agent_id: string;
  score: number;
}

export interface TaskRunInfo {
  task_run_id: string;
  provider_id: string;
  artifact_ids: string[];
}
