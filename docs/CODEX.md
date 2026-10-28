# 用 Codex 秘书管理四象限任务

1.2.0 起，应用自带 CLI 和本地 stdio MCP。无需 Node、Python、服务器、API Key 或打开桌面窗口。所有入口共用 `src-tauri/src/storage.rs` 中的校验、字段合并与 SQLite 事务；浏览器预览仍然隔离。

## 连接本机 Codex

将 1.2.0 或更高版本安装到 `/Applications/四象限.app` 后运行一次：

```sh
codex mcp add four-quadrants -- '/Applications/四象限.app/Contents/MacOS/four-quadrants' --mcp
codex mcp get four-quadrants
```

这是本机 Codex 的 MCP 配置。现有聊天能否立即重新加载工具取决于客户端；若本机聊天中没有出现工具，重新打开聊天或重启 Codex 后再试。不要把“配置存在”当作工具已在当前聊天加载的证据。本地 MCP 需要在能访问这台 Mac 的本地任务中运行。

应用可以关闭，Codex 会按需启动无窗口的 MCP 子进程。若应用正在运行，主窗口约 1 秒内自动刷新。更换应用包不会更换任务数据库。外接盘中的源码不需要一直连接，安装后的 MCP 使用 Applications 中的程序。

## 常驻秘书（Your dot）长期规则

