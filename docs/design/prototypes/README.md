# Mac 助手输入框交互草案

[规格](../agent-composer.md) · [HTML](mac-agent-composer.html) · [截图](mac-agent-composer.png)

这是一份独立 HTML/CSS/JavaScript 界面草案，没有接入 OpenMath 应用、Pi 或模型服务。示例对话、供应商与模型在界面中明确标记；左侧是此前真实内核产生的草方块几何预览，源码只展示摘要，不是新的 Agent 执行证据。

可交互内容：深浅主题、窄栏、模型分组/搜索/选择、执行/讨论、菜单键盘与焦点恢复、文件选择、浏览器图片粘贴/文件拖入、附件预览与移除、单元格引用、草稿保护、待发送消息预览。点击发送只展示草稿快照，不调用模型、不上传文件、不修改笔记本。刷新页面不持久化附件。

通过 HTTP 本地查看：

```sh
python3 -m http.server 8766 --bind 127.0.0.1 --directory docs/design/prototypes
```

打开 `http://127.0.0.1:8766/mac-agent-composer.html`。也可直接用浏览器打开 HTML 文件；实际剪贴板事件可用性取决于浏览器。Finder 文件 URL/文件承诺、原生媒体处理和真实供应商载荷仍需 Mac 宿主实现。

2026-10-08，使用现有 Playwright/Chromium 验证了 10 项浏览器交互，页面无 JavaScript 异常，浅色、335px 窄栏及 390px 窗口无横向溢出。真实文件选择器附加图片；混合图片/文字粘贴与音视频/PDF/未知文件拖入使用合成浏览器事件。后者仅验证分类与接收，没有验证音视频解码、原生 Mac 粘贴或模型理解。结果与补充截图保存在本仓库 `target/agent-composer-evidence/`，不会当作实际 Agent 验收。

截图等待菜单动画结束后获取。未连接请求时上下文显示未知；不会显示虚构 token 数量或百分比。

`assets/grass-block.png` 是 OpenMath 草方块示例的自有几何预览；界面没有复制用户截图文件或第三方应用资产，没有增加前端依赖。

## 原生 Mac 工作台 UX 草案

[完整 UX 规格](../macos-ux.md) · [交互草案](mac-workspace-ux.html) · [初始工作台截图](mac-workspace-ux.png) · [组件清单](../macos-ui-inventory.json)

同一服务器打开 `mac-workspace-ux.html`，查看可收起大纲、笔记本编辑和步骤/变量/助手目的地、命令搜索、设置、上下文管理样式、单元格插入/移动/删除/撤销与消息过渡。运行按钮只演示运行/停止状态，不计算、不调用模型；源码变化将静态示例输出标为过期。

设置可切换浅深主题和减少动态效果。系统 Reduce Motion 开启时不能由评审页关闭，系统与控件显示同步；动画属于装饰，不改变消息接受或运行/停止状态。新格插入后又被用户编辑时，演示撤销拒绝丢弃该草稿。实际原生 UndoManager/事务仍须实施。

2026-10-08，Playwright/Chromium 的 13 项浏览器交互验证通过，没有页面脚本异常，720px 窄窗口无页面横向溢出。检查包含草稿、中文合成、停止迟到回调、结构撤销、系统/应用动效偏好和上下文编辑。中途隐藏控件查询与系统媒体事件等待问题已修正，过程记录保留在 `target/native-ux-evidence/checks.json`；补充窄窗口截图和 HTML 动效录屏在同目录。后者不是原生帧率、系统控件或 Agent 运行验收。

这里所有控件仍是 HTML 评审表示，**不计入 75% Apple 标准组件的设计比例**。组件清单标明每项为 planned，原生 Mac 实现后逐项复核，不能用本页截图证明原生代码已完成。

## 供应商、模型与媒体设置草案

[服务契约](../model-media.md) · [原生 UI/UX](../macos-model-media-ux.md) · [交互草案](mac-model-settings.html) · [供应商截图](mac-model-settings-providers.png) · [模型截图](mac-model-settings-models.png) · [媒体截图](mac-model-settings-media.png) · [实际评审记录](model-media-review.json)

沿用前述 HTTP 服务打开 `mac-model-settings.html`。本轮也可在已启动的 `http://127.0.0.1:8768/mac-model-settings.html` 查看。供应商、模型、功能映射和媒体处理四页的可操作数据均为内存示例；刷新恢复初始状态，没有连接 Pi、Rust、Keychain、模型服务或真实上传，密码不写浏览器持久存储。

重点评审供应商的默认上下文/多模态/思考映射，以及模型逐项“继承供应商/单独设置”。默认变化只传播继承项，Pro 示例单独关闭图片；“全部恢复供应商默认”先形成草稿，保存后生效。映射编辑只演示配置关系，真实协议的字段/枚举/预算/伴随参数验证仍需 adapter。高级协议/完整多预设 CRUD、目录真实分页/手工 API 导入和真实媒体处理由规格定义，不因本页能点击而称实现。

顶部“评审场景”可模拟正常、保存失败、保存 unknown 和探测失败。Test 不保存，未知提交先核对原回执，探测完成但草稿已经变化时不更新当前能力。模型菜单提供讨论/执行、搜索/键盘/恢复焦点与“下一轮生效”；附件示例显示所选页与未覆盖页，切到不兼容模型须明确选择提取文本。

2026-10-08，**27 项 Playwright/Chromium 浏览器交互通过**，页面无脚本异常且无外部网络请求；检查包含 1000/760/640px 宽度无页面横向溢出、深浅主题和系统减少动态效果。新增 Schema 的 15 正例/32 反例及原组件族计数核验通过。一次早期草案检查发现保存等待时保留了旧成功文字，已改为 pending/新回执确认；原记录保留在本仓库 `target/model-media-evidence/`。

三张截图来自最终 HTML 草案；另外的暗色/窄窗截图和测试脚本/日志位于 `target/model-media-evidence/`。浏览器测的是演示状态/导航/继承与布局，不证明原生控件、秘密存储、真实供应商能力、Office/PDF/OCR/音视频或模型理解已验收。原生 MM01–MM15 仍全部 planned。
