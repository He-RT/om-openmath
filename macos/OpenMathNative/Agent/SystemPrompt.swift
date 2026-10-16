import Foundation

enum AssistantMode:Sendable {case execute,discuss}

/// Renders the system prompt generated from agent/prompts (scripts/build_prompts.py). The wording is
/// never edited here, so the Mac and Web assistants stay on one source.
enum SystemPrompt {
  static func render(mode:AssistantMode,version:String)->String {
    SystemPromptText.template
      .replacingOccurrences(of:"{{version}}",with:version)
      .replacingOccurrences(of:"{{mode_line}}",with:mode == .execute ? SystemPromptText.modeExecute:SystemPromptText.modeDiscuss)
  }
  static var releaseVersion:String {(Bundle.main.object(forInfoDictionaryKey:"OpenMathReleaseVersion") as? String) ?? "开发版"}
}
