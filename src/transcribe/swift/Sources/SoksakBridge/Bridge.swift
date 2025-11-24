import Foundation
import WhisperKit
import CoreML

public class WhisperContext {
    var pipe: WhisperKit?
    let modelPath: String?
    let modelName: String?
    
    init(modelPath: String?, modelName: String?) {
        self.modelPath = modelPath
        self.modelName = modelName
    }
    
    func getPipe() async throws -> WhisperKit {
        if let pipe = pipe {
            return pipe
        }
        
        var config: WhisperKitConfig
        if let folder = modelPath {
             config = WhisperKitConfig(modelFolder: folder)
        } else {
            config = WhisperKitConfig(model: modelName, load: true, download: true)
        }
        
        // Use all available compute units
        let compute = MLComputeUnits.all
        config.computeOptions = ModelComputeOptions(
            melCompute: compute,
            audioEncoderCompute: compute,
            textDecoderCompute: compute,
            prefillCompute: compute
        )
        
        let pipe = try await WhisperKit(config)
        self.pipe = pipe
        return pipe
    }
}

@_cdecl("whisperkit_create_context")
public func whisperkit_create_context(
    modelPath: UnsafePointer<CChar>?,
    modelName: UnsafePointer<CChar>?
) -> UnsafeMutableRawPointer {
    var modelPathStr: String? = nil
    if let modelPath = modelPath {
        modelPathStr = String(cString: modelPath)
    }
    var modelNameStr: String? = nil
    if let modelName = modelName {
        modelNameStr = String(cString: modelName)
    }
    
    let context = WhisperContext(modelPath: modelPathStr, modelName: modelNameStr)
    return Unmanaged.passRetained(context).toOpaque()
}

@_cdecl("whisperkit_release_context")
public func whisperkit_release_context(context: UnsafeMutableRawPointer) {
    Unmanaged<WhisperContext>.fromOpaque(context).release()
}

@_cdecl("whisperkit_transcribe")
public func whisperkit_transcribe(
    context: UnsafeMutableRawPointer,
    audioPath: UnsafePointer<CChar>,
    callback: @convention(c) (UnsafePointer<CChar>?, UnsafePointer<CChar>?, Double, Double, UnsafeMutableRawPointer) -> Void,
    progressCallback: @convention(c) (Double, UnsafeMutableRawPointer) -> Void,
    callbackContext: UnsafeMutableRawPointer
) {
    let whisperContext = Unmanaged<WhisperContext>.fromOpaque(context).takeUnretainedValue()
    let audioPathStr = String(cString: audioPath)
    
    Task {
        do {
            let pipe = try await whisperContext.getPipe()
            
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
                        progressCallback(percent, callbackContext)
                    }
                }
            )
            
            for result in results {
                for segment in result.segments {
                    let text = segment.text
                    let start = Double(segment.start)
                    let end = Double(segment.end)
                    
                    text.withCString { textPtr in
                        callback(textPtr, nil, start, end, callbackContext)
                    }
                }
            }
            
            // Signal completion with nulls
            callback(nil, nil, 0, 0, callbackContext)
            
        } catch {
            let errorMsg = error.localizedDescription
            errorMsg.withCString { errorPtr in
                callback(nil, errorPtr, 0, 0, callbackContext)
            }
        }
    }
}
