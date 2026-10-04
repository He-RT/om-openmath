# 原生移动端验收记录

此文件区分已运行证据与待验收项，不将设计规范或编译成功当作完整功能验收。

## 环境

2026-10-04，Apple Silicon macOS，Xcode 27.0 (27A266a)，Rust 1.94.0。真机/模拟器 ARM64 XCFramework 均已构建。iPhone 18 Pro 和 iPad Pro 11-inch (M5) 模拟器为 iOS 27.0。已配对 iPhone 18 Pro 与 iPad Pro 11-inch 第四代正在执行安装验收。

## 已执行

- Rust 桥接真实求解、响应式 3→6、源码文件不含配置、非法请求、中断与定义恢复。
- 原始 53 条数学语料经移动会话桥接，复用原 exact / 700-bit 残差权威校验，没有修改数学期望。
- FFI 空指针/非法长度、未知句柄、缓冲所有权释放及重复释放当前已释放缓冲的边界测试。
- 首轮 Swift 11 项测试通过：真实协议、53 条计算/公式解析、UTF-16/UTF-8/emoji、矩阵/分式/未知排版回退、Markdown 数学与代码隔离、UIDocument 往返、依赖重算、Keychain 隔离/失败回滚、URLProtocol 分块中文、HTTP 错误/重定向拒绝、全请求超时、探测不持久化。
- 首轮 iPhone 界面测试通过真实二次方程 −3/1 和步骤面板。
- iPad 真机自动签名 Release 构建与安装成功；完成用户签名信任后启动成功。

## 修复与复测记录

扩大验收发现 Modern 示例错误使用 `a=2`（该语法为方程）；改为现有规范的 `let a=2`。SwiftMath 对 `operatorname` 无解析支持，新增可逆显示兼容，原公式保留；全部语料公式解析已通过。首次扩大界面定位遇到 iOS 27 菜单的 Button/PopUpButton 类型变化，采用实际控件类型查询。

一次完整模拟器运行记录 #12 为 1984 ms、#41 为 1209 ms，超过 1 秒目标，保留为失败。发现编辑器对相同状态持续触发样式/选区刷新，已消除重复更新；最终 Release 复测和真机测量必须通过后才关闭性能门禁。

## 待补齐最终证据

完整 iPhone/iPad Release 模拟器、已配对真机、输入/旋转/窄窗口/键盘/无障碍/后台流程；最终性能与截图；取消、工具续轮、建议明确插入及保存失败的扩大覆盖；原平台回归与同提交 GitHub CI。完成项将用实际测试结果和附件更新。

## UI 参考与实现映射

使用用户指定的 [telegram-ui-reference skill](/Users/hert/Documents/ChatGPT/ui-learning/skill/telegram-ui-reference/SKILL.md)。只阅读 Markdown 实现契约，不复制 Telegram 源码或构建其项目。

| 本地 Markdown | OpenMath 行为及验证入口 |
| --- | --- |
| `09-adaptation-and-accessibility/screen-size-and-orientation.md` | 696 pt 阈值；稳定笔记本实例和模型；旋转 XCUITest |
| `02-layout-and-navigation/tabs-and-navigation-stacks.md` | 检查面板选择、源码和文档状态分离 |
| `02-layout-and-navigation/safe-areas-and-scroll-containers.md` | NavigationStack / safeAreaInset / 原生滚动容器统一处理安全区 |
| `02-layout-and-navigation/keyboard-avoidance.md` | 原生键盘避让与交互收起，不额外累加键盘高度 |
| `05-input-and-actions/adaptive-composers.md` | UIKit 编辑器有界自适应，触屏与键盘运行入口 |
| `05-input-and-actions/text-selection-and-editing.md` | 原生 TextKit 2、Unicode 位置契约、合成保护及补全 |
| `04-pages-and-flows/settings-pages.md` | 草稿/保存/回读/失败回滚，探测与保存分离 |
| `03-components/list-item-variants.md` | 原生 Form 目的、动作、说明行 |
| `03-components/switches-and-selection-controls.md` | 原生 Toggle/Picker，保存中禁止修改 |
| `09-adaptation-and-accessibility/system-fonts-and-long-text.md` | Dynamic Type、数学独立横向滚动、正文换行 |
| `09-adaptation-and-accessibility/voiceover-and-reduced-motion.md` | 数学源码语义、图形操作、减少动态效果时静态定位 |
| `01-visual-system/semantic-colors-and-themes.md` | 系统语义色及现有绿色强调色、浅深模式 |

