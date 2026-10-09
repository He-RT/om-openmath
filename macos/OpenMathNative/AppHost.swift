import AppKit
import OpenMathHost
import SwiftUI

@main
struct OpenMathNativeApp: App {
  var body: some Scene {
    Window("OpenMath Preview", id: "workspace") {
      LinkingStatusView()
        .frame(minWidth: 640, minHeight: 560)
    }
    .defaultSize(width: 1360, height: 860)
  }
}

private struct LinkingStatusView: View {
  private let abi = om_host_abi_version()
  private let metadata = om_host_metadata_version()
  private let kernelVersion = String(cString: om_host_kernel_release_version())
  private let targetVersion =
    Bundle.main.object(forInfoDictionaryKey: "OpenMathTargetVersion") as? String ?? "未知"

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 24) {
        Label("OpenMath", systemImage: "function")
          .font(.largeTitle)
        Text("原生 Mac 开发预览")
          .font(.title2)
        Text("完整文档宿主尚未接通；本窗口验证原生工程与 Rust 库链接。")
          .foregroundStyle(.secondary)
        Form {
          LabeledContent("桥接 ABI", value: String(abi))
          LabeledContent("实际内核元数据", value: metadata > 0 ? String(metadata) : "链接查询失败")
          LabeledContent("内核源码版本", value: kernelVersion)
          LabeledContent("本版交付目标", value: targetVersion)
        }
        Text("笔记本编辑、执行、存储与助手将按任务接入。现有 OpenMath .3 安装和数据保留。")
          .foregroundStyle(.secondary)
      }
      .padding(24)
      .frame(maxWidth: 800, alignment: .leading)
      .frame(maxWidth: .infinity, alignment: .leading)
    }
    .tint(.green)
  }
}
