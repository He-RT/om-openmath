# 原生 Mac 开发

原生客户端正在按 [`.4`完整计划](../docs/plan/PRE_ALPHA_4.md)实施，当前运行源码版本仍`.3`。本目录不是已发行Mac包；公开安装和旧数据保持独立。

## 当前可用

- 可复现的Xcode27/macOS27 ARM64工程和SwiftUI/AppKit开发窗口。
- 独立`om-apple-ffi`静态库，实际C ABI/内核元数据/版本读回；完整会话、文档事务和事件泵尚待后续任务。
- 固定Pi/Node/Swift依赖及原许可审计；此阶段未将Pi helper接入或嵌入应用。

## 构建

在仓库根目录执行：

```bash
bash macos/Scripts/verify-env.sh
python3 macos/Scripts/generate-project.py
python3 macos/Scripts/generate-project.py --check
bash macos/Scripts/build-native.sh
```

产物为`target/macos-native/Build/Products/Release/OpenMath Preview.app`。Preview独立Bundle ID为`org.openmath.OpenMath.NativeMacPreview`，只有本地ad-hoc签名；当前不注册`.omnb`关联、不初始化旧Tauri数据或主动连接模型。Xcode工程可直接打开，不需要第三方工程生成器。

工程将库定位到仓库`target/release/libom_apple_ffi.a`，脚本先构建真实Rust库再构建Swift；没有装系统Node或依赖本机Homebrew dylib。最终用户包的随包Node、签名公证、文档/模型/媒体功能以实施任务和真实包验收为准。

关于原生框架、主Actor、文档/计算所有权和后续存储边界，见[宿主契约](../docs/design/macos-host-state.md)。窗口采用实际可用宽高/系统滚动、缺业务能力明确显示，不用动画或mock文字制造成功；参考telegram-ui-reference的屏幕适配和进度/回执基础路径。

原生宿主调度和 C ABI 已接通真实隔离计算、Editor Preview 与 Desktop 能力查询；文档持久化/写入/checkpoint 与完整工作台仍按 `.4` 计划接入。底层 `get_function_catalog` 支持 `offset`/`limit` 分页（默认 0/32），回复携带 `total`/`next_offset`，全部条目来自实际内核回调。`native_renderers_ready=false` 表示当前链接预览还没有完整原生渲染器。

运行跨语言验收：

```sh
bash macos/Scripts/test-native-contracts.sh
```

该脚本直接由 Swift 调用新的 C ABI，验证精确 `2+2`、真实数学错误、Desktop 能力、直接取消和解码后释放。`create` 失败必须读取并释放错误 buffer；`submit` 回复只是入队回执。`close_begin` 只撤销入口和发出取消，`close_finish` 必须后台执行并等待活动调用/owner 停止。调用者不读/释放 opaque handle，不再使用已消费的旧 handle。为避免旧地址重新指向新会话，小 token tombstone 保留到进程结束，上限 4096 次创建/128 同时宿主；完整服务和结果内存均在 finish 释放。返回 buffer 恰好释放一次，地址可由后续 buffer 重用。

新增 `macos-native.yml` 在固定 Xcode 27/SDK27 镜像构建和执行这些验收。当前产物仍是独立 ad-hoc 开发预览，不能据此宣称发行签名、公证或完整 `.4` 已通过。
