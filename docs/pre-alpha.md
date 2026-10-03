# 首个 pre-alpha 验收记录

[English](pre-alpha.en.md) · [发行说明](release-0.1.0-pre-alpha.1.md)

M0–M13 已实现共享 CAS/内核、终端、原生及生产 WASM 笔记本、可配置 AI、源码文件和导出。本地首轮验收对应 `a40d325`；最终 CI 选择器修复 `84d9daf` 的 [CI 37126375546](https://github.com/He-RT/om-openmath/actions/runs/37126375546) 全部通过。下表记录此前已完成证据，发行构建在同一版本提交上重新执行门禁。

## 已完成的本地门禁

| 项目 | 证据 |
|---|---|
| Rust | fmt/Clippy 通过；878 工作区测试、3 仓库策略测试通过 |
| 编辑/构造延迟 | 1000 字以内 warm preview <5 ms（最大 0.564 ms）；1000 项加法 <10 ms |
| 纯 WASM | kernel 编译到 wasm32；14 纯协议测试通过 |
| 依赖 | locked cargo-deny 通过；无新第三方运行依赖 |
| 协议 | 56 个生成 TypeScript 文件无漂移 |
| 前端 | Node 26 lint/typecheck/build，59 单元测试通过 |
| 开发浏览器 | 29 项通过；生产性能项在开发套件跳过 |
| 真实静态 dist | 27 界面测试及 53 行性能语料通过 |
| 原生语料/性能 | 53 个冷启动 om 进程匹配原始精确/700-bit 权威检查，单项 <200 ms |
| WASM 语料/性能 | 53 个冷启动生产 Worker 经独立原生验证，单项 <1 s |
| 原生包 | Apple Silicon app/DMG 构建，DMG 校验通过 |
| 凭据 | tracked 凭据形状扫描 0；测试仅使用合成服务 |

正常 Rust 套件有两项显式 opt-in 忽略：早期性能项和需构建产物的生产 WASM 权威 runner。原生 release gate 只在 release 编译，实际验收显式执行新增的两个权威/性能 runner。

## 性能与数学证据

原生 53 行最终最大 119.914 ms，生产 WASM 最大 145 ms。这是开发机 M 系列 Mac 的本地测量，不是跨硬件延迟保证。每个案例冷启动进程/Worker，关闭自动图像，保留默认验证/步骤。逐行数据和未变的语料 digest 见 [pre-alpha-results.json](pre-alpha-results.json)。

初次 quintic/cyclic-3 分别为 1256/5581 ms。profile 定位重复根隔离、代数坐标/投影及复数整数幂溢出检查中的无用辐角。修复使用有界成功证书缓存、每次新取消/期限检查、私有精确展开以及精确零/一/相同复值捷径；CBall.log_magnitude 保留真实包围及原点/范围拒绝。另有回归验证精确恢复的 1 不再显示伪微小虚部。

53 条数学期望、原始限制和分支未改。内联审查覆盖次数、精度、中断、构造证据、根顺序、极点、步骤、只读状态及 unwrap/unsafe。

## 真实原生与生产 UI

此前实际 OpenMath.app 窗口经 CUA 验证：x²+2x=3 的精确 −3/1、两交点、实时预览和真实因式分解/零乘积步骤；a=2→5 时 a+1 从 3→6。最终 app 用原生文件选择器打开保存的源码笔记本并重新计算。

独立配置和合成 loopback 服务验证真实 probe（pong、首字节/总耗时）、讲解流、解析后的 Ask 及显式运行 ±2、确认后的 ghost/Tab 只插入源码。真实终端 PTY 验证着色、In/Out、多行和精确结果。生产 WASM UI 验证同样模型设置、隐私、图像、中断/恢复、中英文及浅深主题。

原生菜单保存源码 `.omnb`、导出当前结果 Markdown/LaTeX；独立 TeX 经内置编译器成功编译。单元测试验证转义、动态 Markdown fence 和过期结果排除。文件不含配置、凭据、对话或输出缓存。

此前授权 DeepSeek 测试通过共享 kernel 的 probe/翻译/FIM/chat；自动化和打包验收只使用合成提供商，未保存真实密钥。

## Windows 与 GitHub 发行门禁

版本改为 `0.1.0-pre-alpha.1`，同源 Windows EXE/MSI、macOS app/DMG/CLI、Web 包由[发布工作流](../.github/workflows/release.yml)生成。Windows 验收分别安装两种包，Playwright 通过仅 CI 开启的 WebView2 CDP 连接真实 Tauri 窗口，检查中文、精确根、步骤及响应式，再卸载。全部 job 和同一 SHA 的 CI 成功后才公开预发行版；不能用本地 macOS 测试冒充 Windows 证据。实际运行与截图见该工作流的 Actions 记录及 `windows-install-evidence` artifact。

## 复现

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_PROFILE_TEST_OPT_LEVEL=2 cargo test --workspace --locked
python3 -m unittest discover -s tests -v
cargo build -p om-kernel --no-default-features --locked --target wasm32-unknown-unknown
cargo deny --locked check
cargo test -p om-kernel --no-default-features --test protocol --locked
cargo test -p om-cli --release --test corpus all_authority_rows_pass_native_release --locked -- --ignored
cd app
npm run lint && npm test && npm run build
OPENMATH_E2E_PREVIEW=1 OPENMATH_PERFORMANCE=1 npm run test:e2e -- --workers=1
npm run test:e2e -- --workers=1
cd ..
cargo test -p om-cli --release --test corpus production_wasm_results_match --locked -- --ignored
```

生产性能项写 `target/pre-alpha/wasm-corpus.json`，供独立原始权威验证。需要 Node 26、wasm-bindgen 0.2.129、Python 3.11+；优化测试仍保留 debug/溢出检查。

当前 macOS/Windows 包未签名，macOS 未公证；Linux 安装包不在本次发行范围。各分发包保留锁定依赖的许可证和证书数据许可。Web 模型需 CORS，Unicode TeX 可能需要相应字体/引擎。边界见[求解指南](solve.md)，下载与校验见[安装指南](install.md)。
