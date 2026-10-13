# 原生 Agent 规范协议与会话入口

R4.3.01 的业务记录由 OpenMath 定义，与 Pi 内部 state 无关。机器契约为 `native-agent.schema.json`；Rust、Swift、TypeScript 生成闭合 DTO。实现和实际输入/输出证据见 [开发验收](../acceptance/pre-alpha.4/development/r43/agent-protocol-native.json)。

`om_host_agent_command` 是可信本机控制接口。`SourceEndpoint` 派生实际 runtime/document/generation，持有唯一 AgentEndpoint；helper 不能选择另一个文档。任务、连接均须新激活，关闭后的替代代次严格增加。日记恢复只恢复事实，不恢复 grants、提案授权或映射表。应用宿主关闭与文档批准关闭先撤销接纳，再等待物理 IO/进程退出。

| 边界 | 行为 |
|---|---|
| 私有 stdio | UTF8 NDJSON；每帧 1MiB，严格重复成员/深度64/节点32768检查，批次32；原始完整字节进入 Rust，失败或未完成 EOF 关闭连接 |
| 可信控制封套 | 2MiB；不能用封套转义绕过内部帧限制 |
| 账本 | 2MiB 完整载荷；原有 library4/session event1 格式，兼容 assistant_end 投影与完整 canonicalJSON、peerMessageID/connectionID/sourceSequence 一并保存 |
| ACK | SQLite 提交、原事件及回执读回、完整 canonical 字节 hash 一致后才由 Rust 产生；原帧重放返回相同 ACK |
| 模型/工具身份 | 宿主熵生成 pending intent ID，绑定当前 turn/request/call；ID 不表示操作已接纳、已执行或模型已发送 |
| 请求关联 | prepare_request 建立原请求映射；model_request 必须同 turn，引用不能以相同请求身份替换；引用尚须后续可信 ContextSnapshot/model adapter 解析 |
| 方向 | helper 可提交 ready/规范 assistant 事件/工具提案引用/prepare/model 请求/error/settled；helper model_event、宿主 ACK/输入/工具结果方向拒绝 |
| settled | 只进入 awaiting_settlement；无数学或任务成功推断 |

接纳过的原始消息限制为4096条/8MiB，工具提案32、模型请求64。写端按宿主序号缓冲，防止并发恢复的 ACK/取消/输入乱序或字节交错；等待32项/2MiB，原发送保留4096条/8MiB。旧 ACK 只允许相同字节重放。stderr 有界读取并丢弃；子进程显式环境只有 PATH，macOS 可以加入 `__CF_USER_TEXT_ENCODING`，不继承 provider 环境或 shell/profile。

Swift AgentSessionService 归 NativeDocumentSession 持有。构造与恢复不会启动模型、求值或进程。输入、steer、follow_up 的规范消息先持久化；重试保持原输入帧。消息处理串行等待，取消绕过账本等待并立即撤销 Rust 接纳。完整事件尚未确认时不会映射工具调用。Foundation 的 `read(upToCount:)` 在实际小帧测试中等待填满长度，已换为单次 POSIX read；读者独立 fd 和有限终止/强制退出路径避免旧描述符复用。

传输帧大小与展开后的日记载荷分别受限。实际 1,000,914 字节合法帧展开为超过2MiB的日记载荷，保存被拒绝，没有事件或 ACK；不会裁切为摘要后宣称完整消息。未来更大正文可以经另一个明确的持久 Blob 契约接入。

实际测试使用合成 Node 私有进程，验证完整握手、原 ACK 重放、宿主意图映射、八类恶意/截断进程输出、写入顺序与实际 SQLite 恢复。Pi 1.0.4 和生产随包 runtime/helper 属于 R4.3.02；模型编码、真实网络/SecretStore、12工具 handler 与 Agent loop 属于后续任务。这里没有这些效果，32最终门禁仍为 not_run。
