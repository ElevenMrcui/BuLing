import {
  Brain,
  CheckCircle2,
  Cpu,
  FolderKanban,
  type LucideIcon,
  Inbox,
  LayoutDashboard,
  ListChecks,
  Package,
  Settings,
  Shield,
  Sparkles,
  Users,
  Workflow,
} from "lucide-react";

export type ViewKey =
  | "control-panel"
  | "project-center"
  | "team-center"
  | "task-center"
  | "workflow-center"
  | "artifact-center"
  | "review-center"
  | "skill-center"
  | "memory-center"
  | "model-center"
  | "permission-center"
  | "log-center"
  | "settings";

export interface NavItemDef {
  key: ViewKey;
  label: string;
  icon: LucideIcon;
  group: "core" | "support";
  /** false = 后端还没有对应能力，导航上给「即将推出」标签，点进去是说明页而不是假装能用。 */
  ready: boolean;
}

export const NAV_ITEMS: NavItemDef[] = [
  { key: "control-panel", label: "控制台", icon: LayoutDashboard, group: "core", ready: true },
  { key: "project-center", label: "项目中心", icon: FolderKanban, group: "core", ready: true },
  { key: "team-center", label: "团队中心", icon: Users, group: "core", ready: true },
  { key: "task-center", label: "任务中心", icon: Inbox, group: "core", ready: false },
  { key: "workflow-center", label: "工作流中心", icon: Workflow, group: "core", ready: true },
  { key: "artifact-center", label: "产出中心", icon: Package, group: "core", ready: false },
  { key: "review-center", label: "评审中心", icon: CheckCircle2, group: "core", ready: false },
  { key: "skill-center", label: "技能中心", icon: Sparkles, group: "support", ready: false },
  { key: "memory-center", label: "记忆中心", icon: Brain, group: "support", ready: false },
  { key: "model-center", label: "模型中心", icon: Cpu, group: "support", ready: true },
  { key: "permission-center", label: "权限中心", icon: Shield, group: "support", ready: false },
  { key: "log-center", label: "日志中心", icon: ListChecks, group: "support", ready: false },
  { key: "settings", label: "设置", icon: Settings, group: "support", ready: false },
];

export const COMING_SOON_COPY: Partial<Record<ViewKey, { title: string; body: string; goTo?: ViewKey }>> = {
  "task-center": {
    title: "任务中心还没独立出来",
    body: "认领（能力匹配）和手动指派现在就能用，就在「工作流中心」里——按项目 + 工作流管理任务节点。跨项目的任务汇总视图是下一步。",
    goTo: "workflow-center",
  },
  "artifact-center": {
    title: "产出中心待接入",
    body: "Agent 产出的 Artifact（PRD / 架构 / 代码…）已经在写进项目目录并登记进 project.sqlite（opc-tool），但还没有对应的 Tauri 命令把列表暴露给界面。",
  },
  "review-center": {
    title: "评审红线现在按项目走",
    body: "Gate 的通过 / 打回已经是真实功能，在「工作流中心」里，跟着具体项目的工作流走。跨项目汇总「还有哪些等我审」的视图是下一步。",
    goTo: "workflow-center",
  },
  "skill-center": { title: "技能中心待建", body: "Skill / Tool / MCP 目前只在 agents/*.yaml 里以 JSON 数组声明，还没有独立的管理层。" },
  "memory-center": { title: "记忆中心待建", body: "项目内 Memory 表 + FTS5 全文索引已经在 schema 里，还没有 Runtime crate 和 UI。" },
  "permission-center": { title: "权限中心待建", body: "六大类权限（File/Command/Network/Git/Docker/MCP）的 schema 已就绪，弹窗确认组件和管理界面还没做。" },
  "log-center": { title: "日志中心待建", body: "execution_logs 表已经在 schema 里，但 opc-privacy / opc-audit 这两个 Runtime crate 还没落地，现在没有东西可写。" },
  settings: { title: "设置页待建", body: "深浅色主题已经能在侧边栏底部切换；更多设置项（Provider Key 管理、数据目录等）还没做。" },
};
