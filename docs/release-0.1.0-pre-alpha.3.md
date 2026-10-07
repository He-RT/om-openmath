# OpenMath 0.1.0-pre-alpha.3

本版按原完整科研扩展范围交付：现代组合语法、常用符号/数值计算、矩阵、ODE、优化拟合、统计/数据/单位、跨端二维绘图与参数探索，以及桌面/Web 三维场景。保留已公开的 `.1/.2`，继续在 `dev` 开发，不自动合并 `main`。

## 新增能力

- 统一小写接口与真实命名参数；管道、`fn`、范围/切片、记录/字段、矩阵 `@`。稳定函数身份、帮助/选项/能力查询、补全/Hover 与 AI 使用同一实际目录；新增别名保留用户绑定优先级。
- 常用数值/特殊函数、精确与机器线代、LU/QR/Cholesky/SVD/实对称特征、统计、正态/均匀分布、可重复随机、CSV/JSON 纯数据与 SI 单位。
- 有边界的符号积分、数值 Gauss–Kronrod 积分、极限/Taylor、笛卡尔微分、非刚性 Dormand–Prince ODE/连续插值/简单终止事件、Brent/BFGS 局部优化及可认证凸二次全局子集、QR/LM 拟合。
- 三端原生数据/数值报告、曲线/参数/隐式/区域/密度/场与流线/数据/频数图、log 轴与独立 `explore`；参数变化/取消隔离旧回复，不修改主笔记本变量。
- 共享真实 SVG/PNG/CSV/JSON 导出与 CLI 文件字节回读；Rust 生成三维曲面/参数/隐式网格、图元/样式/变换，桌面/Web 用 WebGL2 旋转、缩放、平移、复位及基础光照，CLI/桌面/Web 导出实际世界 OBJ。

[全部接口及实际边界](reference/README.md)、[现代语言](design/modern-language.md)、[科研与场景笔记本](examples/README.md)随源码交付。地月 L2 包含高精度坐标/平衡残差/位置示意；完整西瓜包含条纹表皮、皮层、切面和 12 个瓜籽，均有[真实图形及导出证据](acceptance/r36b/README.md)。

## 安装与分发

Windows x64 中文 EXE/MSI、macOS ARM64 DMG/应用 ZIP、两平台 CLI ZIP、Web 静态 ZIP、iOS 27 ARM64 模拟器应用 ZIP与两 ARM64 切片 XCFramework，仍为九类附件。源码工程随仓库交付；`release-manifest.json` 记录相同源码提交、版本、文件字节数与 SHA256。安装见[安装指南](install.md)与[移动端指南](ios.md)。

桌面包未签名/公证。iPhone/iPad 真机通过本地 Xcode 自动签名安装；个人 Team、证书和签名材料不公开，不提供通用 IPA 或商店安装承诺。最低 iOS/iPadOS 27，单窗口/单文档；本轮不增加自动 iCloud、多窗口、专用手写识别。

## 数学与平台边界

新增数值积分、ODE、优化拟合及多数矩阵/特殊函数首先提供机器精度；接受命名参数不代表实现任意精度。精确/近似、局部/全局、条件、误差估计与严格证明分别说明；失败/未收敛/奇点/超限/取消保留真实诊断，不造成功。新三维是有限采样/网格与基础光照，不是认证完整曲面或物理光线追踪。

iOS 继续完整计算与二维原生显示，三维明确提示尚未适配并保留源式，不自动生成未展示的大网格。无 WebGL2 保留源式、数据与 OBJ。Notebook Agent/事务/幂等/撤销/框架适配仍是后续预留，没有 Pi/Rig 依赖或可执行写入工具，`.omnb` v1 仍只保存源码。

## 验收与发布门禁

已于 2026-10-08（亚洲/新加坡）公开发布。发行源码 `0889d34e4fce9926054025ca71ea328f6cc65b39` 的 [CI](https://github.com/He-RT/om-openmath/actions/runs/37654889786) 与[发行流程](https://github.com/He-RT/om-openmath/actions/runs/37655070652) 全部通过；九类公开附件、清单、GitHub SHA256、实际 CLI 与 DMG 已逐项回读核验。手机/平板各 29 项单元＋5 项界面流程，发行原 53 条整次请求最大约 888/441 ms；Mac 原生 200 ms 门槛和 Windows 两种实际安装/完整三维检查通过。[完整证据](acceptance/r3-release/README.md)随仓库交付。

原 53 条数学权威语料和数学期望保持，原单项 1 秒平台门槛不放宽。本地完整 Rust 回归、纯 WASM、前端/真实生产图形/导出、两架构和无签名 iOS 测试构建已执行；最终实际结果记录在[研发账本](plan/PROGRESS.md)。GitHub 最终同提交 CI、iPhone/iPad 27 实跑、Windows 两种实际安装/中文窗口及包字节核验、macOS DMG/CLI 与九类附件门禁全部成功后，工作流才公开此预发行版。

本机不启动模拟器；云端运行不替代人工设备组合验收。用户暂缓的 VoiceOver 完整遍历、浮动键盘和真机窄窗口仍保留为未验证；已有 iPhone 真机 UI 驱动 code 74 与历史 MainActor 恢复延迟记录也保留，不声称永久消除所有平台问题。
