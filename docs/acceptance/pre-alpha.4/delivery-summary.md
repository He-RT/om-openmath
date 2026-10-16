# .4 Mac / Web 交付记录

候选：`e1d6afd11a48dec807b0c6cce09938adaeeffdd6`，版本 `0.1.0-pre-alpha.4`。只有 scope 2 的8项交付检查决定本版完成；旧98任务/32门禁不改成已完成。

**2026-10-16 用户现场复核更正：** .4公开发布、下载哈希、安装和既有数学/性能事实成立；不能据此宣称完整产品设计已完成。用户截图暴露代码块破碎、助手交互偏离设计，以及emoji写入被当作绘图目标完成。R4D04/R4D05改为failed，ready/released_verified撤回；不移动公开tag或抹去先前测试结果。具体原因见[用户现场复核](../../plan/PRE_ALPHA_4_PROGRESS.md#d074--2026-10-16-用户现场复核纠正完成声明)。

| 检查 | 实际结论与证据入口 |
| --- | --- |
| R4D01 | 同源 Release 原生 Mac、自带 Node 26.11.1 / Pi / 字体；DMG 安装后严格签名通过，程序关于面板读回版本/完整 SHA。生产 Web ZIP 解压1310文件逐字节一致，浏览器读回版本、源码和实际 Rust 版本。 |
| R4D02 | 原53输入及独立高精度期望未改；CLI、Mac、生产 WASM 输出全部交给优化前保留并核 SHA 的独立验证器通过。生产 Web 原有科学/语法/文件兼容检查通过。 |
| R4D03 | 安装版实际新建、中文粘贴、菜单/快捷键撤销重做、精确解2/3；运行中点击停止显示“已停止”，同格随后2+2得4。保存后退出，确认进程终止；再启动并打开自己的 `.omnb` v1，源码完整且未自动运行。Web 35项生产检查通过。 |
| R4D04（现场未通过） | 先前记录：复用未变的原生 Markdown/SwiftMath/表格/步骤、CoreGraphics2D、Metal3D 实窗验收；当前安装包再次实际显示精确公式。Web35项包含原有绘图及导出。应用参考与采用决定集中在 `docs/plan/UI_REFERENCE_LOG.md`。 |
| R4D05（现场未通过） | 先前记录：当前安装资源及真实 NativeAssistant/ProviderTransport/Rust/Pi 完成授权 gpt-6.1-sol 任务：read→read→preview→apply→read→run→inspect。原 Blob 核 SHA 后独立读回当前已接纳 x→2/3；相关真实 Pi/协议23项通过，复用原确定性工具/停止/授权回归。Web模型与CORS入口检查保留。 |
| R4D06 | 原53各运行时全过原阈值，见下表。Mac 为正常 AppKit 前台事件循环，逐条/逐次确认 active app 和 key window；不是选取最快的重复测量。 |
| R4D07 | 当前 Release 对象回归验证早停回滚、晚停保留已提交前缀、丢ACK、真实未知提交按原ID核对及长计算期间编辑；未重算或换ID。复用有效保存/恢复/权限回归。当前候选5603个文件的授权密钥精确扫描为0处匹配。 |
| R4D08 | DMG、app ZIP、Web ZIP及manifest已生成；DMG实际安装和Web解压运行通过。中文指南 `docs/release-0.1.0-pre-alpha.4.md`。公开tag核对e1d；三个公开资产及manifest匿名下载核字节/SHA通过。公开DMG另装新目录并确认实际运行路径，About读回.4/完整e1d，打开自身中文笔记本并实际得到2/3/4。公开appZIP解压4224文件与DMG安装逐一一致；公开Web解压实际HTTP启动、计算得精确−3/1，设置读回.4/Rust .4/完整e1d。 |

| 实测 | 结果 | 原条件 |
| --- | --- | --- |
| CLI 原53最长 | 166.721292ms | 每条 <200ms |
| Mac submit→MainActor接纳整次最长 | 813.039833ms | 每条 <1000ms |
| Web WASM / worker往返最长 | 229 / 231ms | WASM每条 <1000ms |
| Mac 10KiB100次输入 | p95 49.620792ms；max 83.137458ms | p95 ≤50ms；计至第二显示同步帧 |
| Web 10KiB100次输入 | p95 31.600000ms；max 51.700000ms | p95 ≤50ms；计至第二rAF |
| 原生普通相机 | 7936三角形，181真实drawable / 3.01659秒≈60Hz；静止0帧 | M5，1576×760；渲染代码未变 |

首工作区帧909.1125ms、测量进程峰值RSS188547072字节，未称整个发行应用冷启动；Web FCP148ms，实际后端SwiftShader Vulkan，未称硬件WebGL验收。旧超标/未完成测量全部保留，不改原精度、正确性或性能门槛。

原始日志保留本地可复核目录，不复制数据库或源码归档：

- `target/pre-alpha.4/e1d6afd11a48dec807b0c6cce09938adaeeffdd6/`：三资产、安装记录、Web35项/原53、真实助手Blob读回和密钥精确计数。
- `target/plan-scope2/root-proposals-cli/`、`target/macos-performance/foreground-appkit-12jjx46n/`：CLI与Mac原53/输入及独立正确性。
- `target/plan-scope2/current-kernel-regression.txt`：当前真实内核运行回归入口。

已知限制：Mac ad hoc签名、未Developer ID签名/公证；机器既有security-disabled override不能证明标准Gatekeeper通过。旧完整Pi fixture尾端曾出现StorageError.inUse，保留原失败；七类撤销独立通过，50次关闭/立即重开未复现，未放宽存储锁或将失败冒记通过。高级媒体/长会话/全供应商协议/极端网格仍后置。

公开下载的原始文件、哈希记录和UI读回位于当前候选根的 `public-download/`；未将标准Gatekeeper/公证冒记为通过。候选[CI38906287178](https://github.com/He-RT/om-openmath/actions/runs/38906287178)已全部成功，iOS按scope跳过。已删除本轮比较用、无进程使用的appZIP临时解压副本235579847字节，保留原下载包和证明；停止被公开Web服务替代的旧4174服务，当前4175展示实际公开包。
