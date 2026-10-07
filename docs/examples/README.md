# 可运行科研与场景案例

以下源码需要 **OpenMath `0.1.0-pre-alpha.3`**；开发中的旧版本号不表示公开 `.2` 已支持新函数。`.omnb` 是原有 v1 源码笔记本，不包含输出、模型凭据或 Agent 会话。桌面/Web/移动端可通过「打开」选择笔记本，按顺序运行；不会在打开文件时自动执行。

## 地月系 L2

[源码](earth-moon-l2.om) · [笔记本](earth-moon-l2.omnb)。使用地月圆型限制性三体模型，旋转系质心原点、地月距离 384400 km，GM 取计划中的精确标称分数。只求月球外侧的局部根，给 50 位精度与足够迭代预算；不将求根失败当作坐标。

| 数量 | 本例结果 |
|---|---|
| 质心坐标 x | 444244.2226008393 km |
| 地心坐标 x | 448914.9072421657 km |
| 距月球中心 | 64514.9072421657 km |
| y、z | 0 |
| 无量纲平衡残差 | 约 −2.4055×10⁻⁵⁰ |

报告仍保留高精度数值；只在二维展示边界显式转为机器数。图中地球、质心、月球和 L2 的横坐标按真实位置比例，标记大小/文字偏移是逻辑像素，不表示天体大小。GM 和距离为模型标称值，数值位数不等于物理准确度；这不是某一天的真实星历位置。

```sh
om --no-config run docs/examples/earth-moon-l2.omnb
om --no-config export --input docs/examples/earth-moon-l2.omnb --format svg --output earth-moon-l2.svg
om --no-config export --input docs/examples/earth-moon-l2.omnb --format png --output earth-moon-l2.png
```

## 三维西瓜

[源码](watermelon.om) · [笔记本](watermelon.omnb)。包含整瓜条纹参数曲面、半瓜表皮、浅色皮层、红色切面及 12 个椭球瓜籽；位置变换、实际颜色和法线均由 Rust 生成。小瓜籽使用较粗的有限网格，保持完整案例在既有导出预算内。

桌面/Web 可旋转、缩放、平移、复位和导出 OBJ，原始世界网格可展开查看。无 WebGL2 明确提示且保留原式与 OBJ；iOS 明确显示三维尚未适配，继续支持二维和计算。OBJ 包含标准顶点/法线/面，以及常见 RGB 顶点扩展和完整数据注释；不声称阅读器一定支持颜色材质。

```sh
om --no-config run docs/examples/watermelon.omnb
om --no-config export --input docs/examples/watermelon.omnb --format obj --output watermelon.obj
```

CLI 不开启交互窗口。默认不覆盖已有导出文件；明确需要覆盖时加 `--overwrite`。导出成功取实际写入和字节回读回执。真实截图与验收记录见 [场景验收](../acceptance/r36b/README.md)。
