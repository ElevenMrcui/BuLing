# runtime/migrations/ · SQLite Migration

不令 OPC 用**双层 SQLite**：APP 级一份、PROJECT 级每个项目一份。migration 也按层分目录。

```
migrations/
├── app/           # ~/.opc/db.sqlite 用
│   └── 0001_init.sql
└── project/       # <project>/.opc/project.sqlite 用
    └── 0001_init.sql
```

## 命名规范

- 文件名：`NNNN_短名.sql`（4 位序号 + 下划线 + kebab-case 描述）
- 例：`0002_add-mcp-servers.sql`, `0003_split-skills-table.sql`
- 序号严格递增，不允许跳号也不允许倒序
- 一份 migration 一件事（原子）；改多个东西写多份

## 执行合约

- 每份 migration 必须**幂等安全**：跑第二次不应 error（但可以是 no-op）
- 必须在最后 `INSERT INTO _migrations (version, name) VALUES (N, 'NNNN_...')`
- Runtime 启动时按序号从小到大跑；`SELECT MAX(version) FROM _migrations` 判断进度

## 硬约束（append-only 表）

以下表**只允许 INSERT**，应用层禁止 UPDATE / DELETE（审计要求）：

- `execution_logs`（app 与 project 两个库都有）
- `artifact_versions`
- `reviews`

改这些表的 schema 时（加列、加索引）可以，但**改字段语义 / 删表**必须走一次数据迁移设计评审。

## 本地手工验证

装了 `sqlite3` CLI 的话：

```bash
sqlite3 /tmp/opc-app-test.db < runtime/migrations/app/0001_init.sql
sqlite3 /tmp/opc-app-test.db "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;"

sqlite3 /tmp/opc-project-test.db < runtime/migrations/project/0001_init.sql
sqlite3 /tmp/opc-project-test.db "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;"
```

没装 sqlite3 CLI 的话（比如 CI 沙箱），可以用 `better-sqlite3`：

```bash
cd /tmp && npm i better-sqlite3
node -e "
const Db = require('better-sqlite3');
const fs = require('fs');
const db = new Db(':memory:');
db.exec(fs.readFileSync('runtime/migrations/app/0001_init.sql','utf8'));
console.log(db.prepare(\"SELECT name FROM sqlite_master WHERE type='table' ORDER BY name\").all());
"
```

**每份新 migration 必须先手工跑一次确认无 error 再提交**。
