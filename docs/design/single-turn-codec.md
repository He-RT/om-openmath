# 独立单轮模型 codec（R4.3.03）

`om-llm::single_turn` 只编译一次不可变请求、解码一次实际响应；它没有 HTTP、凭据查找、工具执行、重试或下一轮状态。Pi 仍是唯一循环。旧 Job/LlmChat 不参与本接口，旧调用保持独立回归。

`om-agent-contract` 从原 `native-agent.schema.json` 生成 canonical DTO 和依赖类型。宿主 re-export 同一 Rust 类型，Swift/TypeScript 来源及 357 DTO 数量不变；required-null、严格 UTF8/重复成员、深度/节点与闭合结构验证共用原实现。生成器以完整相对路径暂存，避免两个 crate 的同名文件互相覆盖。

`prepare(Request, message_id, task_id, turn_id)` 从同一可信快照产生 body 和 Decoder；工具声明、stream、provider/route/model/adapter binding 同源，输出身份由宿主指定。API 模型别名与不可变 model_ref 分开。请求只含 body，无 URL/headers/key；最终认证及传输生命周期由 R4.3.04 实现。

当前支持 Chat Completions 和 Anthropic Messages 的文本/工具 SSE 与 JSON 响应。系统消息必须位于会话开头，原 call ID 与参数字节保留，每个工具结果须匹配先前尚未结算的调用；缺失、重复、孤立或 outcome 不一致拒绝。媒体、step links、未知 content blocks 和旧 function_call/audio 明确不支持，后续协议/模态由 R4.4.11 补齐。

Decoder 接收实际状态码、原始 bytes 和真实 EOF，取消/毒化/结算后不接受迟到数据。每值 1 MiB、总响应 8 MiB、32 calls、128 blocks、参数 64 KiB、单文本 256 KiB。Chat 需要 finish_reason 与 `[DONE]`；Anthropic 需要 message_start/block 关闭/stop_reason/message_stop。未知增量 metadata 可忽略，未结束 SSE 不默默截断。

结果分开保存 canonical、实际 usage、原 finish reason、原 provider ID/model 与 replay-only 内容。缺失 usage 保持 null。`length`、`max_tokens` 或未知结束理由为 incomplete，未完成 calls 只进诊断，没有可执行 tool proposals，也没有完整 replay。Pi 数值 carrier 的占位 0 不充当计费事实。

兼容 replay 必须匹配全部 binding；Chat refusal/reasoning 内容、Anthropic thinking/signature/redacted thinking 按原协议回放，不投影成用户正文。换 route/model/provider/adapter 后拒绝复用 opaque replay。模型响应不授予权限；实际工具仍经过原宿主边界。

验证包含所有 UTF8 切分点、SSE 多 data/CRLF、多工具参数、HTTP/取消/缺终止/未知字段，以及 Rust→真实固定 Pi 的两轮 result 配对和截断停止。原生随包 helper fixture 使用实际 Rust codec 产物，经 Source owner→私有帧→SQLite/readback/ACK 驱动 Pi；工具仍返回真实 unavailable，未执行数学工具。两个用户授权 Chat 模型仅做实际短流开发 smoke；未声称 Anthropic 付费 API、生产 URLSession/SecretStore/Registry 或最终 gate 已验收。

协议参考：[Chat chunk schema](https://developers.openai.com/api/reference/resources/chat/subresources/completions/streaming-events)、[Anthropic streaming](https://platform.claude.com/docs/en/build-with-claude/streaming)。完整输入/源码 hash/原失败/实际输出见 [单轮证据](../acceptance/pre-alpha.4/development/r43/single-turn-codec.json)。
