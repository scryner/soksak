import Foundation
import WhisperKit

@_cdecl("whisperkit_transcribe")
public func whisperkit_transcribe(
    audioPath: UnsafePointer<CChar>,
    modelPath: UnsafePointer<CChar>?,
    modelName: UnsafePointer<CChar>?,
    context: UnsafeMutableRawPointer,
    callback: @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, Double, Double, UnsafeMutableRawPointer) -> Void,
    progressCallback: @convention(c) (Double, UnsafeMutableRawPointer) -> Void
) {
    let audioPathStr = String(cString: audioPath)
    var modelFolder: String? = nil
    if let modelPath = modelPath {
        modelFolder = String(cString: modelPath)
    }
    var modelNameStr: String? = nil
    if let modelName = modelName {
        modelNameStr = String(cString: modelName)
    }
    
    Task {
        do {
            // Initialize WhisperKit
            let pipe: WhisperKit
            if let folder = modelFolder {
                 pipe = try await WhisperKit(WhisperKitConfig(modelFolder: folder))
            } else {
                //  pipe = try await WhisperKit(model: modelNameStr, download: true)
                pipe = try await WhisperKit(WhisperKitConfig(model: modelNameStr, download: true))
            }

            // Load audio
            // WhisperKit expects [Float] samples or an AVAudioFile.
            // For simplicity, let's assume we can load the file using standard AVFoundation or similar,
            // but WhisperKit has helpers.
            // Actually, WhisperKit's `transcribe(audioPath:)` is convenient if available,
            // but checking the docs/examples, it often takes an array of floats or an AVAudioFile.
            
            // Let's use a simple approach: load audio using a helper if available, or just pass the path if supported.
            // Looking at WhisperKit API, `transcribe(audioPath:)` might be available in higher level abstractions,
            // but `pipe.transcribe(audioFile:)` is common.
            
            // let url = URL(fileURLWithPath: audioPathStr)
            
            // Transcribe
            // Load audio
            let audioBuffer = try AudioProcessor.loadAudio(fromPath: audioPathStr)
            let duration = Double(audioBuffer.frameLength) / audioBuffer.format.sampleRate
            let audioSamples = Array(UnsafeBufferPointer(start: audioBuffer.floatChannelData![0], count: Int(audioBuffer.frameLength)))

            // Transcribe
            let results = try await pipe.transcribe(
                audioArray: audioSamples,
                callback: nil,
                segmentCallback: { segments in
                    if let lastSegment = segments.last {
                        let current = Double(lastSegment.end)
                        let percent = (current / duration) * 100.0
                        progressCallback(percent, context)
                    }
                }
            )
            
            for result in results {
                for segment in result.segments {
                    let text = segment.text
                    let start = Double(segment.start)
                    let end = Double(segment.end)
                    
                    text.withCString { textPtr in
                        callback(textPtr, nil, start, end, context)
                    }
                }
            }
            
            // Signal completion with nulls
            callback(nil, nil, 0, 0, context)
            
        } catch {
            let errorMsg = error.localizedDescription
            errorMsg.withCString { errorPtr in
                callback(nil, errorPtr, 0, 0, context)
            }
        }
    }
}
