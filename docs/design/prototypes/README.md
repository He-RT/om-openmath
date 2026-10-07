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
