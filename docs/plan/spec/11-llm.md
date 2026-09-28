<!-- Extracted from docs/plan/PLAN.md sections [11]. PLAN.md is authoritative; keep in sync. -->

## 11. LLM 层规格（om-llm）

### 11.1 设计：sans-IO
`om-llm` 核心不做任何 IO：它负责 **构造 HTTP 请求**（`HttpRequest { method, url, headers, body }`）、**增量解码响应字节流**（SSE / JSON）、**管理多轮对话与工具调用状态机**。传输由调用方完成：
- native（CLI、Tauri）：`om-llm` 的 `http` feature 提供 `drive_native(job, &reqwest::Client, on_event)` 异步驱动器。
- 浏览器（WASM）：TypeScript 用 `fetch` 获取流（`app/src/kernel/llmDriver.ts`），把文本块通过 `LlmHttpChunk` 回灌给 kernel。
这样同一套逻辑在所有平台上都一样，并且可以用录制的 fixture 完整测试。

```rust
// crates/om-llm/src/lib.rs
pub struct Profile { pub name: String, pub kind: ProviderKind, pub base_url: String, pub model: String,
    pub api_key: Option<String> /*运行时解析后的真实 key*/, pub temperature: f32, pub max_tokens: u32,
    pub supports_tools: bool, pub supports_json_mode: bool, pub timeout_ms: u64, pub extra_headers: BTreeMap<String,String> }
#[serde(rename_all = "snake_case")] pub enum ProviderKind { OpenaiChat, Anthropic, OpenaiFim, OllamaFim, MistralFim }
pub struct HttpRequest { pub method: String, pub url: String, pub headers: Vec<(String, String)>, pub body: String, pub stream: bool }
pub struct ChatMessage { pub role: Role /*system|user|assistant|tool*/, pub content: String,
    pub tool_calls: Vec<ToolCall>, pub tool_call_id: Option<String> }
pub struct ToolCall { pub id: String, pub name: String, pub arguments: String /*JSON 文本*/ }
pub struct ToolSpec { pub name: &'static str, pub description: &'static str, pub parameters: serde_json::Value /*JSON Schema*/ }

pub fn build_chat_request(p: &Profile, msgs: &[ChatMessage], tools: &[ToolSpec], json_mode: bool, target: Target) -> HttpRequest;
pub fn build_fim_request(p: &Profile, prefix: &str, suffix: &str) -> HttpRequest;
pub enum Target { Native, Browser }   // Browser 时 Anthropic 需加 anthropic-dangerous-direct-browser-access: true
pub enum StreamEvent { Text(String), ToolCallDelta { index: u32, id: Option<String>, name: Option<String>, args_fragment: String },
    Finish { reason: String }, Error(String) }
pub struct SseDecoder { buf: String }         // feed(&str) -> Vec<SseMessage{event: Option<String>, data: String}>；处理 \r\n、多行 data、注释行 ':'
pub fn decode_openai_chunk(data: &str) -> Vec<StreamEvent>;      // "[DONE]" -> Finish{reason:"done"}
pub fn decode_anthropic_event(event: &str, data: &str) -> Vec<StreamEvent>;
pub fn parse_fim_response(kind: ProviderKind, body: &str) -> Result<String, LlmError>;
```

### 11.2 各提供商的 HTTP 形状（照此实现，并用 fixture 测试）
- **OpenAI 兼容 Chat**（OpenAI、DeepSeek、通义千问 DashScope 兼容模式、小米 MiMo 兼容接口、Ollama `/v1`、LM Studio、vLLM、OpenRouter）：
  `POST {base_url}/chat/completions`；头 `Authorization: Bearer {key}`（key 为空时不加）、`Content-Type: application/json`；
  body `{"model","messages":[{"role","content"}…],"stream":true,"temperature","max_tokens","tools":[{"type":"function","function":{"name","description","parameters"}}],"response_format":{"type":"json_object"}}`（后两者按需）。
  流：SSE，每条 `data: {"choices":[{"index":0,"delta":{"content":"…","tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"solve","arguments":"{\"eq"}}]},"finish_reason":null}]}`；结束 `data: [DONE]`。`delta.reasoning_content`（DeepSeek 推理模型）忽略。工具参数按 `index` 累加拼接。助手带工具调用的消息回填为 `{"role":"assistant","content":null,"tool_calls":[…]}`，工具结果为 `{"role":"tool","tool_call_id":"call_1","content":"…"}`。
