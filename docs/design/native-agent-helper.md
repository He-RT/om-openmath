# 固定 Pi 与随包原生 helper

R4.3.02 以 Pi Core/AI 1.0.4、Node 26.11.1 darwin-arm64 组成真正可执行的生产闭包。[实际验收](../acceptance/pre-alpha.4/development/r43/pi-production-runtime.json)保留完整输入、13条真实双向帧、3条SQLite事件、构建与原失败日志。

`build-helper.py` 校验固定归档SHA256、官方发布者 OpenPGP 签名与实际Node字节/arm64头；使用固定TypeScript开发编译器输出 ESM。用户包包含独立Node、86个锁定的生产依赖、所有实际动态资源与许可；没有npm、转译器、TS源码、source map、测试fixture或CodingAgent/shell插件。当前完整闭包4169文件，约172MB。每个文件列入 HelperManifest；manifest 本身由App资源签名封印。公开同Team重签、公证和发行manifest仍由R4.5执行。

入口为包内绝对 `Agent/main.mjs`。AgentProcess 使用Security API校验Node/App实际签名，验证完整文件清单、每个字节hash、架构与所需资源；拒绝额外文件/符号链接/缺资源。它创建专用空0700 cwd，显式空PATH环境，去掉NODE_OPTIONS/NODE_PATH/提供方key继承，没有系统node/npm发现路径。Node公共permission模式只允许读取包内文件，不授予写入、addons或子进程；这是Node自身的机制，不代表OS全盘隔离。Hardened Runtime仍按实际组件身份验证。

规范消息与Pi state隔离。用户输入先经原生账本持久化，再在 user_input/steer/follow_up 帧同时传递完整 canonical message 和事件引用。Pi 的 custom streamFn 请求先经过 native prepare 绑定不可替换的context/model refs，再发送model_request。宿主 model_event 关联turn/request/UTF8 byte_offset；终结帧携带完整 canonical message。helper保留原始 arguments_json，与Pi参数转换对象分别存在，不以重新stringify后的参数代替原字节。

三个只读工具由当前SourceEndpoint执行：agent_read_tool只接已映射operation_id，参数从已耐久ACK的原提案取回，调用方不能替换。宿主冻结完整闭合工具结果并返回result_ref/result_hash；Swift将原字节存为Library引用Blob，记录带规范tool消息的native_readonly_result恢复事实，再独立回读事件与Blob后agent_confirm_tool。tool_result帧的result_json只承载这份已确认的有界数据，规范消息只保存result_ref。原生侧核对原call/turn/task、结果哈希及当前scope；取消、过期或文档/编辑/定义绑定变化拒绝新投递和旧帧重放。helper校验工具结果契约，把数据交给Pi现有tool结果流程继续下一轮，不执行源码、不把读取冒充DocCommit/CAS效果。未实现工具仍以unavailable/null结果终止该批工具，恢复记录不会重新授予执行权限。

beforeToolCall 等待真实 canonical ACK，工具顺序执行；后续模型请求等待完成的规范事件。十二工具声明来自固定机器契约，没有动态插件。尚未实现的宿主工具只返回 unavailable，日记中记录“未执行”事实，不捏造admission/operation receipt。没有配置模型adapter时发出明确错误并settled failed，不添加虚构assistant输出。模型codec、URLSession/SecretStore、真实工具handler和Agent UI分别属于后续任务。内部Pi模型carrier和零usage只是私有桥接数据，不是实际模型或计费指标。

服务监督私有pipe握手、帧顺序、重放、取消、退出；握手超时5秒。取消不等待模型流结束，迟到callback由Rust当前scope/phase拒绝。实际SIGKILL测试表明helper关闭后同一文档/主内核仍输出4；独立Source/Kernel owner不因helper崩溃停用。

验收使用随包真实Node/Pi及实际C ABI/source/SQLite，由合成native model port提供模型数据，未调用外部模型或执行工具效果。真实App签名/资源路径、空PATH/污染旧环境、不存在的资源、错的真实Mach-O字节（即使伪造匹配manifest hash）、取消后的迟到delta/旧stdin均验收。所有最终门禁仍not_run，公开/运行版本仍.3。
