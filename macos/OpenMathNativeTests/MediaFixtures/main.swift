import AVFoundation
import Foundation
import ImageIO
import PDFKit
@main struct MediaFixtures {
  static func main() async throws {
    let root=URL(fileURLWithPath:CommandLine.arguments[1],isDirectory:true)
    for name in ["rgb-pattern.png","math-bitmap.png"] {
      let source=CGImageSourceCreateWithURL(root.appendingPathComponent(name) as CFURL,nil)!
      let image=CGImageSourceCreateImageAtIndex(source,0,nil)!
      precondition(image.width==128 && image.height == (name=="rgb-pattern.png" ? 96 : 48))
      precondition(image.colorSpace != nil)
    }
    let pdf=PDFDocument(url:root.appendingPathComponent("math-text.pdf"))!
    precondition(pdf.pageCount==1 && pdf.string!.contains("x^2+y^2=1") && pdf.string!.contains("2+2=4"))
    let audio=try AVAudioFile(forReading:root.appendingPathComponent("tone.wav"))
    precondition(audio.length==16000 && audio.fileFormat.sampleRate==16000 && audio.fileFormat.channelCount==1)
    let buffer=AVAudioPCMBuffer(pcmFormat:audio.processingFormat,frameCapacity:16000)!
    try audio.read(into:buffer)
    precondition(buffer.frameLength==16000 && buffer.floatChannelData != nil)
    let values=buffer.floatChannelData![0]
    let power=(0..<16000).reduce(0.0){$0+Double(values[$1]*values[$1])}/16000
    precondition(power>0.01 && power<0.1)
    let asset=AVURLAsset(url:root.appendingPathComponent("marker.mp4"))
    let duration=try await asset.load(.duration)
    precondition(abs(duration.seconds-1)<0.1)
    let tracks=try await asset.loadTracks(withMediaType:.video)
    precondition(tracks.count==1)
    let size=try await tracks[0].load(.naturalSize)
    precondition(size.width==128 && size.height==96)
    let audioTracks=try await asset.loadTracks(withMediaType:.audio)
    precondition(audioTracks.isEmpty)
    let generator=AVAssetImageGenerator(asset:asset)
    generator.requestedTimeToleranceBefore = .zero;generator.requestedTimeToleranceAfter = .zero
    for frame in 0..<10 {
      let decoded=try await generator.image(at:CMTime(value:Int64(frame),timescale:10))
      precondition(decoded.image.width==128 && decoded.image.height==96)
      precondition(abs(decoded.actualTime.seconds-Double(frame)/10)<0.05)
      var pixels=[UInt8](repeating:0,count:128*96*4)
      let context=CGContext(data:&pixels,width:128,height:96,bitsPerComponent:8,bytesPerRow:128*4,space:CGColorSpaceCreateDeviceRGB(),bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
      context.draw(decoded.image,in:CGRect(x:0,y:0,width:128,height:96))
      let center=(48*128+frame*10+6)*4
      precondition(pixels[center]>230 && pixels[center+1]>230 && pixels[center+2]>230)
    }
    print("Actual ImageIO/PDFKit/AVFoundation: own PNG dimensions/color, PDF text, PCM audio power, all 10 H264 frame times and moving-marker pixels passed")
  }
}
