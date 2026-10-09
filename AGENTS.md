# 四象限

面向用户用简体中文。主窗口和悬浮小组件各自保持 2×2 任务矩阵；不添加用户未请求的导航、账号、同步、统计、标签或设置模块。

新增功能使用全局 feature-architecture Skill；前端视觉调整使用本机 frontend-design Skill，先审视 docs/DESIGN.md，再实现并截图复查。

架构：任务规则在 src/domain；串行保存与错误状态在 src/storage；UI 在 src/components；正式持久化仅由 src-tauri/src/storage.rs 的 SQLite 事务负责。浏览器 localStorage 只用于隔离的开发预览，不与正式数据互换。窗口关闭先提交当前编辑、等待保存队列再隐藏当前窗口，不退出应用。退出菜单必须广播到所有窗口，全部提交编辑、等待变更队列并确认后，再经 finish_exit 结束进程。任务变更只提交发生变化的字段或移动意图，由 SQLite 在单事务中合并到最新文档；快照带 revision，拒绝迟到事件。

验收命令：npm run build、npm run test、npm run test:e2e、cargo test --manifest-path src-tauri/Cargo.toml --release。打包 npm run tauri -- build。交付前验证真实 .app，区分应用重启与整机重启、Apple Silicon 构建与其他架构。

不得把截图或自动化测试的演示任务预置到正式初始数据。修改正式存储文件前先检查内容；只清理明确由本次验收创建的临时任务。
