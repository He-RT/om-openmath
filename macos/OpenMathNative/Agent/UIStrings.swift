import Foundation

enum UIStrings {
  static let values:[String:String]=[
    "cell.pending":"尚未运行","cell.running":"正在计算…","cell.verified":"精确解，已代入验证",
    "cell.exact":"精确值","cell.numeric":"数值近似","cell.conditional":"有条件成立",
    "cell.stale":"结果过期","cell.error":"计算出错","cell.stopped":"已停止",
    "doc.edited":"已编辑","doc.saved":"已保存","kernel.ready":"内核就绪",
    "agent.turn_ended":"回合结束","agent.stopped":"已停止，已完成的修改保留",
    "agent.failed":"请求失败：","agent.budget":"已达到本轮上限","agent.next_turn":"从下一轮开始使用"
  ]
  static func text(_ key:String)->String {values[key] ?? key}
  static func end(_ reason:TurnEndReason)->String {
    switch reason {
    case .modelFinished:return text("agent.turn_ended")
    case .stopped:return text("agent.stopped")
    case .failed(let reason):return text("agent.failed")+reason+"。已提交的修改不会自动重做"
    case .budget(let kind):
      let name=switch kind {case .steps:"步骤";case .time:"时间";case .context:"上下文"}
      return text("agent.budget")+"（"+name+"），已提交的修改保留"
    }
  }
}
