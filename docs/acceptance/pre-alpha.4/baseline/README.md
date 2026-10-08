# R4.0 开发基线与环境预检

[任务进度](../../../plan/PRE_ALPHA_4_PROGRESS.md) · [基线索引](index.json) · [环境](environment.json)

2026-10-09。以下是 R4.0.01/11 的实际只读检查，**不是最终 `.4` Release candidate 门禁**。

- 本地 `.3` tag 与 GitHub 公开 release 均指向 `0889d34e4fce9926054025ca71ea328f6cc65b39`；公开 pre-release 保留九产品附件及清单，记录服务返回的字节数/digest。这次没有重新下载全部历史附件，不把目录查询当字节回读。
- 原53语料与公开tag逐字节相同；索引保留整体和每条规范数据的散列、原检查方式及ID，数学期望未改。已有源式、协议/目录/Notebook与科研案例单独记录来源。`.3`发行后整理的验收记录/清单不假称在发行源码里。
- 现有 `/Applications/OpenMath.app` 仍是 Tauri `.3`；原生Preview尚未创建，没有替换当前安装。旧三个tag保持不动。
- ARM64/macOS27.0/Xcode27.0/SDK27、Rust1.94.0与Mac/iOS真机/iOS模拟器/WASM目标已复核。host Node为25.9，发行候选Node26.11.1尚未随包，两个身份分开。
- Developer ID Application 为0，Apple Development为1。未导出私钥、身份名称或Team；普通环境预检通过，要求Developer ID的预检明确失败，不能把本地开发签名当公开公证材料。

可复核命令：`gh release view v0.1.0-pre-alpha.3 --json ...`、`git rev-parse v0.1.0-pre-alpha.*`、`git show <baseline>:tests/corpus/solve.toml`、`bash macos/Scripts/verify-env.sh`、`bash macos/Scripts/verify-env.sh --require-distribution-signing`。最后一条目前预期非零，错误仅`developer_id_application_missing`；不调用Apple公证或供应商。

后续签名CI输入/临时Keychain与finally清理、现场测试范围由 [signing.example.json](../../../../macos/native-package/signing.example.json) 定义。具体签名/公证和live模型仍需其对应任务真实实施；缺材料时继续独立的数学、宿主、存储和原生UI开发。