## 扩大验收结果

- 最新原生测试集为 **18 项 Swift 单元 + 2 项 XCUITest**。iPad 真机和新建 iPad Pro 11-inch (M5) / iPadOS27 模拟器已通过；iPhone27模拟器亦通过。后续只在新增边界修复后复测，具体结果随交付更新。
- 原生 iPhone18Pro / iOS27.0.1：53条实际计算最大97.345ms；iPadPro11第四代 / iPadOS27.0：最大166.371ms。原始语料与数学期望不变，完整结果在 `ios-evidence/*-device-corpus.json`。
- 旧iPad模拟器实例的XCTWaiter启动失败在新建同型号、同27.0运行时实例上消失；旧失败保留。iPhone真机XCUITest驱动多次code74启动失败，未计为通过。随后用Xcode27 Device Hub直接控制真机，验证−3/1、步骤和暗色界面，并保存原始设备截图。用户要求手机留作其他用途后立即停止控制。
- Device Hub在iPad验证真实双栏、键盘选区/⌘Return运行（2+3→5）、希腊字母按钮、合成候选期间旧输出失效提示。输入法合成场景促使补齐所有运行入口及向内核同步的保护；专门回归证明未提交文本不进入CAS，提交后的3+4产生7。
- 修复Swift请求取消后HTTP超时的终止回馈丢失；保留实际LLM终止事件，并从未取消的收尾任务提交HttpEnd。URLProtocol全请求超时、取消、工具续轮/明确插入回归通过。
- 公式、代码/表格/强调中的Markdown数学、未知HTML原文、长式横向滚动、旧图形请求隔离和数字投影隔离已接入。字体与真实生产Rust依赖的许可随应用和XCFramework复制。
- 883项Rust、59项前端单元、dev29/prod27浏览器流程、9项Python契约、工作区Clippy/fmt、cargo-deny和纯WASM检查通过。

最终发布仍要求同提交GitHub门禁、完整附件与SHA256。尚未执行的无障碍和文件手工场景会保留明确记录，不能由截图或编译结果替代。

2026-10-04最终边界回归：新增行内数学不换行的真实UIHosting布局检查、未编辑已打开文件的bookmark恢复检查，当前20项Swift单元与2项XCUITest在iPad27真机通过。Device Hub经真实UIDocumentPicker打开6源单元格文件，未自动执行；用户点击运行全部后产生2、3、−3/1、正弦采样与(−2,2)区间。原生中文/emoji/强调/表格/行内分式/代码原文已截图。Markdown经系统分享面板导出后从设备回读，包含真实源码和输出，未含配置/凭据。手工发现行内排版强制换行和未编辑打开文件未记住路径，均修复并加入回归。GitHub首轮iPhone门禁通过，iPad随后因驱动启动超时失败；门禁改为每轮新建并关闭唯一27.0模拟器，保持原数学/性能阈值，菜单测试使用已观测控件的中心坐标，未增加重试或隐藏失败。

## 2026-10-05 收尾及用户范围调整

修复 GitHub 环境检查中 `xcodebuild -version | head -1` 导致的 Broken pipe / exit 134：完整读取版本响应后严格校验 27.0，继续严格要求两个 SDK 27。Windows 的中文语料读取显式指定 UTF-8；发行脚本、测试和 Cargo 文本输出也采用 UTF-8，CI 将隐式编码警告作为错误。

本地扩大运行发现两处测试之间的竞争：旧测试的自动保存覆盖恢复测试的标准 UserDefaults；已取消 URLSession 在后一个 HTTP fixture 开始后进入 URLProtocol，污染共享请求计数。恢复用例使用独立存储，HTTP 用例使用独立合成端点与计数，原有恢复和凭据不重发断言保持不变。模拟器结果文件名独立，退出后移除本轮临时设备，重复开发验证不覆盖证据。

