import type { Locale } from '../../i18n';
import type { ValueKind } from '../../kernel/generated/ValueKind';
import type { ValueNature } from '../../kernel/generated/ValueNature';
// Namespace imports keep the producer's machine vocabulary separate from user-authored record keys.
export const valueKinds: Record<string, [string, string]> = {
  scalar: ['表达式', 'Expression'], list: ['列表', 'List'], matrix: ['矩阵', 'Matrix'],
  record: ['记录', 'Record'], table: ['表格', 'Table'], model: ['拟合模型', 'Fitted model'],
  interpolation: ['插值数据', 'Interpolation'], series: ['有限级数', 'Finite series'], quantity: ['量与单位', 'Quantity'],
};
const fields: Record<string, [string, string]> = {
  value: ['结果值', 'Value'], solution: ['连续解', 'Solution'], point: ['参数点', 'Point'],
  model: ['模型', 'Model'], parameters: ['拟合参数', 'Parameters'], bindings: ['变量值', 'Bindings'],
  converged: ['计算状态', 'Computation state'], guarantee: ['数学保证', 'Guarantee'], scope: ['模式', 'Scope'],
  goal: ['目标', 'Goal'], method: ['方法', 'Method'], termination: ['停止原因', 'Termination'],
  error_estimate: ['误差估计（非证书）', 'Estimated error (not a certificate)'], residual_norm: ['残差范数', 'Residual norm'],
  residuals: ['实际残差', 'Actual residuals'], sum_squares: ['残差平方和', 'Residual sum of squares'],
  sum_squares_status: ['平方和表示状态', 'Sum of squares status'], rms: ['均方根残差', 'RMS residual'],
  rms_status: ['RMS 表示状态', 'RMS status'], numerical_rank: ['数值秩', 'Numerical rank'],
  iterations: ['实际迭代', 'Iterations'], evaluations: ['实际调用', 'Evaluations'],
  accepted_steps: ['接受步', 'Accepted steps'], rejected_steps: ['拒绝步', 'Rejected steps'],
  data: ['样本与预测', 'Samples and predictions'], certificate: ['证书数据', 'Certificate data'],
  null_space: ['自由方向', 'Free directions'], domain: ['定义域', 'Domain'], precision: ['精度模式', 'Precision'],
  model_evaluations: ['模型调用', 'Model evaluations'], jacobian_evaluations: ['Jacobian 调用', 'Jacobian evaluations'],
  variables: ['输入变量', 'Variables'], parameter_names: ['参数名', 'Parameter names'],
  faces_examined: ['检查的约束面', 'Faces examined'], projected_gradient_norm: ['投影梯度范数', 'Projected gradient norm'],
  bracket_width: ['剩余区间宽度', 'Remaining bracket width'], optimal_set: ['最优集合形式', 'Optimal set'],
  sample_count: ['样本数', 'Samples'], parameter_count: ['参数数', 'Parameters'], degrees_of_freedom: ['自由度', 'Degrees of freedom'],
  rank_tolerance: ['数值秩阈值', 'Rank threshold'], damping: ['阻尼', 'Damping'], gradient_cosine: ['梯度夹角余弦', 'Gradient cosine'],
  event_evaluations: ['事件调用', 'Event evaluations'], abs_tol: ['绝对容差', 'Absolute tolerance'], rel_tol: ['相对容差', 'Relative tolerance'],
  intervals: ['实际区间', 'Intervals'], residual: ['重构残差', 'Reconstruction residual'],
  lower: ['下三角因子', 'Lower factor'], upper: ['上三角因子', 'Upper factor'], permutation: ['行置换', 'Permutation'],
  values: ['数值', 'Values'], vectors: ['向量', 'Vectors'], sweeps: ['实际扫描', 'Sweeps'], rotations: ['实际旋转', 'Rotations'],
};
const statuses: Record<string, [string, string]> = {
  certified_global: ['精确全局认证', 'Certified global'], numerical_stationary_candidate: ['数值驻点候选', 'Numerical stationary candidate'],
  numerical_bounded_candidate: ['数值有界候选', 'Numerical bounded candidate'], numerical_local_fit: ['数值局部拟合', 'Numerical local fit'],
  numerical_linear_least_squares: ['数值线性最小二乘', 'Numerical linear least squares'],
  residual_tolerance: ['达到残差容差', 'Residual tolerance met'], gradient_tolerance: ['达到梯度容差', 'Gradient tolerance met'],
  overflow: ['超出机器表示范围', 'Outside machine range'], underflow: ['低于机器表示范围', 'Below machine range'],
  finite: ['有限可表示', 'Finite'], machine: ['机器精度', 'Machine precision'], local: ['局部', 'Local'], global: ['全局', 'Global'],
  end: ['到达终点', 'End reached'], event: ['事件终止', 'Event termination'],
  exact_ldlt: ['精确 LDLᵀ', 'Exact LDLᵀ'], linear_qr: ['线性 QR', 'Linear QR'], levenberg_marquardt: ['Levenberg–Marquardt', 'Levenberg–Marquardt'],
  brent: ['Brent 有界搜索', 'Bounded Brent'], bfgs: ['BFGS 局部搜索', 'Local BFGS'],
  affine: ['仿射自由方向', 'Affine free directions'], one_certified_box_optimum: ['一个盒约束认证点', 'One certified box optimum'],
  min: ['最小化', 'Minimize'], max: ['最大化', 'Maximize'],
};
export function pair(value: [string, string] | undefined, language: Locale, fallback: string): string {
  return value?.[language === 'zh-CN' ? 0 : 1] ?? fallback;
}
export function kindLabel(kind: ValueKind, language: Locale) { return pair(valueKinds[kind], language, kind); }
export function fieldLabel(label: string, language: Locale, authoritative: boolean) { return authoritative ? pair(fields[label], language, label) : label; }
export function statusLabel(source: string, language: Locale, authoritative: boolean): string | null {
  if (!authoritative) return null;
  try { const value: unknown = JSON.parse(source); return typeof value === 'string' && statuses[value] ? pair(statuses[value], language, value) : null; }
  catch { return null; }
}
export function natureLabel(nature: ValueNature, language: Locale): string {
  return pair({ exact: ['精确数', 'Exact number'], machine: ['机器数', 'Machine number'], high_precision: ['高精度数', 'High precision'], symbolic: ['符号', 'Symbolic'], text: ['文本', 'Text'], boolean: ['布尔', 'Boolean'], null: ['空值', 'Null'] }[nature] as [string,string], language, nature);
}
