import AppKit
import SwiftUI

struct NativeStepsView:View {
  @ObservedObject var model:NativeStepsModel
  var body:some View {
    VStack(alignment:.leading,spacing:12) {
      if let page=model.page {
        if !page.recorded {Text("这个结果没有记录步骤").foregroundStyle(.secondary)}
        else if page.total==0 {Text("记录中没有步骤。").foregroundStyle(.secondary)}
        else {
          ForEach(page.entries,id:\.step.id) {entry in
            Divider()
            NativeStepDetail(entry:entry,identity:model.original.result_ref)
            if entry.child_count>0,entry.path.count<=128 {
              DisclosureGroup(isExpanded:Binding(get:{model.expandedPaths.contains(entry.path)},set:{expanded in
                if expanded {model.expandedPaths.insert(entry.path)} else {model.expandedPaths.remove(entry.path)}
              })) {
                if model.expandedPaths.contains(entry.path) {
                  NativeStepsView(model:model.childModel(for:entry)).padding(.leading,18).padding(.top,8)
                }
              } label: {
                Text("\(entry.child_count) 个子步骤").font(.system(size:12)).foregroundStyle(.secondary)
              }.accessibilityIdentifier("step-children-"+entry.path.map(String.init).joined(separator:"-"))
            }
          }
          if page.total>NativeStepsModel.pageSize {
            HStack {
              Button("上一页") {Task {await model.load(offset:page.offset>=NativeStepsModel.pageSize ? page.offset-NativeStepsModel.pageSize:0)}}.disabled(page.offset==0 || model.busy)
              Spacer()
              Text("\(page.offset+1)–\(page.offset+UInt32(page.entries.count)) / \(page.total)").font(.caption).foregroundStyle(.secondary)
              Spacer()
              Button("下一页") {Task {await model.load(offset:page.offset+NativeStepsModel.pageSize)}}.disabled(UInt64(page.offset)+UInt64(NativeStepsModel.pageSize)>=UInt64(page.total) || model.busy)
            }
          }
        }
      }
      if model.busy {WorkbenchProgress("正在读取步骤…").controlSize(.small)}
      if let error=model.error {
        Text(error).font(.callout).foregroundStyle(.secondary)
        Button("重新读取") {Task {await model.load()}}.disabled(model.busy)
      }
    }.frame(maxWidth:.infinity,alignment:.leading)
      .task {if model.page==nil {await model.load()}}
      .accessibilityIdentifier("native-steps-"+model.original.result_id)
  }
}
private struct NativeStepDetail:View {
  let entry:NativeStepEntry,identity:String
  private static let textFields:Set<String>=["verification","method","reason","formula","sign","op","order","signs","label","symbol","tag","text","level"]
  var body:some View {
    VStack(alignment:.leading,spacing:10) {
      HStack(alignment:.firstTextBaseline,spacing:10) {
        Text(entry.path.map{String($0+1)}.joined(separator:".")).font(.system(size:12).monospacedDigit()).foregroundStyle(.secondary)
        Text(Self.titles[entry.step.rule_id] ?? entry.step.rule_id).font(.system(size:12.5,weight:.semibold))
      }
      if !entry.step.after_latex.isEmpty {
        formulas(entry.step.after_latex,label:"",key:"after")
        if !entry.step.before_latex.isEmpty,entry.step.before_latex != entry.step.after_latex {
          DisclosureGroup("变换前") {formulas(entry.step.before_latex,label:"",key:"before")}.font(.caption).foregroundStyle(.secondary)
        }
      } else {formulas(entry.step.before_latex,label:"",key:"before")}
      if let explanation=entry.step.params["reason"] ?? entry.step.params["text"],!explanation.isEmpty {
        Text(explanation).font(.system(size:12)).foregroundStyle(.secondary).textSelection(.enabled)
      }
      if !entry.step.params.isEmpty {
        DisclosureGroup("参数与条件") {
          VStack(alignment:.leading,spacing:10) {
            ForEach(entry.step.params.keys.sorted(),id:\.self) {key in
              VStack(alignment:.leading,spacing:4) {
                Text(Self.parameterLabels[key] ?? key).font(.caption).foregroundStyle(.secondary)
                if let value=entry.step.params[key] {
                  if Self.textFields.contains(key) {Text(value).font(.callout).textSelection(.enabled)}
                  else {NativeMathView(request:.init(identity:identity+entry.step.id+"/param/"+key,originalLatex:value,wolfram:"",modern:nil,fontSize:15),showsControls:false)}
                }
              }
            }
          }.padding(.top,6)
        }.font(.caption)
      }
    }.frame(maxWidth:.infinity,alignment:.leading).padding(.vertical,4)
  }
  @ViewBuilder private func formulas(_ values:[String],label:String,key:String)->some View {
    if !values.isEmpty {
      if !label.isEmpty {Text(label).font(.caption).foregroundStyle(.secondary)}
      ForEach(Array(values.enumerated()),id:\.offset) {index,latex in
        NativeMathView(request:.init(identity:identity+entry.step.id+"/"+key+"/\(index)",originalLatex:latex,wolfram:"",modern:nil,fontSize:15),showsControls:false)
      }
    }
  }
  // Short navigation labels correspond to the same actual rule IDs used by Web steps.zh-CN.
  // They do not replace the producer's formulas, parameters, conditions or step hierarchy.
  private static let titles=[
    "back_substitute":"回代原变量","binomial_formula":"求二项式方程的根","branch":"求解当前分支",
    "cardano_formula":"三次方程求根公式","clear_denominators":"消去分母","discriminant":"计算判别式",
    "domain_filter":"检查定义域","drop_extraneous":"舍去增根","eliminant":"构造单变量方程","expand":"展开表达式",
    "factor":"因式分解","ferrari_formula":"四次方程求根公式","generic_assumption":"记录参数条件",
    "groebner":"计算 Gröbner 基","invert_function":"反解函数","isolate_term":"移项","linear_formula":"求解一次方程",
    "normalize":"整理方程","note":"计算诊断","palindromic_formula":"倒数对称代换","quadratic_formula":"二次方程求根公式",
    "raise_to_power":"两边同取幂","record_exclusion":"保留限制条件","resultant":"结式消元","root_objects":"精确代数根",
    "row_reduce":"精确行变换","sign_chart":"区间符号分析","split_component":"求解因子分支","square_free":"无平方因子分解",
    "substitute":"变量代换","verify":"代入原方程验证","zero_product":"零乘积求解"
  ]
  private static let parameterLabels=[
    "factor":"因子","factors":"因子","parts":"分解","new_var":"新变量","def":"定义","var":"变量","value":"值",
    "results":"结果","cond":"条件","solution":"候选解","excluded":"已排除","digits":"位数","matrix":"矩阵",
    "basis":"基","branches":"分支","constants":"参数","verification":"验证","method":"方法","reason":"原因",
    "formula":"公式","func":"函数","result":"结果","points":"端点","signs":"符号","label":"分支","text":"消息"
  ]
}
