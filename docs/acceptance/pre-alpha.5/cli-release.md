# pre-alpha.5 CLI 补充发行

2026-10-21，用户追加 macOS ARM64 / Windows x64 现有 CLI ZIP，要求不加助手、不做无关验证。两包产品源码均为已发布的 `86da06206c1ce547921ff2fb6ada892c5b090b48`；仅新增打包脚本、独立清单与工作流，没有修改 CLI 或内核功能。

[专用构建 39539290622](https://github.com/He-RT/om-openmath/actions/runs/39539290622)两平台均成功。每个平台只执行 Release 构建、许可证打包，以及从实际 ZIP 解压后的 `--version` 和无配置求根检查：均返回 `OpenMath 0.1.0-pre-alpha.5`，`solve(x^2 - 5*x + 6 = 0, x)` 得到 2、3。没有重跑图形、模型或完整数学性能矩阵，没有新增真实模型请求。

| 资产 | bytes | SHA256 |
|---|---:|---|
| om-cli_0.1.0-pre-alpha.5_macos_arm64.zip | 12571508 | d463dc6c7da75bb66eed4c9a5ad35253d02a5fdf6c624b5a7659a931aac7fc3f |
| om-cli_0.1.0-pre-alpha.5_windows_x64.zip | 13219338 | 5738842856dc96c63c33cdcff165ec5407f2b5b89ad95cd56e6effbf35b91549 |

两个 ZIP 与 `cli-release-manifest.json` 追加在[原 `.5` Release](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)，原 tag、Mac/Web 三包及 `release-manifest.json` 保留。补充清单只覆盖这两个 CLI ZIP，含完整源码 SHA；包内另有 `BUILD.json`、使用说明、项目许可证及 Rust 依赖许可。Windows 使用静态 C 运行库；两平台无需 Node 或图形运行时，没有新增 Developer ID／Authenticode 发行签名或 Apple 公证。

打包逻辑的现有发行检查为6项通过。首轮 Windows 检出被历史记录长路径阻挡、Mac ZIP 遇到 Cargo 许可证早于1980年的时间戳，以及稀疏检出遗漏编译 Schema 的失败均保留在 Actions；已分别以限定检出、ZIP时间戳兼容及补齐Schema修复。旧失败与取消的构建不算通过，最终成功以本页指定 run 为准。

本次没有再做公开下载后的重复运行；上述运行结果来自各平台构建后实际 ZIP 的解压文件。下载与运行方式见[安装指南](../../install.md)。
