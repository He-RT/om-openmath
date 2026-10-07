# 独立三维采样、显示与 OBJ

[下一版账本](../plan/NEXT_RELEASE.md) · [验收](../acceptance/r36a/README.md) · [原二维契约](plotting-2d.md)

本节为 `.3` dev 的 R3.6a 基础路径，应用版本仍是 `.2`。曲面、三维参数曲线/曲面、隐式零曲面与OBJ已接入；场景组合/图元/样式/变换与L2/西瓜已接，规格见[场景图](scene-graph.md)和[可运行案例](../examples/README.md)。完整发行仍须最终同SHA门禁。

## 入口与边界

实际held注册 `Plot3D`（fn119）、`ParametricPlot3D`（fn219）与 `ImplicitPlot3D`（fn220），描述版本26，兼容原大小写/Wolfram与原现代别名优先级。二维 implicit_plot 的两个轴仍解析为 ContourPlot/fn119，三轴才选三维入口；没有改变原有效二维表达式的数学含义。已有用户parametric_plot绑定仍优先。

```om
# 以下为.3 dev已接通接口。
plot(sin(x)*cos(y),x:-pi..pi,y:-pi..pi,view:"surface")
parametric_plot([cos(t),sin(t),t],t:0..2*pi)
parametric_plot([cos(u)*sin(v),sin(u)*sin(v),cos(v)],u:0..2*pi,v:0..pi,
  color:fn(p,u,v)=>[0.1,0.5+0.3*cos(12*u),0.2])
implicit_plot(x^2+y^2+z^2=1,x:-2..2,y:-2..2,z:-2..2,mesh_points:24)
```

现代plot一轴默认line、两轴默认surface，也可显式指定line/contour/density/surface；显示目的不同仍显式选择。三维参数图只支持三坐标一参数曲线或两参数曲面，二维二参数区域映射不属于本批。mesh_points为8..64分段，曲面默认48，隐式默认24。color是颜色字符串/#RRGGBB或只读函数，函数收到位置向量p，可另接全部数学轴参数；返回3/4个有限0..1实分量，越界/复数/高精度控制不静默截断或降级。源式保持，采样仅机器实数，不承诺任意精度网格或光照。

数学轴/域固定在Scene3DRequest，与相机分离；轴屏蔽主会话ownvalue，其他参数是实际只读依赖。原式先在只读局部作用域准备，再编译数值程序，color同样只读，不能赋值/清除或推进主随机状态。所有遍历/数值/颜色/三角化共享原Interrupt时钟/预算，取消返回真实失败，不回传部分网格伪造完成。

## 实际几何

Scene3DRequest/Scene3DData独立于原PlotRequest/PlotData，不向二维坐标偷偷加入第三项。Mesh保留原世界position、真实三角形索引、从实际三角形边计算的单位法线、RGBA。曲线按连续段保留世界坐标/颜色，常量参数曲线是实际point标记；不是强行画出小球代替数学位置。

标量/参数曲面采用固定网格、有限角点及中心域/可识别跳变检查，再由Rust生成实际三角形；孔洞附近不可用格不连接。有限网格可能漏掉窄结构/奇点，skipped是实际计数，不表示找到全部域外点。参数曲面的法线方向遵守参数顺序，不把任意参数化的法线称作总是外向。

隐式曲面使用规则坐标网格与六tetra分解；公共边缓存真实零点，经最多40次原函数探测与相对小端值残差检查。极点符号变化不能当作零点，边探测非有限/未达到残差即拒绝连接；三角形根据正/负标量侧定向。均匀网格不是完整解集或认证边界，空/退化/未采到有限曲面明确诊断。恒等零的体积集不伪造成二维曲面。

总顶点≤200000、所有位置/法线/颜色/索引/世界范围有限且形状一致。超限/不可表示尺度、高精度数字降级、错误维度、重复轴、未知参数、非法颜色/未收敛边都真实失败。数据可查原坐标、三角形、法线、颜色和跳过数，显示不重新计算函数。

## 桌面/Web与移动端

React WebGL2仅上传内核几何、计算相机/显示归一化、透视和法线显示变换。环境光+方向光使用实际法线，透明几何按视图深度排序近似混合；没有物理光线追踪承诺。相机保留旋转/倾角/归一化距离和平移，初始按已有网格显示半径拟合，窄窗口保留归一化中心和相对缩放。世界坐标从未被相机归一化覆盖，OBJ仍导出原坐标。

可见按钮/键盘方向键/Shift方向键、正负/Home和拖动/滚轮连接同一相机操作，静态路径完整，无必须动画；减少动态效果也保持同样结果。普通鼠标拖动旋转、Shift拖动平移。原数学域不随相机平移缩放重绑定，参数探索使用冻结数学上下文重算，相机由原producer视图保留。资源创建失败清理已分配shader/VAO/buffer/program，contextlost停止绘制，恢复或明确重试重建；卸载销毁GPU对象。

缺少WebGL2或编译/上下文失败时隐藏空canvas，给明确能力提示，保留原式、实际数据和OBJ导出；不展示合成图伪装三维支持。GUI基础路径可通过可访问控件、原坐标范围与网格数据完成，设备线宽超出WebGL实际范围时以支持范围绘制，原宽度仍在数学/图形数据中。

SetHostPlatform声明当前宿主渲染目标，独立于数学定义/Agent权限。FFI创建时明确iOS，普通/探索三维输出为data=None+unavailable说明及原式，不自动执行未展示的大网格；移动端SampleScene3D明确拒绝。Native SwiftUI显示真实文本和源码，不空白、不制作二维图片冒充三维。桌面/Web提供三维展示；CLI提供实际网格/OBJ而不承诺交互窗口。GetCapabilities区分这些支持与任务权限，实际WebGL2是否可创建仍由宿主检测。

## OBJ与证据

ExportScene3D直接检查并序列化已采样世界顶点/法线/索引/线/点，不重采样或运行数学源式；原文件保存/回读契约沿R3.5d。标准v/vn/f/l/p，v附RGB为常见扩展；RGBA与全部原采样数据保存在scene_json注释，阅读器是否采用RGB扩展取决于它的能力，不冒充通用材质/MTL/glTF支持。产物16MiB、原元数据8MiB，复用共享预算/中断，OBJ作为新增明确格式不开放通用Export/文件IO函数。

```sh
# .3 dev，无交互窗口；默认不覆盖已有文件。
om --no-config export -e 'parametric_plot([cos(t),sin(t),t],t:0..2*pi)' --format obj --output helix.obj
```

UI依据为用户指定本地telegram-ui-reference portable媒体相机、progress-and-result-feedback、system-fonts-and-long-text章节；数学域/相机/三维布局按本项目契约映射，未复制Telegram类体系。WebGL规范参考：[WebGL2官方接口](https://developer.mozilla.org/en-US/docs/Web/API/WebGL2RenderingContext)、[法线与基础光照](https://developer.mozilla.org/en-US/docs/Web/API/WebGL_API/Tutorial/Lighting_in_WebGL)、[上下文丢失事件](https://developer.mozilla.org/en-US/docs/Web/API/HTMLCanvasElement/webglcontextlost_event)。实际执行/截图/独立数学与OBJ回读证据见验收页，不能把参考算法或旧SHA成功当作本提交已完成。
