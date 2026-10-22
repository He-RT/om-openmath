# Git 整改执行结果

2026-10-22 至 23，按用户授权完成 R1–R3；R4 原始证据迁移保留待单独确定。基线为 `3ca8c4161042fac1a89a9d3915270adbc50946a2`。默认集成分支继续为 `dev`，没有新增或合入 `main`。

## R1：规则与保留边界

[PR #2](https://github.com/He-RT/om-openmath/pull/2)，规则提交 `81497d76`；集成提交 `9225a4e9ec5e80ad9b4cff2d7b2e2b3b65070940`。

- 补子目录 IDE/workspace、npm 调试输出及 Mac/iOS 根临时原生结果规则；15 个应忽略和 17 个应保留样例通过，跟踪且命中忽略的路径为零。
- `.gitattributes` 只删除空行；基线 24,561 个跟踪路径的 whitespace/diff/merge/text/eol/binary 结果完全一致，没有扩大属性范围，没有 renormalize。
- 22,083 个原有验收/设计文件逐文件 SHA256 未变；数据库、WAL/SHM、锁、blob、OBJ、SSE、验收日志、失败根、发行包和本地个人目录均保留。
- 当前操作入口没有个人绝对路径；历史设计来源、旧验收与明确授权的真实 API 入口按用途保留，未机械替换。

本地快照、属性结果及逐文件散列在 `target/git-remediation/`；未将其展开到验收目录。

## R2：CI、PR 与 dev 保护

CI 不再通过顶层 `paths-ignore` 消失。轻量检查始终执行；路径分类选择适用任务，未知构建输入以及实际被代码使用的设计、示例和协议/测试输入按源码处理。iOS 模拟器仍须显式派发。汇总 `CI required` 逐项核对成功和有依据的跳过，失败、取消、缺失或意外跳过均拒绝放行。

| 实际运行 | 结果与覆盖 |
|---|---|
| [39791648828](https://github.com/He-RT/om-openmath/actions/runs/39791648828) | 既有封装脚本的浏览器环境未声明，frontend lint 失败；汇总同步失败，其余适用任务成功 |
| [39810962276](https://github.com/He-RT/om-openmath/actions/runs/39810962276) | 修复 `be834d79` 后，Rust、frontend、dependencies、native、轻量和契约检查全部成功；iOS 按范围跳过，未派发 full_suite |
| [39811313514](https://github.com/He-RT/om-openmath/actions/runs/39811313514) | 纯文档有意行尾空白探针：轻量及汇总失败；产品构建按分类跳过 |
| [39816335017](https://github.com/He-RT/om-openmath/actions/runs/39816335017) | 受保护 dev 上的实际文档 PR 失败；普通合并请求被服务器拒绝，PR 保持 OPEN |
| [PR #4 最新检查](https://github.com/He-RT/om-openmath/pull/4/checks) | 追加修复后的文档 PR 轻量及汇总成功，可正常合并；无产品构建 |

原生步骤完成后 GitHub 曾延迟返回 job 终态；取消请求返回运行已结束，没有实际取消或额外重跑，最终以正式 success 为准。发行来源 PR 的相关检查见 [39816308420](https://github.com/He-RT/om-openmath/actions/runs/39816308420)。

远程 `dev` 已启用 branch protection：要求 PR；required check 为 GitHub Actions（app_id `15368`）的 `CI required`，strict=true；管理员同样受约束；禁止强推与删除；所需审批数为 0，不强设第二位维护者。修改前为未保护、rulesets 为空；前后 API 结果与请求配置保存在 `target/git-remediation/dev-protection-*.json`。没有使用 `[skip ci]`、强推或管理员绕过来集成失败状态。

维护恢复：优先在 PR 中修复检查或恢复 `CI required` 名称。仅在错误配置导致检查永远无法产生且 PR 无法修复时，由管理员保存现值，通过 GitHub 设置或 protection API 修正；改名前值为无保护，可用 `gh api -X DELETE repos/He-RT/om-openmath/branches/dev/protection` 恢复该前值，再用保存的 PUT 配置恢复门槛。此路径不用于绕过产品失败；恢复操作必须留记录。

## R3：发行来源

[PR #3](https://github.com/He-RT/om-openmath/pull/3)，集成提交 `f44a20183365c1693f5a105fa3db60abb7eb4c4c`；[PR #5](https://github.com/He-RT/om-openmath/pull/5) 后续修正版本采集的工作目录，集成为 `c855795577a48c63a5a6282dcab7c70fba5bcabe`。入口为 `python3 scripts/release.py sources {record,index,verify}`；具体操作见[发布流程](releasing.md#统一来源索引后续构建)。

- Mac/Web、CLI、Tauri 成功构建后生成独立 source sidecars，记录实际产品/打包/工作流 SHA、参数、工具版本与镜像、依赖散列、run URL、检查范围及资产大小/散列；Windows 的两份外部检查脚本覆盖另记来源，未提交产品修改被拒绝。
- 索引复用原 manifest，要求每个资产恰好一份来源；错误提交、缺失依赖、错平台/工作流/散列、重复来源、资产篡改和覆盖现有输出均拒绝。新版本将索引及引用的 sidecars/manifest 一起持久发布。
- 29 项 Python 检查、工作流 actionlint 和相关 PR 的契约 CI 通过。工具版本在产品检出目录采集，以匹配 rustup 的实际工具链选择；该修复的 12 项相关检查及 [39817399687](https://github.com/He-RT/om-openmath/actions/runs/39817399687) 通过。新增记录步骤未重新派发平台打包，下一次构建再实际执行；不把静态检查描述为新安装验收或字节级可复现。
- 现有 CLI 两包演练：从 origin 实际取回产品 `86da06206c1ce547921ff2fb6ada892c5b090b48` 和打包 `657d57aef33350d54ce122a67691c7170de64a59`，复用原 manifest 核对两包，并验证源工作流及依赖散列。演练索引在 `target/git-remediation/cli-source-replay/cli-sources-replay-index.json`，SHA256 为 `c42f1740541f105db57322de9a3dcfd46f902d06481adacb126ee8be770ac605`。旧工具信息来自原成功 run 日志，缺失的精确版本明确记为未知，不伪装为本轮构建时采集。

`.1`–`.5` 标签和既有公开资产没有移动或改写；`.5` 仍为多提交来源。不重跑冻结发行、真实模型或本机 iOS 模拟器，不迁移/取消跟踪 R4 原始根，不改写历史。最终集成 SHA 的检查按其实际变更范围另有 dev push 结果；上表旧 SHA 的结果不冒充最终发行验收。
