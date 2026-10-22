# Git 管理约定与 2026-10-22 检查

当前在 `dev` 开发，普通提交推送 `origin/dev`；不重写已有公开历史、移动发行标签或把开发改动直接并入 `main`。发布版本的二进制与清单放 GitHub Release，源代码仓库保存构建入口、锁文件、文档和必要证据。

## 本次检查结论

以 `b44ba56b` 为基线检查 Git 索引、忽略规则、文件大小、分支与上游同步状态：

- 已跟踪 24,539 个文件，工作树对应约 150.2 MiB；其中 `docs/` 为 22,170 个文件、约 132.5 MiB，占体积约 88.2%。Git 已打包对象约 81.0 MiB。
- 没有发现已跟踪的 `target/`、`node_modules/`、Web 构建目录、Xcode 用户状态、Gradle 缓存或安装包。现有目录级构建输出隔离基本合理。
- 原规则只覆盖 `app/test-results/`、`app/playwright-report/`，漏了仓库根目录等位置；本机确有 42 个非忽略的根目录测试输出，以及一个个人 VS Code 设置文件。
- 没有统一排除个人 IDE 配置、常见工具缓存、局部环境配置与私有签名材料；发行流程使用的根目录 `release-assets/`、`package-evidence/` 也未被忽略；本次补齐。
- 验收目录保留了失败日志、压缩源码快照、数据库/WAL/SHM 和 Xcode 失败结果。这些属于已记录的证据，不能当作通用构建缓存删掉或取消跟踪。

本次不做历史清洗、重新压缩旧证据、LFS 迁移或 Git GC。已有大文件均低于 GitHub 的单文件限制；目前的主要改进是补齐本地产物隔离，并约束后续归档方式。

## 应当提交

- 产品源码、测试及其必要输入、脚本、平台配置、图标和第三方许可。
- `Cargo.lock`、npm lockfile、Swift `Package.resolved` 等依赖锁；不要添加通用 `*.lock` 忽略规则。
- 已约定检查生成一致性的文件，例如 `app/src/kernel/generated/`、函数文档目录和原生 Xcode 工程。
- 可评审的设计源稿、说明和选定截图。本次 iOS/iPadOS 草稿 01、02 的 HTML / SwiftUI / 截图属于项目文档；其编译出的 `.app` 留在 `target/`。
- 能支撑验收结论的记录、失败证据、必要样例与散列清单。

`app/src-tauri/gen/` 与上述生成源码不同：Android 工程由工具初始化并由 `scripts/prepare_android.py` 配置，继续忽略整个目录。

## 应当忽略并保留在本地

[根 `.gitignore`](../.gitignore) 统一覆盖：

- 构建与依赖：`target/`、`node_modules/`、Web/WASM 产物、Tauri 生成工程、Swift/Gradle/Kotlin 构建缓存、DerivedData，以及根目录 `release-assets/`、`package-evidence/`。
- 临时报告：任意项目层级的 `test-results/`、`playwright-report/`、`blob-report/`、覆盖率目录。
- 个人状态：根目录 `.vscode/`、`.idea/`、个人 workspace、Xcode `xcuserdata` 与编辑器交换文件。
- 工具缓存：Python、Vite、ESLint、TypeScript 增量缓存。
- 本地配置与签名材料：`.env` / `.env.*`、原有 `Local.xcconfig`、签名密钥库与描述文件。`.env.example` / `.env.*.example` 可提交无凭据模板。

新增忽略规则不会删除磁盘文件，也不会自动停止跟踪已有文件。不使用 `git add .` 或 `git rm -r --cached .` 批量刷新索引；按本次任务的明确路径添加，必要的历史证据保持原样。

不要用全局 `*.log`、`*.gz`、`*.obj`、`*.sqlite*` 忽略代替目录管理：日志可能是失败证据，OBJ 是模型源数据，数据库及 WAL 可能是恢复验证输入。

## 后续工作流程

1. 临时构建、完整测试输出、下载和诊断过程先写入 `target/<任务>/`。从中挑选需要长期保存的摘要、截图、必要失败输入与散列清单放到 `docs/acceptance/`。
2. 新增大量原始快照时，优先使用单个归档加 SHA256 / 文件清单，避免把整套运行根和依赖缓存拆成成千上万个文件入库。已冻结档案不迁移、不改散列。
3. 提交前检查 `git status --short`、`git diff --cached --stat`、`git diff --cached --check`；确认无构建输出和机器配置。
4. 用 `git check-ignore -v --no-index <路径>` 解释规则，用 `git ls-files -ci --exclude-standard` 查找已经跟踪却又命中忽略的文件。遇到后一种情况先核对用途，不能据此自动删除证据。
5. 提交按用途分开。文档与忽略规则变更只做相关的轻量检查；与生产构建无关时可按现有约定使用 `[skip ci]`，不因此重跑发行包或真实模型调用。
6. 推送前获取上游状态，正常快进推送 `dev`。忽略规则整理不需要强推、移动标签或改写历史。

`.gitignore` 只控制未跟踪文件的默认可见性，不能保证已提交内容不含敏感信息。提交模板、配置或新增证据时，仍需核对具体文件内容。
