<!-- Extracted from docs/plan/PLAN.md sections [8]. PLAN.md is authoritative; keep in sync. -->

## 8. 算法规格

> 本节是最难的部分。**严格按伪代码实现**。标 **[不确定]** 的地方表示 Mathematica 的确切行为未经核实：这类情况的测试一律用数值验证（`~`），不做字符串比较；我们自己的选择写在 `docs/solve.md` 的“差异”一节。

### 8.1 规范形式（om-core/canon.rs）

#### 8.1.0 数与传染规则
- `Rational` 必须约分且分母 > 0，分母为 1 时归一为 `Integer`。`Complex` 的实部和虚部属于同一类（`Complex[1, 2.]` → `Complex[1., 2.]`）。精确的 `im == 0` 归一为实部；近似的 `0.` 虚部则保留为复数。`I = Complex[0, 1]`。
- 传染：精确 ∘ Real → Real（取该 Real 的精度）；Machine ∘ Big → Machine；Big(p1) ∘ Big(p2) → Big(min(p1, p2))；Machine 运算溢出时改用 53 bit 的 Big 重算（**永远不产生** inf/NaN）。
- 特殊值：`DirectedInfinity[z]`（`Infinity = DirectedInfinity[1]`，`-Infinity = DirectedInfinity[-1]`），`ComplexInfinity = DirectedInfinity[]`，`Indeterminate`。解析器把 `Infinity` 符号直接转成 `DirectedInfinity[1]`。

#### 8.1.1 全序 `canonical_cmp(a, b)`（仅供内部使用；打印器另有显示顺序）
1. 两个都是数：先比实部（数值），再比虚部。数值相等时精确数在前，其次精度低的在前。
2. 数排在非数之前。
3. 其余情况用 `cmp_term`：令 `factors(e)` 为 `Times` 去掉首个数值系数后的参数列表（非 Times 时为 `[e]`）。**从最后一个元素往前**逐个用 `cmp_factor` 比较；全部相等时短的在前；仍相等再比数值系数（缺省视为 1）。
4. `cmp_factor(p, q)`：把各自看作 `(base, exp)`（非 Power 时 exp = 1）。先用 `cmp_atom` 比 base，再用 `canonical_cmp` 比 exp。
5. `cmp_atom`：Symbol < String < Normal。Symbol 按 `(name.to_lowercase(), name)` 比较（小写优先）；String 按字节比较；Normal 先用 `canonical_cmp` 比 head，再从左到右逐个比参数，最后比参数个数。

#### 8.1.2 `add(args)`（Plus）
1. 展平嵌套的 Plus。
2. 任一参数是 `Indeterminate` → 返回 `Indeterminate`。
3. 无穷：收集所有 `DI[z]` 项的方向。方向不同，或 `ComplexInfinity` 与其他无穷同时出现 → `Indeterminate`；否则无穷吸收所有有限项和符号项（`DI[z] + x → DI[z]`）。
4. 把所有数加总为 `c`（按传染规则）。
5. 每个非数项 t 拆成 `(coef, rest)`：`Times[k, r...] → (k, Times[r...])`（rest 只有一个因子时就是该因子），其他 → `(1, t)`。
6. 按 `canonical_cmp(rest)` 排序；rest 相同的合并（系数相加）；丢弃**精确 0** 系数的项。Real 系数保留（`x − 1.0x → 0. x`，再由 Times 规则变成 `0.`）。
7. 每项用 `mul([coef, rest])` 重建。`c` 为精确 0 时丢弃；`0.` 保留（`x + 0.` → `Plus[0., x]`，与 Mathematica 相同）。
8. 用 `canonical_cmp` 排序；0 项 → `0`，1 项 → 该项，否则 → `Plus[...]`。
**复杂度要求：** 先排序再合并，O(n log n)。禁止 O(n²) 的两两比较。

#### 8.1.3 `mul(args)`（Times）
1. 展平；应用 Indeterminate 规则。
2. 把所有数乘为 `c`。
3. 无穷：精确 0 × 无穷 → `Indeterminate`；否则 `c · DI[z] → DI[sign(c·z)]`，其中 `sign(w) = w/|w|`；ComplexInfinity 吸收。
4. `c` 为精确 0 → 返回 `0`；`c` 为 `0.` → 返回 `0.`。
5. 按 base 分组（`(base, exp)` 的含义同 `cmp_factor`），同一 base 的指数相加，用 `pow(base, sum)` 重建。结果可能是数（例如 `Sqrt[2]^2 = 2`），把它乘回 `c`。迭代到不动点（因子个数只减不增，所以一定终止）。
6. **数值根式合并：** 在形如 `Power[r, e]`（r 是正有理数，e 是非整数有理数）的因子中：e 相同的两个 → `pow(r1·r2, e)`；指数互为相反数（e 与 −e，e > 0）→ `pow(r1/r2, e)`。重复到没有可合并项为止。
7. **−1 分配：** `c == −1` 且除它以外只剩一个 Plus 因子时，分配进去：`-(1+x) → -1 - x`。Mathematica 只对 −1 这样做，`-2(1+x)` 保持不变。
8. 按 `canonical_cmp` 排序；`c` 不是精确 1 时放在最前面；0 个参数 → `1`，1 个参数 → 该参数。