- **Anthropic Messages**：`POST {base_url}/v1/messages`（base 默认 `https://api.anthropic.com`）；头 `x-api-key: {key}`、`anthropic-version: 2023-06-01`、`content-type: application/json`；body `{"model","max_tokens","system":"…","messages":[…],"stream":true,"tools":[{"name","description","input_schema"}]}`（system 从消息列表中抽出）。SSE 事件：`message_start`、`content_block_start`（`content_block.type` 为 `text` 或 `tool_use{id,name}`）、`content_block_delta`（`delta.type` 为 `text_delta.text` 或 `input_json_delta.partial_json`）、`content_block_stop`、`message_delta`（`delta.stop_reason`：`end_turn`/`tool_use`/`max_tokens`）、`message_stop`、`ping`、`error`。工具结果回填为 user 消息 `{"role":"user","content":[{"type":"tool_result","tool_use_id","content"}]}`；助手工具调用消息为 `{"role":"assistant","content":[{"type":"tool_use","id","name","input":{…}}]}`。
- **FIM（幽灵补全）：**
  - `openai_fim`（DeepSeek beta 等）：`POST {base_url}/completions`，body `{"model","prompt":prefix,"suffix":suffix,"max_tokens","temperature":0,"stop":["\n"],"stream":false}` → `choices[0].text`。
  - `ollama_fim`：`POST {base_url}/api/generate`，body `{"model","prompt":prefix,"suffix":suffix,"stream":false,"options":{"temperature":0,"num_predict":max_tokens,"stop":["\n"]}}` → `response`。
  - `mistral_fim`：`POST {base_url}/v1/fim/completions`，body `{"model","prompt","suffix","max_tokens","temperature":0,"stop":["\n"]}` → `choices[0].message.content`。
  - 如果 `complete` 功能配置的是 chat 类 profile：用非流式 chat 请求加专用提示词（“只输出应插入光标处的文本，不要解释”），并把 `prefix⟨CURSOR⟩suffix` 放进用户消息。
- **预设（设置界面的一键填充）：** OpenAI `https://api.openai.com/v1`；DeepSeek `https://api.deepseek.com/v1`（FIM: `https://api.deepseek.com/beta`）；通义千问 `https://dashscope.aliyuncs.com/compatible-mode/v1`；OpenRouter `https://openrouter.ai/api/v1`；Ollama `http://localhost:11434/v1`（FIM: `http://localhost:11434`）；LM Studio `http://localhost:1234/v1`；Anthropic `https://api.anthropic.com`；小米 MiMo 及其他：“OpenAI 兼容（自定义）”，由用户填 base_url 和 model。**模型名只作为可编辑的默认值**，不要硬编码进逻辑。

### 11.3 功能与提示词（提示词文件：`crates/om-llm/prompts/*.md`，用 `include_str!` 编译进二进制，`{{name}}` 占位符替换）

1. **translate（自然语言 → 表达式）** `prompts/translate.md`：
   ```
   You translate math requests (Chinese or English) into ONE Wolfram Language expression for the OpenMath CAS.
   Rules:
   - Output ONLY a JSON object: {"wolfram": "<expression>", "explanation": "<one short sentence in {{lang}}>"}.
   - Use only these functions: {{function_list}}.
   - Equations use ==. Multiplication may be written with * or a space. Use Sqrt[], Pi, E, I, Log[] (natural log).
   - For "solve"/"求解"/"解方程" use Solve[eqs, vars] (or Solve[eqs, vars, Reals] when the user asks for real solutions / 实数解).
   - Systems use a list: Solve[{eq1, eq2}, {x, y}]. Inequalities use Reduce[ineq, x, Reals] unless the user says solve.
   - Numeric requests ("approximately", "数值解", "近似") use NSolve or N[...].
   - Symbols already defined in the notebook: {{defined_symbols}}. Reuse their names.
   - Never output anything except the JSON object.
   Examples:
   User: solve x squared plus 2x equals 3 → {"wolfram":"Solve[x^2 + 2*x == 3, x]","explanation":"..."}
   User: 求方程 x^3 - 2x + 1 = 0 的实数解 → {"wolfram":"Solve[x^3 - 2*x + 1 == 0, x, Reals]", ...}
   User: 解方程组 x+y=10, x-y=2 → {"wolfram":"Solve[{x + y == 10, x - y == 2}, {x, y}]", ...}
   User: sin x = 1/2 在 0 到 2π 之间的解 → {"wolfram":"Solve[Sin[x] == 1/2 && 0 <= x <= 2*Pi, x]", ...}
   User: 分解因式 x^4-1 → {"wolfram":"Factor[x^4 - 1]", ...}
   User: x^2 < 4 的解集 → {"wolfram":"Reduce[x^2 < 4, x, Reals]", ...}
   User: find numeric roots of x^5 - x + 1 → {"wolfram":"NSolve[x^5 - x + 1 == 0, x]", ...}
   User: 圆 x²+y²=25 和直线 y=x+1 的交点 → {"wolfram":"Solve[{x^2 + y^2 == 25, y == x + 1}, {x, y}]", ...}
   ```
   流程：`supports_json_mode` 时开启 JSON 模式；取回复中第一个 `{` 到最后一个 `}` 解析；`wolfram` 字段用 `om_parse::parse_expr(…, Wolfram)` 解析；失败时把诊断发回模型重试（“Your expression failed to parse: {{diagnostics}}. Return corrected JSON.”），**最多 2 次**；成功后生成 `Suggestion { wolfram: input_form(expr), modern: modern_form(expr), latex: latex(expr), explanation }`（modern/latex **由我们自己的格式化器从解析结果生成**，不信任模型给的文本）。前端展示建议卡片，**需要用户点击才插入或运行**。
