# 四象限

面向用户用简体中文。仅保留主窗口的 2×2 任务矩阵，悬浮小组件功能已移除；不添加用户未请求的导航、账号、同步、统计、标签或设置模块。

新增功能使用全局 feature-architecture Skill；前端视觉调整使用本机 frontend-design Skill，先审视 docs/DESIGN.md，再实现并截图复查。

架构：任务规则在 src/domain；串行保存与错误状态在 src/storage；UI 在 src/components；正式持久化仅由 src-tauri/src/storage.rs 的 SQLite 事务负责。浏览器 localStorage 只用于隔离的开发预览，不与正式数据互换。窗口关闭先提交当前编辑、等待保存队列再隐藏当前窗口，不退出应用。退出菜单必须让主窗口提交编辑、等待变更队列并确认后，再经 finish_exit 结束进程。任务变更只提交发生变化的字段或移动意图，由 SQLite 在单事务中合并到最新文档；快照带 revision，拒绝迟到事件。

验收命令：npm run build、npm run test、npm run test:e2e、cargo test --manifest-path src-tauri/Cargo.toml --release。打包 npm run tauri -- build。交付前验证真实 .app，区分应用重启与整机重启、Apple Silicon 构建与其他架构。

导图入口放在主窗口右上角；单击任务选中，双击/Enter/F2 改名，选择只属于当前窗口、不写入数据库。思维导图由 `src-tauri/src/mindmaps.rs` 创建文件与唤起 Xmind，关联只经 `storage.rs` 的独立表事务保存；保持 v1 任务 JSON 兼容。新文件自动保存于 Documents/四象限导图；删除任务仅删除关联，永不删除用户导图。

CLI/MCP 适配在 src-tauri/src/automation.rs，只能通过 storage.rs 的事务写入。跨进程写入使用 IMMEDIATE 事务；读快照必须在同一读事务中读取任务与 revision。GUI 按 revision 观察外部提交并更新主窗口。连接方法、工具契约与秘书约定见 docs/CODEX.md。

本机 MCP 注册、当前聊天加载工具、常驻秘书（Your dot）保存规则与实际调用是独立验收项，不能互相替代。架构见 docs/DESIGN.md，安装、发布及未完成验收以 docs/VERIFICATION.md 为准。

不得把截图或自动化测试的演示任务预置到正式初始数据。修改正式存储文件前先检查内容；只清理明确由本次验收创建的临时任务。