#### 8.1.4 `pow(b, e)`（Power），按顺序判定
1. 任一为 Indeterminate → Indeterminate；`0^0`、`DI^0`、`1^DI` → Indeterminate。
2. `e` 为精确 0 → 1；`e == 1` → b；`b == 1` → 1。
3. `b == 0`：e 为数且 Re(e) > 0 → 0；e 为数且 Re(e) < 0 → `ComplexInfinity`，并发出消息 `Power::infy`；e 为符号 → 保持不求值。
4. `b = DI[z]`：e 为负数 → 0；e 为正整数 → `DI[z^e]`。
5. 两个都是数：Int/Rat 的 Int 次幂精确计算，结果超过 `MAX_EXACT_BITS = 2^24` bit 时保持不求值并发出 `General::ovfl`；任一方为 Real/Complex → 数值主值幂；**有理数的有理数次幂** `r^(p/q)`（q > 1）→ 第 6 步。
6. **根式提取**，`r = ±a/b`，指数 `p/q`：
   - r < 0 → `mul([pow(-1, p/q), pow(a/b, p/q)])`。
   - `(-1)^(p/q)`：把 p/q 对 2 取模，得到 e ∈ [0, 2)。e = 0 → 1；e = 1 → −1；e = 1/2 → I；e = 3/2 → −I；e ∈ (1, 2) → `-(−1)^(e−1)`；e ∈ (0, 1) → 保持 `Power[-1, e]`。
   - 把 p 写成 `sgn·(k·q + s)`，其中 0 ≤ s < q；结果 = `(a/b)^(sgn·k) · (a/b)^(sgn·s/q)`。
   - 提取 q 次方因子：`a = c^q · a'`，`b = d^q · b'`（`om_num::extract_root_factor`，只做有界试除（2^16 以下的素数）加上精确完全幂检验，**构造时不做完整因式分解**）；系数为 `(c/d)^(sgn·s)`，根式为 `(a'/b')^(sgn·s/q)`。
   - 完全幂约化：若 `a' = g^m` 且 `gcd(m, q) > 1`，改写后递归（`4^(1/4) → 2^(1/2)`）；b' 同理。
   - 输出：b' = 1 → `Power[a', t]`；a' = 1 → `Power[b', −t]`；否则 → `Power[Rational[a', b'], t]`。
7. `b = E` 且 `e = Log[z]` → z。`e = Times[Complex[0, r], Pi]`（r 是分母为 1 或 2 的有理数）→ ±1 或 ±I。
8. `b = Power[x, a]`：当 e 是整数，或者 a 是满足 −1 < a < 1 的实有理数（主值分支安全）时 → `pow(x, a·e)`；否则保持。
9. `b = Times[...]`：e 为整数 → `mul(每个因子 ^ e)`；否则把正有理数值因子提出来（`(2x)^(1/2) → Sqrt[2]·x^(1/2)`），剩余部分保持 `Power[Times[rest], e]`。
10. 其他 → `Power[b, e]`。

#### 8.1.5 规范形式测试向量（输入以 Wolfram 语法解析后 canonicalize，比较 FullForm）
| # | 输入 | FullForm |
|---|---|---|
|1| `a-b` | `Plus[a, Times[-1, b]]` |
|2| `x+x` | `Times[2, x]` |
|3| `2x+3x` | `Times[5, x]` |
|4| `x*x` | `Power[x, 2]` |
|5| `x^2*x^-2` | `1` |
|6| `y+x+2` | `Plus[2, x, y]` |
|7| `x^2+x` | `Plus[x, Power[x, 2]]` |
|8| `0*x` | `0` |
|9| `Sqrt[12]` | `Times[2, Power[3, Rational[1, 2]]]` |
|10| `8^(1/3)` | `2` |
|11| `Sqrt[2]Sqrt[3]` | `Power[6, Rational[1, 2]]` |
|12| `Sqrt[2]Sqrt[2]` | `2` |
|13| `Sqrt[2]/Sqrt[3]` | `Power[Rational[2, 3], Rational[1, 2]]` |
|14| `Sqrt[1/2]` | `Power[2, Rational[-1, 2]]` |
|15| `Sqrt[12/5]` | `Times[2, Power[Rational[3, 5], Rational[1, 2]]]` |
|16| `2^(-3/2)` | `Times[Rational[1, 2], Power[2, Rational[-1, 2]]]` |
|17| `4^(1/4)` | `Power[2, Rational[1, 2]]` |
|18| `(-1)^(1/2)` | `Complex[0, 1]` |
|19| `(-8)^(1/3)` | `Times[2, Power[-1, Rational[1, 3]]]` |
|20| `(-1)^(4/3)` | `Times[-1, Power[-1, Rational[1, 3]]]` |
|21| `(x^2)^(1/2)` | `Power[Power[x, 2], Rational[1, 2]]` |
|22| `(x^(1/2))^2` | `x` |
|23| `(x^(1/2))^(1/3)` | `Power[x, Rational[1, 6]]` |
|24| `(x y)^2` | `Times[Power[x, 2], Power[y, 2]]` |
|25| `(x y)^(1/2)` | `Power[Times[x, y], Rational[1, 2]]` |
|26| `(2x)^(1/2)` | `Times[Power[2, Rational[1, 2]], Power[x, Rational[1, 2]]]` |
|27| `1/2+1/3` | `Rational[5, 6]` |
|28| `1+2.5` | `3.5` |
|29| `x+1.0x` | `Times[2., x]` |
|30| `I^2` | `-1` |
|31| `(1+I)(1-I)` | `2` |
|32| `1/0` | `ComplexInfinity`（并发出 `Power::infy`） |
|33| `0/0` 与 `0^0` | `Indeterminate` |
|34| `Infinity-Infinity` | `Indeterminate` |
|35| `-2*Infinity` | `DirectedInfinity[-1]` |
|36| `I*Infinity` | `DirectedInfinity[Complex[0, 1]]` |
|37| `E^Log[x]` | `x` |
|38| `-(1+x)` | `Plus[-1, Times[-1, x]]` |
|39| `-2(1+x)` | `Times[-2, Plus[1, x]]` |
|40| `x^0` | `1` |

另外加一个 **proptest 属性测试**：随机生成深度 ≤ 4、由 `{x, y, 整数 −3..3, 1/2}` 组成的 Plus/Times/Power 树 t，检查 (a) `canonicalize(canonicalize(t)) == canonicalize(t)`（幂等）；(b) 把 x、y 代入随机有理数后，t 与 `canonicalize(t)` 的数值相等（相对误差 1e−9，跳过出现除零的样本）。

#### 8.1.6 初等函数特殊值（om-simplify/special.rs；om-eval 与 om-solve 共用）
- `Sin`/`Cos`/`Tan` 在 `r·Pi` 处取值，r 的分母 ∈ {1, 2, 3, 4, 6, 12}：先用周期性和对称性约化到 [0, π/2]，再查表（例如 `Sin[Pi/12] = (Sqrt[6]-Sqrt[2])/4`）。
- 奇偶性：`Sin[-x] = -Sin[x]`、`Cos[-x] = Cos[x]`，仅当参数是 `Times[负数, …]` 时才用。
- `ArcSin`/`ArcCos`/`ArcTan` 在 `{0, ±1/2, ±Sqrt[2]/2, ±Sqrt[3]/2, ±1, ±Sqrt[3], ±1/Sqrt[3]}` 处取值。
- `Log[1] = 0`、`Log[E] = 1`、`Log[E^k] = k`（k 为实数值）；`Exp` 就是 `Power[E, x]`。
- 精确参数是负实数时：`Log[-2] = Log[2] + I Pi`。
- `Sqrt` 就是 `Power[_, 1/2]`，已由 canon 处理。
- `Abs`/`Re`/`Im`/`Conjugate` 作用于数值时直接计算；`Abs[-x] = Abs[x]`。
- `ProductLog[0] = 0`、`ProductLog[E] = 1`、`ProductLog[-1/E] = -1`。
- 对数值参数（Real）一律调用数值实现求值。
**测试：** 表中每一项各写一条测试，另外对每一项在 50 位精度下做数值交叉验证。

### 8.2 多项式层（om-poly，只依赖 om-num）

**类型：**
```rust
pub trait Ring: Clone + PartialEq + Debug { fn zero() -> Self; fn one() -> Self; fn is_zero(&self) -> bool;
    fn add(&self, o:&Self)->Self; fn sub(&self,o:&Self)->Self; fn mul(&self,o:&Self)->Self; fn neg(&self)->Self; }
pub trait EuclideanRing: Ring { fn divrem(&self, o:&Self) -> (Self, Self); fn exact_div(&self, o:&Self) -> Option<Self>; }
pub trait Field: Ring { fn inv(&self) -> Option<Self>; }
// 实现：IBig(Ring+EuclideanRing)、RBig(Field)、FpElem{v:u64, p:u64}(Field)、MPoly<IBig>(Ring，用于 Z[参数])
pub struct UPoly<R: Ring> { pub coeffs: Vec<R> }   // 低次在前，末尾无 0；零多项式 = 空 Vec
pub struct Monomial { pub exps: SmallVec<[u32; 4]>, pub deg: u32 }
pub enum MonoOrder { Lex, GrevLex }
pub struct MPoly<R: Ring> { pub nvars: usize, pub terms: Vec<(Monomial, R)>, pub order: MonoOrder } // 按 order 降序，无零系数
```
所有可能耗时的函数最后一个参数都是 `ctx: &om_num::ctx::Interrupt`，返回 `Result<_, om_num::ctx::Abort>`（om-core 重导出相同类型）。

#### 8.2a 除法、伪余式、容量
- `divrem(f, g)`（在域上）：教科书长除法。
- `prem(f, g)`（在 Z 上）：返回 `lc(g)^(deg f − deg g + 1) · f mod g`。循环 `r ← lc(g)·r − lc(r)·x^(deg r − deg g)·g`，直到 `deg r < deg g`，记录迭代次数 k，最后乘以 `lc(g)^(δ+1−k)`。
- `content(f)`：各系数的 gcd，符号取得使 `pp(f) = f/content` 的首项系数为正。
- **测试：** `(x³−2x²−4) ÷ (x−3)` 得 `q = x²+x+3`、`r = 5`；`prem(x²+1, 2x+1) = 5`；`content(6x²+4x+2) = 2`，pp 为 `3x²+2x+1`；`content(−6x+3) = −3`，pp 为 `2x−1`。

#### 8.2b GCD：v1 = GCDHEU 快速路径 + 子结式 PRS 兜底（二者一致性用 proptest 验证）
```
heugcd(f, g):                      # f, g ∈ Z[x1..xn]，非零；对最后一个变量递归
  if n == 0: return igcd(f, g)
  gc = igcd(content(f), content(g)); f /= content(f); g /= content(g)
  B = 2*min(maxnorm(f), maxnorm(g)) + 29
  ξ = max(min(B, 99*isqrt(B)), 2*min(maxnorm(f)/|lc(f)|, maxnorm(g)/|lc(g)|) + 2)
  重复 6 次:
    ff = f(xn=ξ); gg = g(xn=ξ)                 # n−1 元多项式或整数
    if ff != 0 and gg != 0:
      h = heugcd(ff, gg)（递归；失败向上传播）
      for cand in [ interp(h, ξ), f / interp(ff/h, ξ), g / interp(gg/h, ξ) ]:   # "/" 为精确试除，除不尽就跳过
        c = pp(cand)
        if c | f and c | g: return gc * c（归一化：lc > 0）
    ξ = (ξ * isqrt(isqrt(ξ)) * 73794) / 27011
  失败 -> subresultant_gcd(f, g)

interp(h, ξ):   # 对称的 ξ 进制展开，对整数系数递归
  out = []; while h != 0: d = h mod ξ（对称剩余，取值 (−ξ/2, ξ/2]）; out.push(d); h = (h − d)/ξ
  return Σ out[i]·xn^i
```
`subresultant_gcd`：在 `Z[x1..x_{n−1}][xn]` 上做递归 Collins PRS（更新规则与 8.2e 结式循环相同），结果 = `pp(最后一个非零余式) · gcd(contents)`。
**测试：** `gcd(x²−1, x²−3x+2) = x−1`；`gcd(2x²+4x+2, 4x+4) = 2x+2`；`gcd(x⁴−1, x⁶−1) = x²−1`；`gcd(x²−y², x²+2xy+y²) = x+y`；`gcd(x, 0) = x`。

#### 8.2c 无平方分解
在 Q 上用 Yun 算法（先取本原部分）：
```
g = gcd(f, f'); c = f/g; d = f'/g − c'; i = 1
while deg c > 0: a = gcd(c, d); if deg a > 0: out.push((a, i)); c = c/a; d = d/a − c'; i += 1
```
在 Fp 上（f 首一）：
```
sqf_fp(f): if f' == 0: return [(h, m·p) for (h, m) in sqf_fp(pth_root(f))]   # pth_root：x^(pk) 的系数 → x^k（Fp 中 a^(1/p) = a）
  c = gcd(f, f'); w = f/c; i = 1
  while deg w > 0: y = gcd(w, c); z = w/y; if deg z > 0: out.push((z, i)); i += 1; w = y; c = c/y
  if deg c > 0: out += [(h, m·p) for (h, m) in sqf_fp(pth_root(c))]
```
**测试：** `(x−1)(x−2)²(x−3)³` → `[(x−1,1),(x−2,2),(x−3,3)]`；`x²+2x+1` → `[(x+1,2)]`；在 F3 上 `x³+1 = (x+1)³` → `[(x+1,3)]`。

#### 8.2d Z[x] 上的因式分解（Zassenhaus）
```
factor_Z(f): cont, f = content(f), pp(f); 提出 x^k; for (g, m) in yun(f): for h in zassenhaus(g): push (h, m)

zassenhaus(f):  # 无平方、本原、lc>0，n = deg f
  if n <= 1: return [f]
  从固定素数表（3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,73…，跳过 2）中取前 5 个满足
     p ∤ lc(f) 且 gcd(f̄, f̄') = 1 (mod p) 的素数；分别做 ddf+edf 得到因子数 r_p；取 r_p 最小者（并列取最小的 p）
  if r_p == 1: return [f]
  B  = (isqrt(n+1)+1) * 2^n * maxnorm(f) * |lc(f)|      # Mignotte 界 × lc
  l  = 最小的使 p^l > 2B 的 l；M = p^l
  L  = hensel_lift(p, f, mod p 首一因子列表, l)         # f ≡ lc(f)·ΠL_i (mod M)
  T = [0..r); s = 1; out = []; tried = 0
  while 2s <= |T|:
    found = false
    for S in combinations(T, s)（字典序）:
      ctx.tick()?; tried += 1; if tried > 65536: return out + [f]，并标记 PossiblyReducible
      tc = symmod(lc(f) * Π_{i∈S} L_i(0), M); if tc == 0 or (lc(f)*f(0)) % tc != 0: continue   # 常数项剪枝
      G = pp(symmod(lc(f) * Π_{i∈S} L_i, M))
      if G | f（在 Z 上精确整除）:
        out.push(G); f = f/G; T = T \ S; 从 L 中删去 S; found = true; break
    if !found: s += 1
  out.push(f); return out
```
Hensel 提升（二次收敛；von zur Gathen & Gerhard 算法 15.10；在平衡二叉因子树上进行）：
```
hensel_step(m, f, g, h, s, t):  # 前提 f ≡ gh (mod m)，sg + th ≡ 1 (mod m)，h 首一，lc(g) = lc(f)；运算都 mod m²
  e = f − g h;  (q, r) = divrem(s e, h);  g* = g + t e + q g;  h* = h + r
  b = s g* + t h* − 1;  (c, d) = divrem(s b, h*);  s* = s − d;  t* = t − t b − c g*
  return (g*, h*, s*, t*)        # 现在 mod m² 成立
hensel_lift(p, f, [f1..fr], l):
  if r == 1: return [ lc(f)^{-1}·f mod p^l（首一化） ]
  k = r/2; g = lc(f)·Π f1..fk, h = Π f(k+1)..fr (mod p); (s, t) = mod p 扩展 gcd
  m = p; 重复 ceil(log2 l) 次: (g, h, s, t) = hensel_step(m, f, g, h, s, t); m = m²
  mod p^l 约化; return hensel_lift(p, g, [f1..fk], l) ++ hensel_lift(p, h, [f(k+1)..fr], l)
```
Fp 上的 DDF/EDF（p 为奇素数）：
```
ddf(f): i = 1; h = x; out = []
  while deg f >= 2i: h = powmod(h, p, f); g = gcd(f, h − x); if g != 1: out.push((g, i)); f /= g; h = h mod f; i += 1
  if deg f > 0: out.push((f, deg f))
edf(f, d): if deg f == d: return [f]
  loop: a = 随机多项式(deg < deg f, 使用 SplitMix64); g = gcd(a, f); if 0 < deg g < deg f: return edf(g,d) ++ edf(f/g,d)
        g = gcd(powmod(a, (p^d − 1)/2, f) − 1, f); if 0 < deg g < deg f: return edf(g,d) ++ edf(f/g,d)
```
**陷阱：** Swinnerton-Dyer 多项式（如 `x⁴−10x²+1`）在**每个**素数下都分解成 1、2 次因子，重组是指数级的。所以有上限：超过上限就返回一个正确但未必不可约的分解，并标记 `PossiblyReducible`（LLL/van Hoeij 放到 v2）。`symmod` 必须取对称区间 (−M/2, M/2]。
**测试：** `x⁴+4 = (x²−2x+2)(x²+2x+2)`；`x⁶−1 = (x−1)(x+1)(x²−x+1)(x²+x+1)`；`6x²+x−2 = (2x−1)(3x+2)`；`x⁴+1` 不可约；`x⁴−10x²+1` 不可约；在 F3 上 `x⁴+1 = (x²+x+2)(x²+2x+2)`；在 F5 上 `ddf(x⁴−1) = [(x⁴−1, 1)]`。proptest：随机取 2–4 个次数 1–3、系数在 −5..5 的多项式 g_i，`factor_Z(Π g_i)` 各因子的乘积（带重数和 content）等于原多项式，且每个因子都整除原多项式。
**多元因式分解（Factor[x^2 − y^2]）：** v1 使用“Kronecker 代换 + 单元分解 + 试除”：把 y 代换为 x^D（D 大于 x 的次数上界），对单元多项式做分解，再对所有因子子集尝试逆代换并试除。上限 2^12 个子集，超出就只提取 content 和 gcd 公因子。v2 再换成 EEZ/Wang 算法。**测试：** `x²−y² = (x−y)(x+y)`；`x³−y³ = (x−y)(x²+xy+y²)`；`x²y+xy² = xy(x+y)`。

#### 8.2e 结式与判别式（Cohen 算法 3.3.7；D 可以是 Z 或 Z[参数]）
```
res(A, B): if A == 0 or B == 0: return 0
  a = cont(A); b = cont(B); A = pp(A); B = pp(B); g = h = 1; s = 1; t = a^deg B * b^deg A
  if deg A < deg B: swap(A, B); if deg A odd and deg B odd: s = −s
  loop:
    δ = deg A − deg B; if deg A odd and deg B odd: s = −s
    R = prem(A, B); A = B; B = R / (g * h^δ)          # 精确除法
    g = lc(A); h = g^δ / h^(δ−1)                        # 精确
    if B == 0: return 0
    if deg B == 0: break
  h = lc(B)^deg A / h^(deg A − 1); return s * t * h
disc(f) = (−1)^(n(n−1)/2) * res(f, f') / lc(f)
```
**测试：** `res(x²+1, x−1) = 2`；`res_y(x²+y²−1, y−x) = 2x²−1`；`disc(x²+bx+c) = b²−4c`；`disc(x³+px+q) = −4p³−27q²`；`res_y(y²−2, (x−y)²−3) = x⁴−10x²+1`。

#### 8.2f 实根隔离（Descartes 符号法则 + 二分 VCA）与 Aberth 复根
```
isolate(f):  # f 无平方，∈ Z[x]；返回按 lo 排序的 Vec<RootInterval{lo, hi: Rational, exact: bool}>
  out = []; if f(0) == 0: out.push(exact 0); f = f/x
  k = 满足 2^k > 1 + max_i |a_i/lc| 的最小整数          # Cauchy 界
  for sign in [+1, −1]: g = f(sign · 2^k · x)（取本原部分）
     stack = [(g, 0, 1)]                                 # g 在 (0,1) 中的根 t 对应原变量 sign·2^k·(a + (b−a)t)
     while pop (p, a, b):
       ctx.tick()?
       v = sign_variations( taylor_shift_1( reverse(p) ) )   # (0,1) 内根个数的 Descartes 上界
       if v == 0: continue; if v == 1: out.push(map(a, b)); continue
       m = (a+b)/2; pl = 2^deg(p) · p(x/2); pr = taylor_shift_1(pl)
       if pr(0) == 0: out.push(exact map(m)); pr = pr/x
       push (pr, m, b); push (pl, a, m)
  按 lo 排序后返回
refine(f, I, bits): 当 hi − lo > 2^−bits 时根据 sign(f(mid)) 二分；f(mid) == 0 时标记为精确根
```
**陷阱：** 映射负根时区间端点会翻转，存储时必须保证 `lo < hi`。
**测试：** `x²−2` 得到两个区间，分别包含 −1.4142 和 1.4142；`x³−2x` 有 3 个根，中间的是精确 0；`x⁴−10x²+1` 有 4 个区间，分别包含 ±0.31784 和 ±3.14626；`x²+1` 没有实根。

**Aberth–Ehrlich**（NSolve 与复根排序使用）：初值 `z_j = R·exp(i(2πj/n + 0.4))`，R 取 Cauchy 界；迭代 `w_j = (f/f')(z_j) / (1 − (f/f')(z_j)·Σ_{k≠j} 1/(z_j − z_k))`，`z_j −= w_j`，直到 `max|w_j| < 2^−(prec−4)`；最多 200 轮，停滞时精度加倍。**认证：** 用球算术计算圆盘 `D(z_j, n·|f(z_j)/f'(z_j)|)`，每个圆盘内至少有一个根；n 个圆盘两两不相交时，每个圆盘恰好含一个根。认证失败就提高精度重来。

#### 8.2g `Root[f, k]` 的编号顺序
Mathematica 文档：先是实根（升序），然后是非实根，共轭对相邻，并且 `Root[#^2+1&, 1] = −I`（虚部为负的在前）。不同共轭对之间的顺序 **[不确定]**（观察结果与“按实部升序”一致）。**我们的确定性规则：** 非实根按 `(Re 升序, |Im| 升序, Im 为负的在前)` 排序，用认证过的圆盘比较，直到键值能分开为止；如果精度到 `2^−400` 仍然重叠，就判定为并列，并按 |Im| 排序。
**测试：** `x³−2`：Root 1 ≈ 1.26，Root 2 ≈ −0.63−1.091i，Root 3 ≈ −0.63+1.091i；`x⁵−x+1`：Root 1 ≈ −1.1673，接着依次是 ≈ −0.1812∓1.0840i 和 ≈ 0.7649∓0.3525i 两对。

#### 8.2h Q 上的 Gröbner 基
算法是 Buchberger，配合 Gebauer–Möller 更新和 sugar 选择策略。系数取整数，每次约化后取本原部分，防止系数膨胀。多项式放在 arena 中，按 id 索引。
```
update(G, B, h):                     # Becker–Weispfenning
  C = [(h,g) for g in G]; D = []
  while C: p = (h, g1) = C.pop_front()
     if coprime(LM h, LM g1) or (C ∪ D 中不存在 (h, g2) 使 lcm(h,g2) | lcm(h,g1)): D.push(p)
  E = [(h,g) in D if not coprime(LM h, LM g)]
  B' = [(g1,g2) in B if not (LM h | lcm(g1,g2) and lcm(g1,h) != lcm(g1,g2) and lcm(g2,h) != lcm(g1,g2))] ++ E
  G' = [g in G if not LM h | LM g] ++ [h]; return (G', B')
groebner(F, ord):
  G = B = []; for f in F（按 LM 升序）: (G, B) = update(G, B, pp(f))
  while B: p = argmin_B (sugar, lcm 按 ord, 创建序号); h = NF(spoly(p), G)
     if h != 0: (G, B) = update(G, B, pp(h)); if h 是常数: return [1]
  return reduce(G)       # 去掉 LM 可被其他 LM 整除的元素；对每个元素做尾部约化；在 Q 上首一化；按 LM 升序排列
sugar(输入) = 全次数; sugar(spoly(f,g)) = max(sug f + deg(L/LM f), sug g + deg(L/LM g))，其中 L = lcm
```
- **零维判定：** 对每个变量 x_i，都存在某个 LM(g) 是纯幂 `x_i^k`。
- **维数：** 最大的变量子集 U，使得没有任何 LM 完全落在 `K[U]` 中（按子集大小从大到小搜索）。
- **策略：** 先计算 grevlex 基；若理想是零维且需要 lex 基，用 **FGLM** 转换；正维时直接计算 lex 基。
```
fglm(Ggr, lex):
  Blex = []; V = []（已化为行阶梯形的 NF 向量）; Glex = []; L = [1]
  while L: m = lex_min(L); L.remove(m)
    if 某个 LM(Glex) | m: continue
    v = NF_Ggr(m)，表示为 grevlex 标准单项式上的向量
    if v ∈ span(V): 解 v = Σ c_i V_i; Glex.push(m − Σ c_i Blex_i)
    else: Blex.push(m); V.push(v); L ∪= {x_j·m for all j}
  return Glex（排序后）
```
**测试（lex，x > y > z）：** `{x²+y²−1, x−y} → {x−y, y²−1/2}`；`{xy−1, x²−y} → {x−y², y³−1}`；cyclic-3 `{x+y+z, xy+yz+zx, xyz−1} → {x+y+z, y²+yz+z², z³−1}`；`{xy}` 不是零维，维数为 1；`{x+y, x+y+1} → {1}`。FGLM 的结果必须与直接计算 lex 得到的约化基完全一致（用 proptest 比较随机零维小系统）。

#### 8.2i 代数数（om-poly/alg.rs）
- `RealAlg { minpoly: UPoly<IBig>（本原、不可约、lc>0）, iv: (Rational, Rational) }`，1 次时直接是精确有理数；`ComplexAlg { minpoly, disk: CBall, index }`。
- 运算（RootReduce）：α+β 的零化多项式是 `res_y(p(y), q(x−y))`；αβ 的是 `res_y(p(y), y^deg q · q(x/y))`；1/α 的是 `reverse(p)`。对得到的多项式做因式分解，然后加细 α、β 的区间，直到 α∘β 的区间包围中恰好只含一个因子的一个根，由此选出正确的因子和根。
- 符号：一直加细到 0 不在区间内（最小多项式次数 ≥ 2 且不可约，所以值一定非零）。
- 比较：最小多项式相同时比较根的编号，否则加细到两个区间不相交。
- 次数上限 64，超过就返回 Unknown。
- **测试：** √2+√3 的最小多项式是 `x⁴−10x²+1`，根在 (3,4) 中；√2·√3 的零化多项式是 `(x²−6)²`，分解后得到 `x²−6`，根在 (2,3) 中；`Root[x³−2,1]³ − 2 = 0`。

### 8.3 Expr ↔ 多项式转换（om-simplify/convert.rs）
```rust
pub struct PolyView { pub gens: Vec<Expr> /*生成元：变量 + 非多项式子项*/, pub num: MPoly<RBig>, pub den: MPoly<RBig> }
pub fn to_rational_function(e: &Expr, vars: &[Expr]) -> PolyView;  // 分子分母形式（不约分）
pub fn from_mpoly(p: &MPoly<RBig>, gens: &[Expr]) -> Expr;           // 输出时使用规范构造器
```
**生成元归一化（最常见的静默 bug，必须测试）：** 同一个底数的分数次幂要合并成一个生成元，例如 `x^(1/2)` 与 `x^(1/3)` 合并为 `t = x^(1/6)`，并记录 `x = t^6`。`Sin[x]`、`E^x`、`Sqrt[2]` 以及参数符号都作为独立的生成元。`E^(2x)` 与 `E^x` **不在这里**合并，由 8.7 的核统一负责。
**测试：** `Sqrt[x] + x^(1/3)` 只有一个生成元 `x^(1/6)`，多项式为 `t³ + t²`；`(x^2-1)/(x-1)` 的 num 为 `x²−1`、den 为 `x−1`；`together(1/x + 1/y) = (x+y)/(x y)`。

### 8.4 零判定 `is_zero(e) -> Tri { Zero, NonZero, Unknown(ProbablyZero | NoInfo) }`（om-simplify/zero.rs）
- **L0（结构）：** 规范形式为 `0` 或 `0.` → Zero；非零数 → NonZero。
- **L1（有理标准形）：** 经过生成元归一化后做 together，分子多项式恒为 0 → Zero；若没有生成元且分子是非零常数 → NonZero。
- **L2（代数）：** 若所有生成元都是代数数（有理数的根式或 Root）：先做根式去嵌套，`Sqrt[a + b Sqrt[c]] = Sqrt[(a+r)/2] + sgn(b) Sqrt[(a−r)/2]`，条件是 `a² − b²c = r²`、r 为有理数且 a > 0；再按关系 `y^q − a` 约化并检查是否为 0；仍无法判定就用 RootReduce 求最小多项式（次数上限 64），最小多项式为 `x` 时才是 Zero。
- **L3（数值）：** 没有自由符号时，用 64/256/1024 bit 的球算术求值。某个球不含 0 → NonZero（这是严格结论）；所有球都含 0 且半径 < 2^−(p−20) → `Unknown(ProbablyZero)`。有自由符号时，在 3 个随机的高斯有理点上求值：只要有一个点 NonZero 就判 NonZero（说明表达式不恒为零）；全部都是 ProbablyZero → `Unknown(ProbablyZero)`。
- 不含超越函数的有理表达式，经过 L1 后**不会**得到 Unknown。
- **测试：** `Sqrt[2]*Sqrt[3] - Sqrt[6]` → Zero；`Sqrt[3+2 Sqrt[2]] - 1 - Sqrt[2]` → Zero；`Sin[x]^2+Cos[x]^2-1` → Unknown(ProbablyZero)；`Pi - 355/113` → NonZero；`(x+1)^2 - x^2 - 2x - 1` → Zero。

### 8.5 simplify（om-simplify/simplify.rs）
最佳优先搜索。可用变换：`together`、`cancel`、`expand`、`factor`、`factor_terms`、`denest`、`RootReduce`（仅作用于代数数）、`PowerExpand`（仅在明确假设下）、少量三角规则（`Sin²+Cos² → 1`，倍角公式双向）。代价为 `LeafCount`，其中整数计为 `1 + ⌈log10|n|⌉/4`，Root 计为 3。保留最优结果，按 hash 做记忆化，最多扩展 50 个节点，每个节点都调用 `ctx.tick()`。debug 构建中断言结果与输入的差经 L3 检验为零。
`expand`：乘法对加法完全分配，正整数次幂用多项式展开（二项式定理）。`together`：通分。`cancel`：通分后用 GCD 约去公因子。`factor`：`factor_Z`，结果按因子次数升序排列，常数在最前面（与 Mathematica 的 `Factor` 输出一致，例如 `Factor[x^2-1] = (-1 + x) (1 + x)`）。

### 8.6 Solve 分派（om-solve/dispatch.rs）
```
solve(input, vars?, dom, ctx):
 P0 归一化:
   把 List/And 展平为合取式；Or → 各个析取分支分别求解，结果取并集并去重（结构比较 + 对差值调用 is_zero）
   Equal[a,b] → 方程 a−b；链式 a==b==c → a−b 与 b−c；True → 丢弃；False → 返回 {}
   Unequal[a,b] → 排除条件 (a−b ≠ 0)；Element[v, D] → 该变量的定义域
   Less/LessEqual/...：若为一元且 dom 是 Reals（或由不等式隐含）→ reduce_ineq（8.9）；否则 Unevaluated，并发出 Solve::ineq
   未给出 vars：取 {Pi, E, I, C[_], 内置符号} 以外的自由符号，按名字排序；个数多于方程数时发出消息（见 7.5）
   没有方程：返回 All（仍受排除条件约束）
 P1 排除条件（作用于**原始**表达式，在任何化简之前）:
   对每个子项 Power[b, e]（e 为数值且 Re(e) < 0）：b ≠ 0
   Log[b]: b ≠ 0；Tan[u]、Sec[u]: Cos[u] ≠ 0；Cot[u]、Csc[u]: Sin[u] ≠ 0
   eq_i := numerator(together(eq_i))（分子分母的公因子**不约去**，由排除条件处理）
 P2 分类:
   poly    = 经过 P1 后每个方程都是 vars 的多项式（系数可以含参数）
   linear  = poly 且关于 vars 的全次数 ≤ 1
   方程数 = 1 且变量数 = 1 → univariate(eq, x)（8.7）
   linear → linear_system（8.8）；poly → poly_system（8.8）；否则 → nonpoly_system（8.8）
 P3 验证（8.9 之前的 8.8.4）、定义域过滤（8.8.3）、排除条件过滤、排序（8.6.1）、附上数值、写入步骤
```
#### 8.6.1 解的输出顺序（确定性规则）
按各变量规则右端的数值，对变量按字典序比较；每个值的比较顺序为：实数在非实数之前，然后按 Re 升序，再按 Im 升序；仍然并列就用 `canonical_cmp`。重根按重数重复输出（`Solve[(x-1)^2==0,x] = {{x->1},{x->1}}`）。带 C[k] 的解族按 C = 0 时的数值排序。测试中顺序 **[不确定]** 的用例按多重集比较。

### 8.7 一元方程
```
univariate(e, x):
  if e 是 x 的多项式: return poly_uni(e, x)
  K = e 中依赖 x 的最大非多项式“核”（生成元归一化之后）
  if K 全是 Power[b, p/q] 且 b 是 x 的有理式: return radical_path(e, x)（8.7.3）
  if 通过 8.7.4 的核统一能把 K 化为单个核 k(x): y 为新变量; P = e[k → y]
       if P 不含 x: ys = univariate(P, y); 对每个 y_i 求解 k(x) == y_i（8.7.4 反函数表）; 取并集返回
  if e = f(g(x)) − c 且 f 在反函数表中（从外往里剥）: 对每个分支 return univariate(g(x) − f^{-1}(c))
  if dom == Reals 且出现 Abs[u]: 分情况 u ≥ 0（Abs[u] → u）与 u < 0（Abs[u] → −u），分别求解后按各自的条件过滤
  else: 发出消息 Solve::nsmet; return Unevaluated
```
#### 8.7.1 `poly_uni(p, x)`
1. p ≡ 0 → All；p 是非零常数 → {}。含参数时，假定首项系数非零，并记录 `GenericAssumption` 步骤。
2. 系数全为数值（Q）时：`yun` → 对每个无平方部分做 `factor_Z` → 对每个不可约因子 h 调用 `roots_irr(h)`，每个根带上所在无平方部分的重数。
3. 含参数时：只用 content、x^k、次数 ≤ 2 的求根公式、二项式 `a x^n + b`、以及 `p(x^k)` 代换；其他情况返回符号形式的 `Root[p(#1)&, k]`。

#### 8.7.2 `roots_irr(h)`，d = deg h
- **d = 1**：`−h0/h1`。
- **d = 2**：`(−b ∓ Sqrt[D])/(2a)`，`D = b² − 4ac` 经过规范化（Sqrt 会提出平方因子，D < 0 时给出 `I·Sqrt[−D]`）。
- **二项式** `a x^d + b`：`c = −b/a`，根为 `c^(1/d) · (−1)^(2k/d)`（k = 0..d−1），用 `pow`/`mul` 构造，这样形式与 Mathematica 一致（例如 `-(−1)^(1/3) 2^(1/3)`）。
- **h(x) = g(x^m)**（取最大的 m > 1）：递归解 `g(y) = 0`（仅当 g 的根全部能用根式表示时），再对每个 `x^m = y_j` 用二项式规则。双二次方程属于这种情况。
- **回文多项式**（偶数次 2m）：令 `z = x + 1/x`，得到关于 z 的 m 次方程；解出 z_j 后，再解 `x² − z_j x + 1 = 0`。
- **d = 3**：`Cubics → False`（默认）时返回 `Root[h, 1..3]`；`Cubics → True` 时用 Cardano：代换 `x = t − b/(3a)` 得到 `t³ + pt + q`，`Δ = −4p³ − 27q²`，`u = (−q/2 + Sqrt[q²/4 + p³/27])^(1/3)`，`v = −p/(3u)`，三个根为 `u+v`、`ωu + ω̄v`、`ω̄u + ωv`，其中 `ω = (−1)^(2/3)`。Δ > 0（不可约情形）时中间量为复数，这是允许的。**[不确定]** Mathematica 的默认是否就是 Root，测试用数值验证。
- **d = 4**：`Quartics → False`（默认）时返回 `Root`；为 True 时用 Ferrari（预解三次式 → 两个二次式）。**[不确定]**
- **d ≥ 5**：返回 `Root[h(#1)&, k]`，k = 1..d，编号按 8.2g。
- **UI 补充**（“比 Mathematica 更现代”）：kernel 对输出中的 Root 对象，额外尝试 `ToRadicals`（即 cubics/quartics = true 重新求解），成功时在解卡片上提供“根式形式”切换，并始终给出 20 位数值。
- **Reals 下的实根判定：** 用 `isolate` 数出 h 的实根个数 r。对根式形式的根用认证的 Ball 求值：虚部球不含 0 的根是非实根；恰好剩下 r 个时，它们就是实根。否则提高精度，最多重试 3 次，仍失败就判为 Unknown，保留该根并发出消息。`Root[h, k]` 是实根当且仅当 k ≤ r。

#### 8.7.3 根式方程
- **隔离后乘方**（根式个数 ≤ 2 时使用，这样步骤可读）：循环最多 4 次：选出 q 最大的根式 `r = b^(1/q)`，把 e 写成 `c·r + rest`（利用 `r^q = b` 约化 r 的幂），发出 `IsolateTerm` 步骤；构造 `c^q·b − (−rest)^q` 并展开，发出 `RaiseToPower(q)` 步骤。
- **一般情况**：由内到外为每个根式引入 `y_i`，关系为 `y_i^{q_i} − b_i(x)`，令 `P = e[r_i → y_i]`；按逆序依次做 `P = res_{y_i}(P, y_i^{q_i} − b_i)`；最后用 poly_uni 解 `P(x)`。
- **必须执行：** 每个候选解都代入**原始** e（使用主值分支），用 is_zero 检验；只有 Zero（或按 8.8.4 通过数值验证）才保留，其余丢弃并发出 `DropExtraneous` 步骤。
- **测试：** `Sqrt[x+2] − x` 平方后得 `x² − x − 2`，候选 {−1, 2}，丢弃 −1；`Sqrt[x] + Sqrt[x−5] − 5` 得 {9}。

#### 8.7.4 超越方程
**核统一**（返回代换，或失败）：
- (i) 所有核都是 `E^(a_i x + b_i)`，a_i 为有理数：令 `d = lcm(a_i 的分母)`，`y = E^(x/d)`，每个核改写为 `E^{b_i} y^{a_i d}`。
- (ii) 核为 `c_i^(a_i x + b_i)`（c_i 是整数），且都是同一个最小底 g 的幂（`c_i = g^{m_i}`，用完全幂检测）：同理令 `y = g^(x/d)`。
- (iii) 否则把 `c^u` 改写为 `E^(u Log c)`，以 `Log c` 为系数重试 (i)；仅当最终只有一族核时才算成功。
- (iv) 同一个 u 的 Sin 与 Cos 同时出现：令 `y = E^(I u)`，`Sin → (y − 1/y)/(2I)`，`Cos → (y + 1/y)/2`，反解时 `u = −I Log[y] + 2π C`，之后做一次 simplify。

**反函数表**（`k(u) = v`；C = C[n] ∈ Integers，n 取下一个未用的编号）：
| k | 分支 |
|---|---|
| Sin | `ArcSin[v] + 2πC`、`π − ArcSin[v] + 2πC` |
| Cos | `−ArcCos[v] + 2πC`、`ArcCos[v] + 2πC` |
| Tan / Cot | `ArcTan[v] + πC`（v ≠ ±I）/ `ArcCot[v] + πC` |
| Sec / Csc | `±ArcSec[v] + 2πC` / `ArcCsc[v] + 2πC`、`π − ArcCsc[v] + 2πC` |
| E^u | `Log[v] + 2πI C`（v = 0 时无解） |
| a^u（a 为常数） | `(Log[v] + 2πI C)/Log[a]` |
| Log[u] | 当 −π < Im v ≤ π 时为 `E^v`（数值检验；符号情形用 ConditionalExpression），否则无解 |
| u^n（n 为整数） | 二项式根 |
| u^(p/q) | 候选 `v^(q/p)`，再按主值分支验证 |
| Sinh / Cosh / Tanh | `ArcSinh[v] + 2πI C`、`Iπ − ArcSinh[v] + 2πI C` / `±ArcCosh[v] + 2πI C` / `ArcTanh[v] + πI C` |
| ArcSin / ArcCos / ArcTan | 当 −π/2 ≤ Re v ≤ π/2 时为 `Sin[v]` / 当 0 ≤ Re v ≤ π 时为 `Cos[v]` / 当 −π/2 < Re v < π/2 时为 `Tan[v]`（边界细节 **[不确定]**） |
| u·E^u | Complexes：`ProductLog[v]`，并发出消息 `Solve::ifun`（**[不确定]**）；Reals：v ≥ −1/E 时为 `ProductLog[v]`，−1/E < v < 0 时再加上 `ProductLog[−1, v]` |

**输出形式：** `x -> ConditionalExpression[expr, Element[C[1], Integers]]`；有多个常数时条件为 `And[Element[C[1], Integers], ...]`。之后对 expr 应用特殊值表（`ArcSin[1/2] → Pi/6`）。
**Reals 下的解族过滤：** 形如 `u0 + k·C` 的解族，若 k 为纯虚数且 u0 为实数，则令 C → 0 并去掉条件；若 u0 对所有整数 C 都不是实数（u0 为数值，虚部非零，且 k 为实数），整族丢弃；其他情况保留，条件加上 `Element[expr, Reals]`。
**`Solve::ifun` 消息：** 只要用了反函数（Complexes 下 ProductLog、或 inverse_functions 路径未给出完整解族时），就发出 `Solve::ifun: Inverse functions are being used by Solve, so some solutions may not be found; use Reduce for complete solution information.`

### 8.8 方程组、定义域与验证

#### 8.8.1 线性方程组
构造增广矩阵 `[A | b]`，环 D 取 Z（先清分母）或 `Z[参数]`（MPoly<IBig>）。用 Bareiss 无分数消元：
```
prev = 1
对每个 k: 选主元 = 第一个 ≥ k 的行中 M[r][col] 不恒为零者（按列从左到右）
  有参数时：记录 GenericAssumption(pivot ≠ 0)
  for i > k, j > col: M[i][j] = (M[k][col]*M[i][j] − M[i][col]*M[k][j]) / prev   （精确除法）
  prev = M[k][col]
```
A 部分全为 0 而 b 部分非零的行说明方程组矛盾 → {}。然后在 `Q(参数)` 上回代并 cancel：主元变量用自由（非主元）变量表示，排在后面的变量作为自由变量（Mathematica 惯例）。存在自由变量时发出 `Solve::svars`。**测试：** `det [[2,1],[1,3]] = 5`；3×3 方程组唯一解；欠定方程组；矛盾方程组；带参数 `{a x + y == 1, x - y == 0}` → `{{x -> 1/(1 + a), y -> 1/(1 + a)}}`，并有 GenericAssumption `1 + a ≠ 0`。

#### 8.8.2 多项式方程组
```
poly_system(F, vars):         # lex 序：vars[0] > ... > vars[n−1]
  G = 零维 ? fglm(groebner(F, grevlex), lex) : groebner(F, lex)
  if G == [1]: return {}
  if 非零维: U = 最大独立集（优先选靠后的变量），把 U 当作参数；
     对每个所需的 g，若它关于其 LM 变量的次数 ≤ 2，就解这个三角形的 lex 基得到其余变量；否则发出 Solve::svars 并返回部分结果
  g = G 中只含 vars[n−1] 的一元元素; 对 g 的每个不可约因子 f（factor_Z）:
     Gf = groebner(G ∪ {f}, lex)                      # 拆分出分支
     if Gf 具有形状 {x_i − h_i(x_n)}_{i<n} ∪ {f}: 对 f 的每个根 ρ: x_i = h_i(ρ)（先 mod f 约化，再化简）
     else: 引入 t = x_n + Σ c_i x_i，c 取自固定序列 (1, −1, 2, −2, ...)；加入 t − Σ...，按 t 为最小变量的 lex 序重算，再检查形状（最多试 5 组）
```
f 的根来自 `roots_irr`；如果是 Root 对象，x_i 就是关于该 Root 对象的多项式（Mathematica 也是这样返回的）。
- **Eliminate：** 取 lex 基中不含被消元变量的元素，输出为 `And[... == 0]` 并化简成 `lhs == rhs` 的形式。
- **NSolve：** 走相同路径，但求根用 Aberth，结果全部取数值；工作精度默认为机器精度，也可以通过 `WorkingPrecision` 指定。
- **FindRoot：** 带 Armijo 回溯的阻尼 Newton（能求符号 D 时用解析 Jacobian，否则用数值 Jacobian）；给出区间 `{x, a, b}` 时用 Brent 方法。

**非多项式方程组：** 用代换法。反复选取 `(分支数, 表达式大小)` 最小、且把其他变量视为参数时 univariate 能求解的那一对（方程, 变量），把解代入其余方程。每一步消去一个变量，所以最多 n 层；总分支数上限 64。仍然失败时，把核当作新变量并加入已知关系（同一参数的 Sin/Cos 加入 `s² + c² − 1`），然后调用 `poly_system`。

#### 8.8.3 定义域
- **Reals：** 按 8.7.2 与 8.7.4 处理。
- **Rationals：** 先在 Complexes 上求解，只保留 RootReduce 后次数为 1 的解。
- **Integers：**
  - 一元：只保留满足 `a | b` 的一次因子 `a x + b`。
  - 线性方程 `Σ a_i x_i = c`：若 `g = gcd(a) ∤ c` → {}；否则用扩展 gcd 列变换求出幺模矩阵 U，使 `A U = [H | 0]`（列 Hermite 标准形），在整数中解 `H z = c`，剩余的 z 设为 `C[1], C[2], ...`，最后 `x = U z`。输出 `x_i -> ConditionalExpression[..., Element[C[1], Integers]]`。
  - 线性方程组：对整个矩阵做相同的 HNF。

#### 8.8.4 验证
对每个候选 σ 和每个**原始**方程 e，计算 `z = is_zero(e[σ])`：
- **Zero** → 保留，标记 `Exact`。
- **NonZero** → 丢弃，发出 `DropExtraneous`。
- **Unknown：**
  - 来自 poly_uni 因式分解、线性、FGLM 形状回代路径的候选 → 按构造正确，保留为 `ByConstruction`。
  - 来自根式、超越路径的候选 → 用 128 与 512 bit 的球算术求值；两个球都含 0 且半径 < 2^−(prec−16) 时，保留为 `Numeric{digits}`；否则丢弃，并发出 `Solve::verify`。
  - 解族：在 C = 0、1、−1 处检验。
- **含参数时：** 用 SplitMix64 取 3 组随机的小有理数代入参数，跳过使排除条件或 GenericAssumption 为零的点。
- **排除条件：** 若某个排除表达式在候选处为 Zero，丢弃该候选；为 Unknown 时，用数值检验的反面判定。
- **绝不**把根式、超越路径得到的根作为 `Unverified` 输出。

### 8.9 不等式（Reduce-lite，一元有理不等式）
```
reduce_ineq(f rel 0, x): 用 together 得到 f = n/d（不约分）
  crit = sqf(n) ∪ sqf(d) 的实代数根，排序后去重（用精确的 RealAlg 比较去重）
  在每个间隙里取一个有理测试点（相邻隔离区间之间；两端取 ±(界+1)）；s = sign(f(测试点))（精确计算）
  若 rel 在符号 s 下成立，则包含该区间；若 rel 非严格、且 n(c) = 0、d(c) ≠ 0，则包含端点 c
  合并相邻片段；输出 Wolfram 形式 Inequality/Or（例如 -2 < x < 2、x < -2 || x >= 1）
  以及供 UI 使用的区间形式 Vec<Interval>
```
发出 `SignChart` 步骤。`Reduce[eq, x]` 对纯方程等价于 Solve 的结果转成 `x == a || x == b` 的形式；`Reduce[ineq && ineq2, x]` 对两个区间集合求交集。v1 **不支持**多元不等式（返回 Unevaluated，并发出 `Reduce::nsmet`）。
**测试：** `Reduce[x^2<4,x]` → `-2 < x < 2`；`Reduce[(x-1)/(x+2)>=0,x]` → `x < -2 || x >= 1`；`Reduce[x^2>=0,x,Reals]` → `True`；`Reduce[x^2<0,x,Reals]` → `False`；`Reduce[x^3-x>0,x]` → `-1 < x < 0 || x > 1`。

### 8.10 步骤（Steps）数据模型（om-solve/steps.rs）
```rust
pub struct Steps { pub root: Vec<Step> }
pub struct Step {
    pub id: String,                 // "S1"、"S1.2"：稳定的层级编号，LLM 讲解时引用 [S3]
    pub rule_id: &'static str,      // 稳定的规则 ID，例如 "quadratic_formula"、"drop_extraneous"（UI 模板和 LLM 都靠它，与 Rust 枚举的布局无关）
    pub kind: StepKind,
    pub before: Vec<Expr>, pub after: Vec<Expr>,
    pub level: Level,               // Major | Minor（UI 默认只展开 Major）
    pub children: Vec<Step>,
}
pub enum StepKind {
  Normalize, RecordExclusion { cond: Expr, reason: ExclReason }, GenericAssumption { cond: Expr },
  ClearDenominators { factor: Expr }, Expand, Factor { factors: Vec<(Expr, u32)> }, ZeroProduct,
  SquareFree { parts: Vec<(Expr, u32)> }, Substitute { new_var: Expr, def: Expr }, BackSubstitute { var: Expr, value: Expr },
  ApplyFormula { formula: Formula /* Linear|Quadratic|Cardano|Ferrari|Binomial|Palindromic */, bindings: Vec<(String, Expr)>, results: Vec<Expr> },
  Discriminant { value: Expr, sign: Option<Sign> }, IsolateTerm { term: Expr }, RaiseToPower { n: u32 },
  Resultant { var: Expr, result: Expr }, InvertFunction { func: Symbol, branches: Vec<Expr>, constants: Vec<Expr> },
  RowReduce { op: RowOp, matrix: Vec<Vec<Expr>> }, Groebner { order: MonoOrder, basis: Vec<Expr> },
  Eliminant { var: Expr, poly: Expr }, SplitComponent { factor: Expr }, RootObjects { poly: Expr, real_count: u32 },
  Verify { candidate: Vec<(Expr, Expr)>, outcome: Tri, residual: Option<Expr> }, DropExtraneous { candidate: Vec<(Expr, Expr)>, why: String },
  DomainFilter { domain: Domain, kept: usize, dropped: usize }, SignChart { points: Vec<Expr>, signs: Vec<Sign> },
  Branch { label: String }, Note { msg: Message },
}
pub trait StepSink { fn push(&mut self, s: Step); fn enter(&mut self, label: &str); fn exit(&mut self); fn enabled(&self) -> bool; }
pub struct NoSteps; pub struct StepRecorder { /* 维护层级栈，自动分配 id */ }
```
- 步骤**必须**由真正执行计算的那段代码发出，不能事后重构。关闭步骤记录时用 `NoSteps`（零开销：先检查 `enabled()` 再构造 Step）。
- kernel 把 Steps 转为 `StepsView`（JSON）：每步给出 `id, rule_id, level, title_key（i18n 键 = "step." + rule_id）, params（字符串化的 LaTeX 映射，例如 {"a":"1","b":"2","c":"-3","disc":"16"}）, before_latex[], after_latex[], children[]`。前端按 `title_key` 查找 i18n 模板，例如 `"step.quadratic_formula": "应用求根公式：a = {a}, b = {b}, c = {c}"`。
- **每个 rule_id 都必须有 zh-CN 和 en 两套模板**，CI 用测试检查模板是否齐全（`app/src/i18n/steps.test.ts` 读取 `crates/om-solve/rule_ids.txt`，该文件由 `cargo test -p om-solve export_rule_ids` 生成）。
- **步骤快照测试（insta）：** `x^2+2x-3==0`（Factor → ZeroProduct → 2 个 Linear）、`Sqrt[x+2]==x`（IsolateTerm → RaiseToPower → Quadratic → Verify → DropExtraneous）、`Sin[x]==1/2`（InvertFunction）、线性 3×3（RowReduce×n → BackSubstitute）、`(x-1)/(x+2)>=0`（SignChart）。
