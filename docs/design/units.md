# 单位与量纲计算

本页是 `.3` 开发分支的真实实现说明，公开版本仍为 `.2`。单位换算使用精确有理因子；当前支持实数 Number 原子的标量量值，不自动近似根式或含符号表达式。

`quantity(value, unit: "m")` 或 `Quantity[value,"m"]` 构造量；单位必填，位置与命名写法二选一。value 若已是 Quantity，则进行相容量纲的换算。`convert_units(q,"cm")` 明确选择目标单位；`magnitude(q)` 返回当前单位下的数值，`unit(q)` 返回当前单位文字。纯单位文字解析不调用源码求值、文件或网络接口。

```text
quantity(1, unit: "km") |> convert_units("m") |> magnitude()  # 精确 1000
convert_units(quantity(1,unit:"km/h"),"m/s")                  # 精确 5/18 m/s
quantity(1,unit:"km") + quantity(250,unit:"m")                # 精确 5/4 km
quantity(2,unit:"cm")^2 |> convert_units("m^2")               # 精确 1/2500 m^2
quantity(decimal("0.1",precision:50),unit:"km") |> convert_units("m")
```

七维顺序为长度、质量、时间、电流、温度、物质的量、发光强度，基本单位是 m/kg/s/A/K/mol/cd。支持 [BIPM SI 前缀](https://www.bipm.org/en/measurement-units/si-prefixes)（含 2022 年 Q/R/q/r）；单位名大小写敏感，微前缀接受 µ/μ/u。质量前缀作用于 g，不接受 mkg 等双重前缀。具体基本单位依据 [BIPM SI 基本单位](https://www.bipm.org/en/measurement-units/si-base-units)。

派生名支持 Hz/N/Pa/J/W/C/V/F/ohm/Ω/S/Wb/T/H/lm/lx/Bq/Gy/Sv/kat/rad/sr。复合语法支持 `*`、`·`、空格乘积、`/`、括号及 `^` 后的有符号整数；不使用函数调用、脚本或任意变量。其他固定换算含 min/h/d、L/l、t 及 in/ft/yd/mi。

固定关系：1 min=60 s，1 h=3600 s，1 d=86400 s，1 L=1/1000 m³，1 t=1000 kg；1 in=127/5000 m，1 ft=381/1250 m，1 yd=1143/1250 m，1 mi=201168/125 m。英制长度使用国际定义，依据 [NIST 固定换算表](https://www.nist.gov/pml/special-publication-811/nist-guide-si-appendix-b-conversion-factors/nist-guide-si-appendix-b8)，不使用旧美国测量英尺。所有因子精确，原 Number 的精度不被凭空提高；未添加外部数学运行时。

加减要求量纲相同，使用第一个单位量的显示单位；裸数 0 是中性元素，非零裸数只允许与无量纲量相加。乘除和整数幂产生 SI 基准单位形式，零结果保留单位。单位运算先于核心规范化检查，不能因相消而跳过量纲错误。没有单位量的旧表达式保持原有核心路径。

范围限单位文字 256 字节、64 词元、括号 16 层、乘方 -32..32、每个维度指数 -128..128；原数值资源界限及 Interrupt 继续适用。复杂/符号量、非整数单位幂、摄氏/华氏等仿射温标、货币和联网知识库不在首版。七维量纲相同只表示可按本模型换算，不认证物理含义相同（例如 Hz/Bq 或 rad/sr）。`sin`、求解、积分等入口不自动获得任意单位表达式支持，可显式提取量值并保留单位。

验收见 [units.rs](../../crates/om-eval/tests/units.rs)：独立 SI/固定关系、电学等派生关系、真实乘除/相消、机器与已有高精度值、错误量纲/单位/指数及预算。专门的宿主单位结果展示在后续展示批次接续，`.omnb` 仍仅保存源码。
