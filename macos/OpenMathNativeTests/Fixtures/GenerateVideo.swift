import AVFoundation
import CoreVideo
import Foundation
@main struct GenerateVideo {
  static func main() async throws {
    guard CommandLine.arguments.count==2 else { fatalError("output path required") }
    let url=URL(fileURLWithPath:CommandLine.arguments[1])
    if FileManager.default.fileExists(atPath:url.path) { try FileManager.default.removeItem(at:url) }
    let writer=try AVAssetWriter(outputURL:url,fileType:.mp4)
    let input=AVAssetWriterInput(mediaType:.video,outputSettings:[AVVideoCodecKey:AVVideoCodecType.h264,AVVideoWidthKey:128,AVVideoHeightKey:96])
    let receiver=writer.inputPixelBufferReceiver(for:input,pixelBufferAttributes:nil)
    try writer.start()
    writer.startSession(atSourceTime:.zero)
    for frame in 0..<10 {
      let mutable=try CVMutablePixelBuffer(CVPixelBufferCreationAttributes(pixelFormatType:.init(rawValue:kCVPixelFormatType_32BGRA),size:.init(width:128,height:96)))
      mutable.withUnsafeBuffer { pixel in
        CVPixelBufferLockBaseAddress(pixel,[])
        let rowBytes=CVPixelBufferGetBytesPerRow(pixel)
        let data=CVPixelBufferGetBaseAddress(pixel)!.assumingMemoryBound(to:UInt8.self)
        for y in 0..<96 { for x in 0..<128 {
          let offset=y*rowBytes+x*4
          let marker=x>=frame*10 && x<frame*10+12 && y>=36 && y<60
          data[offset]=marker ? 255 : 40;data[offset+1]=marker ? 255 : 150
          data[offset+2]=marker ? 255 : 80;data[offset+3]=255
        } }
        CVPixelBufferUnlockBaseAddress(pixel,[])
      }
      try await receiver.append(CVReadOnlyPixelBuffer(consume mutable),with:CMTime(value:Int64(frame),timescale:10))

    }
    receiver.finish();await writer.finishWriting()
    guard writer.status == .completed else { throw writer.error! }
    print("Own H264 fixture: 128x96, 10 frames/10 fps, moving marker, no audio")
  }
}
