/**
 * 首次种子：给一个可跑的最小状态——
 * - 一个 reviewer 用户
 * - 六个 Agent（与原型一致）
 * - 两个任务（一个待认领、一个待评审）
 * - 一个 Anthropic 已连接的 Provider
 */
import { PrismaClient } from "@prisma/client";
import { encryptApiKey } from "../src/modules/providers/key-crypto.js";

const prisma = new PrismaClient();

async function main() {
  await prisma.user.upsert({
    where: { email: "reviewer@buling.local" },
    update: {},
    create: {
      email: "reviewer@buling.local",
      displayName: "评审人",
      passwordHash: "dev-only",
      roles: ["reviewer", "operator"],
    },
  });

  const agents = [
    { id: "guanhai",  name: "观海", avatar: "🧭", role: "调研分析师", status: "working", skills: ["市场调研", "竞品分析", "数据整理"], modules: ["web", "doc", "chart"], traceDelta: 6 },
    { id: "shiguang", name: "拾光", avatar: "✍️", role: "内容策划师", status: "working", skills: ["文案写作", "内容策划", "SEO优化"], modules: ["doc", "web"], traceDelta: 4 },
    { id: "mingjing", name: "明镜", avatar: "🔍", role: "代码审查官", status: "idle",    skills: ["代码审查", "安全扫描", "架构建议"], modules: ["code", "shield"], traceDelta: 0 },
    { id: "zhiyun",   name: "织云", avatar: "🎨", role: "视觉设计师", status: "offline", skills: ["UI设计", "原型制作", "品牌视觉"], modules: ["image", "doc"], traceDelta: 0 },
    { id: "xingshu",  name: "星枢", avatar: "🧩", role: "项目规划师", status: "working", skills: ["任务拆解", "进度跟踪", "资源协调"], modules: ["puzzle", "calendar"], traceDelta: 0 },
    { id: "chengsi",  name: "澄思", avatar: "📊", role: "数据分析师", status: "idle",    skills: ["数据建模", "可视化", "统计分析"], modules: ["chart", "doc"], traceDelta: -6, recentRejects: 2 },
  ] as const;

  for (const a of agents) {
    await prisma.agent.upsert({
      where: { id: a.id },
      update: {},
      create: {
        id: a.id, name: a.name, avatar: a.avatar, role: a.role,
        status: a.status as any,
        bio: `${a.name} · ${a.role}`,
        skills: [...a.skills], modules: [...a.modules],
        collabPref: "auto",
        modelKey: null,
        effort: "medium",
        permissions: { confirmCode: false, confirmExternal: false, budget: null },
        traceDelta: a.traceDelta,
        recentRejects: (a as any).recentRejects ?? 0,
        stats: { tasksDone: 20, avgScore: 4.8, collabs: 12 },
      },
    });
  }

  await prisma.task.upsert({
    where: { id: "t-open-1" },
    update: {},
    create: {
      id: "t-open-1",
      title: "Q3 消费电子品类竞品分析报告",
      description: "覆盖3家核心竞品的定价策略、渠道打法与用户评价，产出可执行的对标建议。",
      status: "open", priority: "high",
      requiredSkills: ["市场调研", "数据整理"],
      assignees: [],
      due: "8月29日截止",
      openHours: 3,
      plan: [],
    },
  });

  const reviewTask = await prisma.task.upsert({
    where: { id: "t-review-1" },
    update: {},
    create: {
      id: "t-review-1",
      title: "支付模块代码安全审查",
      description: "针对新版收银台代码进行安全扫描与架构合规性审查。",
      status: "review", priority: "high",
      requiredSkills: ["代码审查", "安全扫描"],
      assignees: ["mingjing"],
      due: "8月25日截止",
      progressPct: 90,
      plan: [
        { title: "静态代码扫描", status: "done", agentId: "mingjing" },
        { title: "关键路径人工复核", status: "done", agentId: "mingjing" },
        { title: "输出审查报告初稿", status: "done", agentId: "mingjing" },
        { title: "等待人工评审确认", status: "active", agentId: null },
      ],
    },
  });

  await prisma.doc.upsert({
    where: { id: "d-security-1" },
    update: {},
    create: {
      id: "d-security-1",
      title: "支付模块安全审查报告（初稿）.pdf",
      type: "pdf",
      taskId: reviewTask.id,
      agentId: "mingjing",
      preview: "发现 2 处中危漏洞与 5 处代码异味，附修复优先级与建议改动位置。",
      reviewStatus: "pending",
    },
  });

  await prisma.team.upsert({
    where: { id: "team-launch" },
    update: {},
    create: {
      id: "team-launch",
      name: "新品发布协作组",
      description: "负责发布节奏拆解、文案与视觉的一并交付",
      ownerAgentId: "xingshu",
      memberAgentIds: ["xingshu", "shiguang", "zhiyun"],
    },
  });
  await prisma.team.upsert({
    where: { id: "team-quality" },
    update: {},
    create: {
      id: "team-quality",
      name: "代码质量小组",
      description: "代码审查 · 安全扫描 · 数据合规",
      ownerAgentId: "mingjing",
      memberAgentIds: ["mingjing", "chengsi"],
    },
  });

  const key = encryptApiKey("sk-ant-dev-only-example-token-1234");
  await prisma.provider.upsert({
    where: { id: "prov-anthropic" },
    update: {},
    create: {
      id: "prov-anthropic",
      name: "Anthropic 官方",
      type: "anthropic",
      baseUrl: "https://api.anthropic.com",
      apiKeyEnc: key.enc, apiKeyIv: key.iv, apiKeyTag: key.tag, apiKeyLast4: key.last4,
      enabled: true, status: "connected",
      models: [
        { id: "haiku", label: "Claude Haiku 4.5", hint: "轻量快速", ctx: "200K" },
        { id: "sonnet", label: "Claude Sonnet 5", hint: "均衡通用", ctx: "200K" },
        { id: "opus", label: "Claude Opus 5", hint: "深度推理", ctx: "200K" },
      ],
    },
  });

  console.log("[seed] done");
}

main()
  .catch((e) => { console.error(e); process.exit(1); })
  .finally(async () => { await prisma.$disconnect(); });