2. **explain（步骤讲解）** `prompts/explain.md`：系统提示要求“只能使用提供的步骤与结果，不得引入新的数学结论；用 {{lang}}；引用步骤时写 [S{n}]；公式用 $…$”；用户消息是 `{"input": InputForm, "result": InputForm, "steps": StepsView JSON}`；流式输出，前端用 Markdown + KaTeX 渲染。`step_id` 非空时只讲解该步骤。
3. **complete（幽灵补全）**：见 11.2 FIM；前缀 = 当前 cell 光标前文本（send_context 时前面加上前 3 个 cell 的源码，用方言对应的注释包裹）；后处理：去掉首尾空白中的换行、截断到第一个换行；如果 `prefix + suggestion` 的解析诊断里含 `E0xx` 词法错误（非法字符）就丢弃；与本地补全第一项相同就丢弃。
4. **chat（助手面板，带工具）**：工具：
   - `evaluate {"code": string}`：用 Wolfram 方言解析 → 在 `fork_readonly()` 的求值器里执行（禁止 Set/SetDelayed，否则返回错误文本）→ 返回 `{"input_form","latex","messages"}`；每次调用超时 5 s。
   - `solve {"equations": [string], "variables": [string], "domain": "Complexes|Reals|Integers"}`：同上，但组装成 `Solve`，并返回 `{"solutions": InputForm, "steps_summary": [step titles]}`。
   - `propose_cell {"code": string, "dialect": "modern|wolfram"}`：不执行，只作为建议卡片发送给前端（`LlmSuggestion`），由用户确认插入。
   最多 6 轮工具调用；系统提示 `prompts/chat.md` 要求：“凡是数学计算必须调用工具，不得心算；最终答案必须引用工具结果”。
5. **fix（修复错误）** `prompts/fix.md`：输入源码、方言、诊断与消息，输出 JSON `{"wolfram": "...", "explanation": "..."}`，校验流程与 translate 相同。
6. **test profile**：发送 “Reply with the single word: pong”，max_tokens 8，显示延迟与首字节时间。

### 11.4 Job 状态机（kernel 用它把 LLM 功能接进协议）
```rust
pub struct Job { /* feature, profile, messages, pending tool calls, accumulated text, retries … */ }
pub enum JobStep {
    Http(HttpRequest),                      // 需要发起（下一轮）HTTP 请求
    RunTools(Vec<ToolCall>),                // 需要 kernel 执行工具，然后调用 job.tool_results()
    Done(JobResult),                        // JobResult::Text(String) | Suggestion(Suggestion) | Completion(String)
    Failed(LlmError),
}
impl Job {
    pub fn new(feature: Feature, profile: Profile, input: JobInput, target: Target) -> (Job, JobStep);
    pub fn on_bytes(&mut self, chunk: &str) -> Vec<StreamEvent>;       // 流式事件（文本增量转发为 Event::LlmDelta）
    pub fn on_http_end(&mut self, status: u16, error: Option<String>) -> JobStep;
    pub fn tool_results(&mut self, results: Vec<(String /*call id*/, String /*content*/)>) -> JobStep;
}
```
- HTTP 状态码非 2xx：读取 body 中的 `error.message`（OpenAI/Anthropic 格式）→ `Failed`，前端显示 “{profile}：{status} {message}”。401/403 时提示检查 API Key。
- 取消：`LlmCancel` → kernel 删除 job；native 驱动通过 `tokio_util::sync::CancellationToken` 中止请求；浏览器端通过 `AbortController`。

### 11.5 测试
- `crates/om-llm/tests/fixtures/`：`openai_text.sse`、`openai_tools.sse`（两个工具调用，参数分多块到达）、`anthropic_text.sse`、`anthropic_tools.sse`、`deepseek_fim.json`、`ollama_fim.json`、`mistral_fim.json`、`openai_error_401.json`。
- 解码测试：fixture 按**随机切块**（1–7 字节，用 SplitMix64 固定种子）喂给 SseDecoder，结果必须与整块喂入一致。
- 请求构造：insta JSON 快照（Authorization 头里的 key 用 `sk-test`）。
- Job 状态机：用 fixture 驱动完整的“两轮工具调用 → 最终文本”流程。
- native 驱动：`wiremock` 起本地服务器返回 fixture，验证端到端（仅 `--features http`）。
- 可选真实测试：`OM_LIVE_LLM=1 DEEPSEEK_API_KEY=… cargo test -p om-llm --features http live_ -- --ignored`，默认不跑。
