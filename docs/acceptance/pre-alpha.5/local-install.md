# pre-alpha.5 安装验收与发布记录

2026-10-22 追加的 Tauri 2 Windows MSI / Android SDK 35 APK 使用独立源码与打包清单，见[Tauri 补包记录](tauri-release.md)；下文仍为原 Mac/Web 三包记录。

2026-10-21。发行源码与包内 revision：`86da06206c1ce547921ff2fb6ada892c5b090b48`。三个包已本地验收，[同SHA的CI](https://github.com/He-RT/om-openmath/actions/runs/39533915333)全部通过（Rust、frontend、dependencies、native；iOS按本版范围跳过）。用户明确回复“允许公开发布”后，已创建[公开 GitHub pre-release](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.5)，发布时间 2026-10-21 11:03:20（Asia/Singapore），三个包及 manifest 均已上传。

资产位于 `target/pre-alpha.5/86da06206c1ce547921ff2fb6ada892c5b090b48/release-assets/`。`release-manifest.json` SHA256：`de3823bcf07421131353634f221bb8ed7d2a1fe0538a3eccd9716f1f37dff26f`。

| 资产 | bytes | SHA256 |
|---|---:|---|
| OpenMath_0.1.0-pre-alpha.5_aarch64.dmg | 110205334 | 9e58b03c0edc27b76cc10167d1fc506dc373da295d5ed50aba2727de1eca7848 |
| OpenMath_0.1.0-pre-alpha.5_macos_arm64.app.zip | 81916505 | 84a8c313f4e45d465bce38ee0b340575f3543a87ed4dcf72cc6d10a1a5685326 |
| OpenMath-web_0.1.0-pre-alpha.5.zip | 12241163 | 40715e9a20263a8ba5148462ce2ce9373c31ee9702ad14d6af0eed8fda7a3e52 |

DMG只读挂载并安装到 `/Users/hert/Applications/OpenMath pre-alpha.5 DMG 86da0620/`；app ZIP解压后安装到 `/Users/hert/Applications/OpenMath pre-alpha.5 ZIP 86da0620/`。两个未修改应用的4226个文件逐字节相同，严格codesign验证通过，无OpenMathFixture键。签名为ad hoc，未公证。

通过Finder“打开方式→其他”分别选择确切安装应用，未修改默认关联。两次均先核对没有发行应用进程，再由Finder启动正确路径，实际读入示例副本的四格源码；打开不自动计算。依次运行西瓜定义、scene和新增的`solve(x^2 - 5*x + 6 = 0, x)`，两包均显示真实三维西瓜和已代入验证的2/3。ZIP保存并退出后，DMG读回同一文件并重新计算通过；两者关于窗口均显示.5和上述完整SHA，最终正常退出。

Web ZIP真实解压目录通过自有`127.0.0.1:4180`提供HTTP；浏览器导入同一普通.omnb并全部运行，显示真实西瓜、2/3；设置显示应用与Rust均.5及完整SHA。全过程无新增模型请求，调用账本仍69/80。

实际AX、DOM、截图和包级核验记录位于同SHA目录的`local-acceptance/`。构建日志为`target/plan-5/g4-modern-file-events-{native-package,web-build}.log`。新增文件事件逻辑的现有WorkbenchFixtures入口`--file-events`已通过真实冷/热源码打开、非file与多URL拒绝、关闭存储；完整应用Finder验收亦通过。旧候选和失败记录保留。系统OpenPanel间歇出现有效.omnb的Open禁用仍是已知限制；本次Finder路径已验证可用，未改系统安全或权限。

发布 tag 为 `v0.1.0-pre-alpha.5`，target 为上述完整源码 SHA；后续仅文档提交不改变包内 revision。2026-10-21 用户随后明确要求“不用验证了，同步更新readme等文档”，取消剩余公开资产匿名下载、哈希复核和重复安装回读。本记录中的哈希来自发布前本地资产核验，安装结果来自上述本地发行包；不将公开下载回读记为通过。G4 按用户本次调整后的范围完成，原第 1.2 节第 5 项公开回读部分记为用户取消。

用户随后另行追加 macOS ARM64 / Windows x64 的现有 CLI ZIP，产品源码仍固定为上述完整 SHA；CLI 使用独立 `cli-release-manifest.json`，不改原三包和原清单，结果单列在 [CLI 补包记录](cli-release.md)。原三包的公开回读取消记录不变。