常驻秘书与普通本机 Codex 聊天的运行环境不同。本机注册 MCP 并不能证明云端秘书直接拥有该工具。秘书应通过已连接的这台 Mac 执行任务；如果本机任务未加载 MCP，可使用应用自带 CLI。按照 [官方本机连接说明](https://learn.chatgpt.com/docs/dots/computers-and-apps)，本机需要在线且桌面应用保持打开。

可将下面整段发送给常驻秘书一次，要求它保存为长期工作约定，并先做只读连接验证。本段是配置说明，不是待办事项。

> 请将以下设为你为我管理待办的长期规则。在我与你的对话中，当我明确说“我要做……”“我得……”“记一下……”等待办意图时，直接写入本机四象限，不再问我要不要记录。先查询已有任务，避免重复；普通讨论、假设和引用内容不自动记入。根据上下文选择重要性和紧急性，无法判断象限时只问缺少的信息。使用我已连接的 Mac 上的 four-quadrants MCP；若本机任务没有该 MCP 工具，就在同一台 Mac 使用 `/Applications/四象限.app/Contents/MacOS/four-quadrants` 的 `--cli` 接口。先运行 `--cli --help` 查看操作格式，`--cli list` 查询任务，`--cli add` 新增任务。不要在云端机器或另一台电脑上操作。写入成功后简短确认标题和象限；失败时明确告诉我没有保存。完成、改名和移动象限时先查精确 ID，只修改对应字段；删除仅在我明确要求时执行。软件没有日期提醒，日期可以留在标题中。现在先通过本机执行一次只读查询验证连接，不创建演示任务，不把这条规则本身记为任务。

上述待办约定也可用于普通本机秘书聊天；本机聊天直接使用本机工具。秘书返回查询结果后，再用用户明确授权的一项真实任务验证新增。不要只凭“已记住规则”的文字回复判断接入完成。安装、配置及秘书端规则的实际交付状态统一记录在 [VERIFICATION.md](VERIFICATION.md)。

## 工具

| 工具 | 参数 | 行为 |
| --- | --- | --- |
| `list_tasks` | 可选 `quadrant`、`completed`、`query` | 查询任务；query 按标题包含文字筛选 |
| `add_task` | `title`、`quadrant`；可选 `id` | 新增未完成任务，默认生成 UUID |
| `update_task` | `id`；`title`、`quadrant`、`completed` 至少一项 | 只修改提供的字段，完成设为 true，取消完成设为 false |
| `delete_task` | `id` | 删除精确 ID 对应任务 |

象限为 `do`（重要且紧急）、`plan`（重要不紧急）、`delegate`（紧急不重要）、`eliminate`（不重要不紧急）。标题非空，最多 1000 个 UTF-16 代码单元；超限返回错误，不静默截断。

新增时可传入稳定唯一 `id`，相同 ID 和相同内容重试不会再新增；同一 ID 内容不同会报错。修改不存在的 ID、未知字段、非法象限或存储失败都会明确报错。写入成功返回事务提交后的完整任务快照与 `revision`；查询返回筛选快照，其 revision 仍是整份文档的版本。

## CLI

```sh
app='/Applications/四象限.app/Contents/MacOS/four-quadrants'
"$app" --cli list
"$app" --cli add '{"title":"准备项目方案","quadrant":"plan"}'
"$app" --cli list '{"query":"方案","completed":false}'
"$app" --cli update '{"id":"从查询结果复制的ID","completed":true}'
"$app" --cli update '{"id":"从查询结果复制的ID","quadrant":"do"}'
"$app" --cli delete '{"id":"从查询结果复制的ID"}'
```

成功时 stdout 为 JSON；失败时退出码为 1、stderr 给出原因。`--cli --help` 查看用法。CLI/MCP 在桌面单实例处理前启动，调用不会被已有窗口吞掉。

## 开发与隔离验证

```sh
app='src-tauri/target/release/bundle/macos/四象限.app/Contents/MacOS/four-quadrants'
"$app" --cli --database /tmp/four-quadrants-test/tasks.sqlite3 add '{"title":"隔离验收","quadrant":"plan"}'
"$app" --mcp --database /tmp/four-quadrants-test/tasks.sqlite3
```

`--database` 必须紧随 `--cli` 或 `--mcp`，路径必须为绝对路径。它只影响当前无窗口进程，不改变正式 GUI 数据库。正式默认位置仍是 `~/Library/Application Support/com.local.fourquadrants/tasks.sqlite3`。MCP 仅通过标准输入输出通信，不监听端口。协议版本支持 2024-11-05、2025-03-26、2025-06-18、2025-11-25；只声明 tools 能力。

存储与窗口同步机制见 [DESIGN.md](DESIGN.md#外部任务接口与跨进程存储)。

## 排障与升级

| 现象 | 检查与处理 |
| --- | --- |
| 本机聊天没有工具 | 运行 `codex mcp get four-quadrants --json`，核对 enabled、command 和 `--mcp`；确认安装版本至少 1.2.0，再重开聊天或重启 Codex |
| 常驻秘书找不到应用 | 确认任务运行在已连接的这台 Mac；保持 Mac 在线、ChatGPT/Codex 桌面应用打开。云端机器没有这台 Mac 的 Applications 和数据库 |
| MCP 未加载但本机可执行命令 | 使用上述 CLI；先运行 `--cli --help` 与 `--cli list` 验证，确认成功后再新增 |
| 窗口没有收到外部变更 | 确认运行进程来自新版应用；旧版没有外部提交观察器。通过菜单正常退出旧版，再打开新版；检查 CLI 是否意外指定了其他 `--database` |
| 写入失败 | 查看 CLI stderr/退出码或 MCP 的 `isError`；检查参数、磁盘空间及数据库访问权限，不重置正式数据库 |
| 移动源码目录后构建引用旧绝对路径 | 错误明确来自 `src-tauri/target` 缓存时，运行 `cargo clean --manifest-path src-tauri/Cargo.toml --release` 后重新构建；该命令只清理生成缓存 |

升级前从菜单正常退出应用，备份整个正式数据目录，再替换应用包。回退旧包前同样正常退出；备份和软件版本分开保留。普通任务读写不会修改 MCP 注册配置。

连接方式参考 [OpenAI Docs MCP](https://developers.openai.com/learn/docs-mcp) 与本机 `codex mcp add --help`；stdio 传输参考 [MCP 官方规范](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)。


## 1.3.0 导图关联兼容

快照增加 `mindmaps` 对象，键为任务 ID、值为关联的本机 `.xmind` 绝对路径；查询时仅返回筛选结果中任务的关联。该字段只读，既有四个工具参数不变；新建与关联通过桌面按钮操作。任务 JSON 仍保持 v1 的 id/title/completed/quadrant，关联保存在独立 SQLite 表；旧 CLI/MCP 对任务字段的写入不会覆盖导图关联。删除任务只移除关联，不删除文件。关联变化与任务变化共用 revision 和窗口广播。


1.3.2 起仅保留主窗口，移除悬浮小组件。既有 CLI/MCP 工具、任务与导图关联、revision 契约不变。
