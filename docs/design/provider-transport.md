# 原生单轮传输（R4.3.04 进行中）

`ProviderTransport` 每次只发送一个冻结请求。`ProviderModelSession` 通过新增的 `om_host_model_codec_command` 调用真实 Rust codec；没有测试可执行文件、旧 Job/LlmChat、Node HTTP 或第二模型循环。独立 model mutex/Swift queue 与 CAS/source/event owners 分开。

`native-model-codec.schema.json` 生成 prepare/headers/feed/finish/cancel 与回复 DTO，生成总数为371。可信 native caller 提供 request/context/task/turn/message 身份，Rust 编译 body 后分配随机 codec_ref；请求和 context 不可重复使用。每 host 最多8在途、4096历史 nonce，feed 每次最多8192原字节，继承单轮8MiB总响应边界。host关闭释放全部解码状态；纯 codec 不授予文档、工具、网络权限。

Swift 的 `ProviderRoute` 仅由 native 预备器创建，不从 helper/model frame 解码。binding 与编译请求必须一致；拒绝用户信息、query、fragment、非HTTPS。显式本地策略只允许 HTTP literal 127.0.0.1/::1。Cookie/cache/shared credential storage 禁用，重定向一律拒绝；TLS继续系统验证，不回退 Basic/其它持久凭据。认证只在最后的 URLRequest 头中加入。

SecretStore 只查新服务的精确引用，不枚举或回退旧 profile/key/env。Mac `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` 必须配合 Data Protection Keychain；SDK SecItem.h 明确指出默认 Mac keychain 不支持该属性。已加入 `kSecUseDataProtectionKeychain=true`，并用 fresh LAContext 禁止交互。无权限、锁定或认证失败时拒绝，无替代凭据。此前 D049 只验证了查询形状，不能据此认定默认 Mac keychain 实际执行了解锁访问策略；本批修正并保留旧证据。

实际可执行超时为连接10秒、首响应60秒、流空闲30秒、总180秒。连接预算以首个实际请求体发送进度或响应头为完成证据；URLSession公开didSendBodyData回调确认发往服务器的body字节。本接口始终发非空POST，不将等待模型响应继续算作连接阶段。首响应预算同时覆盖至响应头的总体等待。真实回环HTTP测试在1秒连接预算下收完请求，再等1.3秒才回响应，仍成功；无连接进度的独立超时亦已验证。超时后不重发计费请求。

发送前必须 await journal callback，再 resume task。之后未观察到响应的中断保留 dispatch outcome unknown；HTTP状态与本机取消不证明服务端已停止。`NativeModelLifetime` 让 host关闭在actor/账本暂停时也能直接取消HTTP；回调、EOF和迟到字节不会重建已关闭通道。当前 URLProtocol 证据中的 journal callback 是合成回执边界，真实请求账本/ContextSnapshot producer接线仍待完成，不能称已完成耐久请求集成。

`ProviderAgentModelAdapter` 将同一冻结上下文接到 AgentSessionService 原生端口。Unicode文本按UTF8标量拆为最多32768字节，结束时保留原 canonical；incomplete 发failed且没有可执行调用。usage/replay/原结束理由通过强制 complete callback 返回 native context owner，不由Pi私有carrier代替。完整使用方仍需提供实际上下文及持久化实现。

已验证实际URLSession→Rust C ABI（Chat/Anthropic）载荷/认证、401/429/error echo、redirect、三阶段timeout、取消、host关闭、nonce、并行真实CAS=4与原生端口32KiB/截断。两授权模型实际Swift流调用均返回4及usage，仅属开发smoke；使用明确测试凭据，不冒称Keychain/Registry/耐久请求已验收。真实锁屏、实际请求账本/上下文接线仍为R4.3.04未完成项。完整计划仍32/98；最终gates保持not_run。