iPad 真机在 Device Hub 最大字号（11）下完成真实求解，源码、公式、精确解和校验可见；启用减少动态效果后真实步骤面板正常操作。截图为 `ios-evidence/ipad-device-accessibility-largest-text.png` 和 `ipad-device-reduced-motion-steps.png`。字号恢复为 3，减少动态效果与 VoiceOver 均恢复为关闭。普通软件键盘出现后底栏位于键盘上方，未重复补偿遮挡高度。

用户明确要求本轮跳过旁白及其余人工适配杂项：VoiceOver 完整朗读/焦点遍历、浮动键盘、真机窄窗口和相关手工组合场景保留为未验证，不以截图或自动化测试替代。随后用户要求不再开启本机模拟器；运行中的测试已中断，确认无启动模拟器，后续模拟器门禁仅在 GitHub runner 执行。

本地高负载时 iPhone 模拟器 #12 / #13 的 2215 / 1429 ms 超过 1 秒，保留失败；该轮还暴露上述恢复测试竞争。之后 iPad 模拟器数学/恢复测试通过，HTTP fixture 共享计数失败及界面驱动启动终止保留记录。新修复的最终设备/CI 结果另行记录，原始数学期望、性能阈值与失败断言未放宽。

最终 iPad Pro 11-inch 第四代 / iPadOS27.0 Release 真机：**20 项 Swift 单元测试通过，无失败/跳过**，包括独立恢复存储、各 HTTP fixture 的取消/超时/UTF-8/错误/工具续轮断言。53 条计算最大 **164.373167 ms（#12）**，未知语料排版命令为空；完整输出与计时见 `ios-evidence/ipad-device-final-corpus.json`。最终 XCUITest 交由 CI 执行，真机此前两项界面流程证据继续保留。

## 最终云端门禁与发行

发行源码提交为 `c930fcdf13ce1bc2b698c5cafb3d78fb5a415ce2`。[同提交 CI](https://github.com/He-RT/om-openmath/actions/runs/37218720007) 全部通过：Rust 工作区 883 项测试及协议检查中的 1 项绑定导出测试、59 项前端单元、开发 29 / 生产 27 项浏览器流程、9 项 Python 契约、格式/Clippy/依赖与纯 WASM 检查。

回读实际 `.xcresult`，iPhone18Pro / iOS27 模拟器和 iPadPro11(M5) / iPadOS27 模拟器各 **20 项 Swift 单元＋2 项 XCUITest** 全通过，无失败/跳过。两轮原始 53 语料最大分别为 **536.732708 ms（#35）** 和 **304.069334 ms（#12）**；全部公式可排版，计时包含实际 Swift 请求往返。完整结果在 `ios-evidence/phone-ci-final-corpus.json` 和 `pad-ci-final-corpus.json`。保留历史失败，最终同源 CI 已满足 1 秒门禁。

[发行流程](https://github.com/He-RT/om-openmath/actions/runs/37218757011) 全通过，包含 Windows 两种安装包实际安装/中文窗口、macOS 权威数学/性能、Web 与原生移动附件。已公开 [v0.1.0-pre-alpha.2](https://github.com/He-RT/om-openmath/releases/tag/v0.1.0-pre-alpha.2)，九类分发文件加清单；此前 `.1` 保留。源码工程随标签交付，个人签名配置不上传。此后记录更新只修改文档，分发源码与清单提交不变。

公开附件完整下载后逐字节检查九类文件的大小、清单 SHA256 与 GitHub digest，清单自身的 GitHub digest 也一致；全部 ZIP 的 CRC 通过，标签准确指向发行源码 SHA。再次确认两个 ARM64 切片、模拟器应用版本/平台/许可和签名材料隔离；DMG 的 `hdiutil verify` 通过，公开 macOS CLI 显示 `.2` 并真实求解出 −3 / 1。回读记录与原始清单分别为 `ios-evidence/release-final-verification.json`、`release-manifest-pre-alpha.2.json`。最新版已重新安装并启动于 iPad，未重开本机模拟器。
