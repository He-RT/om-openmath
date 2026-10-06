# 真实图形与纯数据导出

[下一版账本](../plan/NEXT_RELEASE.md) · [验收](../acceptance/r35d/README.md) · [字体许可](../../licenses/plot-font/README.md)

本节为 `.3` dev 的 R3.5d 实施契约。版本仍为 `.2`；SVG/PNG/CSV/JSON为真实宿主/CLI能力，通用CAS `Export` / `export_string` 和Notebook Agent写入工具继续deferred，不因存在文件按钮就注册占位回调。函数描述版本23、235回调/228身份，GetCapabilities追加独立export_formats，不授予Agent任务权限。

## 已有数据与作用域

ExportPlot接收当前成功绘图的真实PlotData和固定显示参数，不求值表达式、不重采样、不在前端计算函数。PlotFigure包含坐标名、白色背景图尺寸、固定颜色/区域透明度、当前滑块值，SVG metadata及PNG UTF-8 iTXt保留完整原始数学坐标/曲线/网格值/向量/频数/跳过数和参数。参数曲线的横纵轴是坐标x/y，数学t区间不被当作相机横轴。

共享Rust图形流包含网格/坐标文字、连续段、区域/密度/热图矩形、原场箭头、散点和解点/范围；SVG和CPU PNG使用同一显示变换/裁剪。线段按内核分段，不跨孔洞重连；对数数据仍保存原数学坐标，显示只转换坐标。图形为有限采样示意，导出不能把它升格为认证解集或高精度图形。

ExportValue核对当前Done单元格、out_index/view_id和≤32层内部path；导出选中子树的全部数据，offset/column_offset不把结果缩成当前页。复用已有纯数据编码器，不执行字符串、源码或任意未求值数学头。CSV为UTF-8/CRLF，列顺序/引号/逗号/换行/空表头保留；JSON沿原纯数据数值规则，重复键、符号调用、不支持的非终止精确有理数均按真实诊断拒绝，不静默转机器值或Null。

临时探索结果的列表/矩阵/记录/表格携可选value_token：固定版平坦Expr图、Number二进制serde、100000节点/8MiB限制。此字段仅是不可变数据快照，不是权限或配置凭据，导出直接解码数据而不重新执行数学源式；主文档、随机状态和输出历史不变，.omnb仍仅保存原源码/initial。大型结果受共享预算，失败/取消不会给伪造文件成功。

## 字节与资源

Artifact包含mime/extension/base64/实际byte_len；返回仅确认字节生成。宿主校验类型与字节数再保存。图形320..2048×240..2048、总像素≤2000000、几何≤200000项、数学元数据≤8MiB、产物≤16MiB；使用实际共享interrupt/时钟，独立的导出遍历预算16Mi steps与最长10秒，不执行CAS。

原2MiB请求上限保留；只有ExportPlot/ExportValueToken两个纯产物请求允许≤16MiB，用于已有大几何/值图。WASM、桌面和FFI都先硬限长，再解析并按真实请求类型检查；不是泛化的文档/网络请求放宽。旧ABI和既有字段保持，新增类型是可选/追加协议。

元数据测试发现serde_json默认快速浮点解析存在末位漂移，已明确启用float_roundtrip，使f64→JSON→f64同值；这不改变数学源码小数的默认精度或把机器数充成高精度数。原53数学期望与整次1秒门禁继续。

PNG为8位RGBA、sRGB、无损stored zlib块、标准CRC32/Adler32与UTF-8 iTXt；没有GPU、截图伪图、系统字体路径或联网字体。Swash0.2.10仅光栅化固定OFL字体（仅std/scale/render），与科研数值/CAS/三维无关；内嵌重命名/子集字体约5.7MiB，构建不依赖Python字体工具。正常中文/希腊/拉丁及已有常用数学字形真实绘制；超出字体覆盖的PNG标签明确失败并提示SVG，不能输出空白字形。SVG保留Unicode文本和原式，字形显示取决于读者字体。字体来源、固定commit/工具版本、哈希和许可证随各分发保留。

参考原始规范：[W3C PNG第三版](https://www.w3.org/TR/png/)、[serde_json float_roundtrip](https://github.com/serde-rs/json/blob/master/Cargo.toml)、[Swash](https://github.com/dfrg/swash)、[Noto字体许可](https://github.com/notofonts/noto-cjk/blob/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Sans/LICENSE)。这些规范是算法依据，实际文件回读和像素证据另列验收。

## GUI、CLI与持久化

Web/桌面和SwiftUI有明确SVG/PNG、CSV/JSON入口。等待新采样/过期/失败图不导出；请求检查producer/视窗代次、liveness与当前参数，迟到结果不触发保存或把旧文件当新结果。标签/按钮保留静态路径；无动画也可完成导出。

浏览器成功只表示已启动真实下载；不能保证外部浏览器已落盘。Tauri系统保存对话框仅授权用户选择文件，写入后二进制读回相同才显示“文件已保存并回读校验”；取消不报成功。iOS生成临时文件后同样读回验证，再交系统分享/“存储到文件”，不把分享面板出现当外部文件提供商已持久化。

CLI无需交互窗口，示例均要求 `.3` dev：

```sh
om --no-config export -e 'plot(sin(x),x:-pi..pi)' --format svg --output sine.svg
om --no-config export -e 'field_plot([-y,x],x:-2..2,y:-2..2)' --format png --output field.png
om --no-config export -e '[[1,2],[3,4]]' --format csv --output table.csv
om --no-config export --input notebook.omnb --format json --output result.json
```

--input使用原宿主源码/笔记本加载路径并导出实际末个结果；指定-e与--input互斥。默认不覆盖现有文件，--overwrite是显式覆盖动作；写完sync/readback一致才输出persisted=true回执。非法源式/格式/维度、已有目标或错误输出类型失败，不能只创建空文件冒充导出。OBJ已在R3.6a接入，见[三维契约](scene3d.md)；完整场景/最终发行仍继续。

依据用户指定portable UI参考的progress-and-result-feedback（真实producer/取消/完成分离）和system-fonts-and-long-text（长标签/窄宽度/系统字体一次）落实静态路径；真实验证与未执行平台项区别记录。本机不启动模拟器。

依赖门禁已将初拟fontdue/ttf-parser路径替换为Swash0.2.10+Skrifa0.44实际依赖：RUSTSEC-2026-0192标明ttf-parser无维护版本，未添加忽略规则，保留完整deny门禁。此变更只涉及固定标签字体，不替换数学算法。
