import SwiftUI

struct SettingsView: View {
  var controller: NotebookController
  @Environment(\.dismiss) private var dismiss
  @State private var draft: JSONValue = .null
  @State private var tab = 0
  @State private var selected = 0
  @State private var saving = false
  @State private var invalidJSON = Set<String>()
  @State private var error: String?
  @State private var testID: String?
  private var profiles: [JSONValue] { draft["llm"]["profiles"].array }
  private var profile: JSONValue {
    profiles.indices.contains(selected) ? profiles[selected] : .null
  }
  var body: some View {
    NavigationStack {
      Form {
        Picker(controller.text("设置", "Settings"), selection: $tab) {
          Text(controller.text("常规", "General")).tag(0)
          Text(controller.text("AI 模型", "AI models")).tag(1)
          Text(controller.text("功能映射", "Feature mapping")).tag(2)
        }.pickerStyle(.segmented)
        if tab == 0 { general } else if tab == 1 { models } else { mappings }
        if let error { Section { Text(error).foregroundStyle(.red).textSelection(.enabled) } }
      }
      .disabled(saving)
      .navigationTitle(controller.text("偏好设置", "Preferences"))
      .toolbar {
        ToolbarItem(placement: .cancellationAction) {
          Button(controller.text("关闭", "Close")) { dismiss() }
        }
        ToolbarItem(placement: .confirmationAction) {
          Button(controller.text("保存", "Save")) {
            Task {
              saving = true
              defer { saving = false }
              do {
                try await controller.applySettings(draft)
                dismiss()
              } catch { self.error = error.localizedDescription }
            }
          }.disabled(saving || draft.isNull || !invalidJSON.isEmpty)
        }
      }
      .task {
        do { draft = try SettingsStore.load() ?? controller.config } catch {
          self.error = error.localizedDescription
          draft = controller.config
        }
      }
      .onDisappear { if let testID { controller.cancelAI(testID) } }
    }
  }
  private var general: some View {
    Group {
      Section(controller.text("外观与输入", "Appearance and input")) {
        Picker(
          controller.text("主题", "Theme"),
          selection: Binding(
            get: { controller.scheme },
            set: {
              controller.scheme = $0
              UserDefaults.standard.set($0, forKey: "OpenMathTheme")
            })
        ) {
          Text(controller.text("跟随系统", "System")).tag("system")
          Text(controller.text("浅色", "Light")).tag("light")
          Text(controller.text("深色", "Dark")).tag("dark")
        }
        Picker(controller.text("语言", "Language"), selection: string(["general", "language"])) {
          Text(controller.text("跟随系统", "System")).tag("auto")
          Text("简体中文").tag("zh-CN")
          Text("English").tag("en")
        }
        Picker(controller.text("方言", "Dialect"), selection: string(["general", "dialect"])) {
          Text("Modern").tag("modern")
          Text("Wolfram").tag("wolfram")
          Text("Auto").tag("auto")
        }
        Picker(controller.text("常量", "Constants"), selection: string(["general", "constants"])) {
          Text(controller.text("数学常量 e/i", "Mathematical e/i")).tag("math")
          Text(controller.text("e/i 作为变量", "e/i as symbols")).tag("strict")
        }
      }
      Section(controller.text("计算", "Evaluation")) {
        Toggle(controller.text("响应式", "Reactive"), isOn: boolean(["general", "reactive"]))
        Toggle(
          controller.text("重算依赖单元格", "Run dependents"),
          isOn: boolean(["general", "auto_run_dependents"]))
        Toggle(controller.text("记录步骤", "Record steps"), isOn: boolean(["general", "show_steps"]))
        Toggle(controller.text("自动绘图", "Automatic plots"), isOn: boolean(["general", "auto_plot"]))
        TextField(
          controller.text("超时（毫秒）", "Timeout (ms)"), value: number(["general", "eval_timeout_ms"]),
          format: .number
        ).keyboardType(.numberPad)
      }
    }
  }
  private var models: some View {
    Group {
      Section(controller.text("模型配置", "Profiles")) {
        Picker(controller.text("配置", "Profile"), selection: $selected) {
          ForEach(Array(profiles.enumerated()), id: \.offset) { index, p in
            Text(p["name"].string).tag(index)
          }
        }
        HStack {
          Button(controller.text("新增", "Add")) {
            var p = profiles.first ?? .null
            p["name"] = .string("profile-\(profiles.count+1)")
            p["api_key"] = .null
            p["extra_headers"] = .object([:])
            var values = profiles
            values.append(p)
            draft["llm"]["profiles"] = .array(values)
            selected = values.count - 1
          }
          Button(controller.text("复制", "Duplicate")) {
            var p = profile
            p["name"] = .string(profile["name"].string + "-copy")
            p["api_key"] = .null
            var values = profiles
            values.append(p)
            draft["llm"]["profiles"] = .array(values)
            selected = values.count - 1
          }
          Button(controller.text("删除", "Delete"), role: .destructive) {
            var values = profiles
            guard values.indices.contains(selected) else { return }
            let name = values.remove(at: selected)["name"].string
            draft["llm"]["profiles"] = .array(values)
            for feature in ["translate", "explain", "complete", "chat", "fix"]
            where draft["llm"][feature].string == name { draft["llm"][feature] = .string("") }
            selected = max(0, min(selected, values.count - 1))
          }
        }
      }
      if !profile.isNull {
        Section(controller.text("提供商", "Provider")) {
          TextField(controller.text("名称", "Name"), text: profileString("name"))
          Picker(controller.text("类型", "Type"), selection: profileString("kind")) {
            ForEach(
              ["openai_chat", "anthropic", "openai_fim", "ollama_fim", "mistral_fim"], id: \.self
            ) { Text($0).tag($0) }
          }
          TextField("Base URL", text: profileString("base_url")).textInputAutocapitalization(.never)
            .autocorrectionDisabled().keyboardType(.URL)
          TextField(controller.text("模型", "Model"), text: profileString("model"))
            .textInputAutocapitalization(.never).autocorrectionDisabled()
          SecureField(controller.text("API 密钥", "API key"), text: profileString("api_key"))
            .textInputAutocapitalization(.never).autocorrectionDisabled()
          Button(controller.text("清除密钥", "Clear key"), role: .destructive) {
            updateProfile("api_key", .null)
          }
          Toggle(
            controller.text("需要 API 密钥", "API key required"),
            isOn: profileBool("requires_api_key", fallback: true))
          Text(
            controller.text(
              "密钥及高级值保存在系统凭据库，不写入笔记本。",
              "Keys and advanced values are kept in the system credential store, outside notebooks."
            )
          ).font(.caption).foregroundStyle(.secondary)
        }
        Section(controller.text("能力与参数", "Capabilities and parameters")) {
          Toggle(controller.text("支持工具调用", "Tool calls"), isOn: profileBool("supports_tools"))
          Toggle(
            controller.text("支持 JSON 模式", "JSON mode"), isOn: profileBool("supports_json_mode"))
          TextField(
            controller.text("温度", "Temperature"), value: profileNumber("temperature"),
            format: .number
          ).keyboardType(.decimalPad)
          TextField(
            controller.text("最大 token", "Max tokens"), value: profileNumber("max_tokens"),
            format: .number
          ).keyboardType(.numberPad)
          TextField(
            controller.text("HTTP 超时（毫秒）", "HTTP timeout (ms)"), value: profileNumber("timeout_ms"),
            format: .number
          ).keyboardType(.numberPad)
          JSONEditor(
            title: controller.text("额外请求头（JSON）", "Extra headers (JSON)"),
            value: Binding(
              get: { profile["extra_headers"] }, set: { updateProfile("extra_headers", $0) }),
            validity: { valid in
              if valid { invalidJSON.remove("headers") } else { invalidJSON.insert("headers") }
            })
          JSONEditor(
            title: controller.text("额外请求参数（JSON）", "Extra parameters (JSON)"),
            value: Binding(
              get: { profile["extra_body"].isNull ? .object([:]) : profile["extra_body"] },
              set: { updateProfile("extra_body", $0) }),
            validity: { valid in
              if valid { invalidJSON.remove("body") } else { invalidJSON.insert("body") }
            })
        }
        Section {
          Button(controller.text("测试连接", "Test connection")) {
            Task {
              if let testID { controller.cancelAI(testID) }
              testID = await controller.beginAI(
                feature: "test_profile",
                body: .object([
                  "type": .string("llm_test_profile"), "profile": profile["name"],
                  "config": profile,
                ]))
            }
          }
          Text(
            controller.text(
              "只向此草稿地址发送短 ping，不保存设置、不发送笔记本。",
              "Sends a short ping to this draft URL, without saving settings or sending notebook data."
            )
          ).font(.caption).foregroundStyle(.secondary)
          if let testID {
            if controller.aiStates[testID] == "pending" {
              HStack {
                ProgressView()
                Button(controller.text("取消", "Cancel")) { controller.cancelAI(testID) }
              }
            }
            Text(controller.aiText[testID] ?? "").textSelection(.enabled)
            if let notice = controller.notice { Text(notice).font(.caption) }
          }
        }
      }
    }
  }
  private var mappings: some View {
    Section(controller.text("AI 功能", "AI features")) {
      Toggle(controller.text("启用 AI", "Enable AI"), isOn: boolean(["llm", "enabled"]))
      Toggle(
        controller.text("补全可读取笔记本上下文", "Notebook context for completion"),
        isOn: boolean(["llm", "send_context"]))
      ForEach(["translate", "explain", "complete", "chat", "fix"], id: \.self) { feature in
        Picker(feature, selection: string(["llm", feature])) {
          Text(controller.text("关闭", "Disabled")).tag("")
          ForEach(Array(profiles.enumerated()), id: \.offset) { _, profile in
            Text(profile["name"].string).tag(profile["name"].string)
          }
        }
      }
    }
  }
  private func get(_ path: [String]) -> JSONValue { path.reduce(draft) { $0[$1] } }
  private func set(_ path: [String], _ value: JSONValue) {
    func write(_ current: JSONValue, _ path: ArraySlice<String>) -> JSONValue {
      guard let key = path.first else { return value }
      var result = current
      result[key] = write(current[key], path.dropFirst())
      return result
    }
    draft = write(draft, path[...])
  }
  private func string(_ path: [String]) -> Binding<String> {
    Binding(get: { get(path).string }, set: { set(path, .string($0)) })
  }
  private func boolean(_ path: [String]) -> Binding<Bool> {
    Binding(get: { get(path).bool }, set: { set(path, .bool($0)) })
  }
  private func number(_ path: [String]) -> Binding<Double> {
    Binding(get: { get(path).double }, set: { set(path, .number($0)) })
  }
  private func updateProfile(_ key: String, _ value: JSONValue) {
    var values = profiles
    guard values.indices.contains(selected) else { return }
    if key == "name", values[selected][key] != value {
      let oldName = values[selected][key]
      for feature in ["translate", "explain", "complete", "chat", "fix"]
      where draft["llm"][feature] == oldName { draft["llm"][feature] = value }
    }
    values[selected][key] = value
    draft["llm"]["profiles"] = .array(values)
  }
  private func profileString(_ key: String) -> Binding<String> {
    Binding(get: { profile[key].string }, set: { updateProfile(key, .string($0)) })
  }
  private func profileBool(_ key: String, fallback: Bool = false) -> Binding<Bool> {
    Binding(
      get: { profile[key].isNull ? fallback : profile[key].bool },
      set: { updateProfile(key, .bool($0)) })
  }
  private func profileNumber(_ key: String) -> Binding<Double> {
    Binding(get: { profile[key].double }, set: { updateProfile(key, .number($0)) })
  }
}
struct JSONEditor: View {
  var title: String
  @Binding var value: JSONValue
  var validity: (Bool) -> Void = { _ in }
  @State private var text = ""
  @State private var error = false
  var body: some View {
    VStack(alignment: .leading) {
      Text(title).font(.caption)
      TextEditor(text: $text).font(.system(.caption, design: .monospaced)).frame(minHeight: 70)
        .onChange(of: text) { _, new in
          if let data = new.data(using: .utf8),
            let parsed = try? JSONDecoder().decode(JSONValue.self, from: data),
            case .object = parsed
          {
            value = parsed
            error = false
            validity(true)
          } else {
            error = true
            validity(false)
          }
        }
      if error { Text("JSON 对象无效 / Invalid JSON object").font(.caption).foregroundStyle(.red) }
    }.onAppear { text = value.pretty }
  }
}
