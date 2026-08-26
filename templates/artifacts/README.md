# templates/artifacts/ · Artifact Markdown 骨架模板

每份模板对应一个 `Artifact.kind`。Agent 产出对应 Artifact 时，Runtime 会**先复制模板到目标路径**，再让 Agent 按模板填内容。这保证：

1. **跨项目一致格式** —— 用户第一次打开某类文档就知道去哪找哪一段
2. **下游可预测解析** —— 后续 Agent 读上游 Artifact 时能定位到具体章节（`## 2. 用户画像`）而不是靠语义猜
3. **审阅红线可锚定** —— 评审面板可以按章节 diff、按章节标注意见

## 目录布局

```
templates/artifacts/
├── README.md                 (本文)
├── common/
│   └── Report.md             每个 task_run 强制产出的执行汇报
├── product/                  产品经理产出
│   ├── PRD.md
│   ├── Acceptance-Criteria.md
│   └── User-Stories.md
├── technical/                技术负责人 + 架构师产出
│   ├── Technology-Stack.md
│   ├── Technical-Risk.md
│   ├── Architecture.md
│   ├── Database-Design.md
│   ├── API-Spec.md
│   ├── Security.md
│   └── Deployment.md
├── project/                  项目经理产出
│   ├── Project-Plan.md
│   ├── Task-List.md
│   ├── Milestone.md
│   └── Risk.md
├── design/                   设计师产出
│   ├── UX-Overview.md
│   ├── Page-Structure.md
│   ├── Interaction.md
│   └── Visual-Tokens.md
├── qa/                       测试产出
│   ├── Test-Plan.md
│   ├── Test-Cases.md
│   ├── Test-Report.md
│   ├── Bug-Report.md
│   └── Regression-Report.md
└── acceptance/               验收产出
    └── Acceptance-Report.md
```

## 模板文件规范

**顶层 YAML frontmatter**（必需，Runtime 会填 `producer` / `created_at` / `project` 三项）：

```yaml
---
kind: <artifact-kind>           # 与 templates/standard-software-delivery.yaml 里的 kind 对齐
version: 1                      # 模板版本，改结构时递增
producer: <role-name>           # 由哪个岗位产出
inputs:                         # 期望的上游依赖
  - <artifact-kind-or-path>
project: <auto-fill>
created_at: <auto-fill-iso>
---
```

**填写指南**放 HTML 注释 `<!-- ... -->` 里：不渲染，但 Agent 能看到；给 Agent 明确的"填什么、不填什么"约束。

**段落编号**：`## 1.` / `## 2.` 而非 `## `，方便下游按编号定位。

**占位符**：用 `[方括号]` 包裹待 Agent 替换的位置，评审时肉眼一扫就知道哪块没填。

## 修改守则

1. **模板改结构 = 破坏性变更**：影响所有既有 Artifact 的解析；改前必须走 Design-First 六步
2. **模板加章节**（软兼容）：`version` 递增，Runtime 用最新模板生成新 Artifact，历史 Artifact 不动
3. **写 Agent 系统提示时**：告诉它"你的 PRD 严格按 `templates/artifacts/product/PRD.md` 骨架填"，别自由发挥章节
4. **不要在模板里放示例内容**：只放骨架 + 填写指南；示例放 `examples/` 目录（P1）

## 关联文档

- [`OPC-产品定义.md § 5/6`](../../docs/OPC-产品定义.md) —— Agent 定义与 Artifact 追溯图
- [`agents/*.yaml`](../../agents/) —— 每个 Agent 的 `expected_outputs` 声明用哪些模板
- [`standard-software-delivery.yaml`](../standard-software-delivery.yaml) —— 工作流引用这些 Artifact
