import Foundation

struct CellPresentationState:Equatable,Sendable {
  enum Status:Equatable,Sendable {
    case pending,running,verified,exact,numeric,geometry,structured,conditional,stale(String),error,unevaluated,stopped,unconfirmed
  }
  struct Facts:Sendable {
    var running=false,stopped=false,hasResult=false,sourceChanged=false
    var staleReason:String?
    var error=false,conditional=false,verified=false,unevaluated=false,hasMetadata=false
    var nature=""
    var outputKind=""
    var structured=false
    var byAgent=false
  }
  let status:Status,byAgent:Bool
  static func derive(_ facts:Facts)->Self {
    let status:Status
    if facts.running {status = .running}
    else if facts.stopped {status = .stopped}
    else if !facts.hasResult {status = .pending}
    else if facts.sourceChanged {status = .stale(facts.staleReason ?? "源码已修改")}
    else if let reason=facts.staleReason {status = .stale(reason)}
    else if facts.error {status = .error}
    else if !facts.hasMetadata {status = .unconfirmed}
    else if ["plot","scene3d"].contains(facts.outputKind) {status = .geometry}
    else if facts.conditional {status = .conditional}
    else if facts.verified {status = .verified}
    else if facts.unevaluated {status = .unevaluated}
    else if facts.structured {status = .structured}
    else if ["machine","high_precision"].contains(facts.nature) {status = .numeric}
    else {status = .exact}
    return .init(status:status,byAgent:facts.byAgent)
  }
  var label:String {
    switch status {
    case .pending:return "尚未运行"
    case .running:return "正在计算…"
    case .verified:return "精确解，已代入验证"
    case .exact:return "精确值"
    case .numeric:return "数值近似"
    case .geometry:return "图形已生成"
    case .structured:return "结构化结果"
    case .conditional:return "有条件成立"
    case .stale(let reason):return "结果过期 · "+reason
    case .error:return "计算出错"
    case .unevaluated:return "内核没有化简这个表达式（原样保留）"
    case .stopped:return "已停止"
    case .unconfirmed:return "结果待核对"
    }
  }
  var dashed:Bool {switch status {case .running,.stale:return true;default:return false}}
  var muted:Bool {switch status {case .pending,.stale,.unevaluated,.stopped,.unconfirmed:return true;default:return false}}
}
