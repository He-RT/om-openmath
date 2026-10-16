import Foundation

@main struct PromptFixtures {
  static func main() throws {
    let version="0.1.0-pre-alpha.5"
    let execute=SystemPrompt.render(mode:.execute,version:version),discuss=SystemPrompt.render(mode:.discuss,version:version)
    for text in [execute,discuss] {
      precondition(!text.contains("{{"),"unresolved placeholder")
      precondition(text.contains("OpenMath "+version))
      for part in ["笔记本操作说明","OpenMath 现代语法要点","已验证的例子","任务规则","INVALID_REFERENCE","inspect_result"] {precondition(text.contains(part),"missing section "+part)}
      precondition(text.count<=9600,"prompt exceeds the 6k token budget")
    }
    precondition(execute.contains(SystemPromptText.modeExecute) && !execute.contains(SystemPromptText.modeDiscuss))
    precondition(discuss.contains(SystemPromptText.modeDiscuss) && !discuss.contains(SystemPromptText.modeExecute))
    precondition(SystemPromptText.sha256.count==64 && SystemPromptText.sha256.allSatisfy{$0.isHexDigit})
    // The operation guide is read from the frozen design document; the generated text must still contain it verbatim.
    let doc=try String(contentsOfFile:FileManager.default.currentDirectoryPath+"/docs/design/prompts/notebook-operation.md",encoding:.utf8)
    guard let start=doc.range(of:"## 模型操作说明\n") else {preconditionFailure("operation heading missing")}
    let rest=doc[start.upperBound...]
    let guide=(rest.range(of:"\n## ").map{String(rest[..<$0.lowerBound])} ?? String(rest)).trimmingCharacters(in:.whitespacesAndNewlines)
    precondition(guide.count>500 && execute.contains(guide),"operation guide drifted from docs/design/prompts/notebook-operation.md; run scripts/build_prompts.py")
    // The old unconditional completion wording must not come back.
    precondition(!execute.contains("已完成所有") && !discuss.contains("任务已结束，请查看回复和工具记录"))
    print("prompt fixtures passed: \(execute.count) chars, sha256 \(SystemPromptText.sha256.prefix(12))")
  }
}
