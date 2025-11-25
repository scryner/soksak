import Foundation
import WhisperKit
import CoreML

public class WhisperContext {
    var pipe: WhisperKit?
    let modelPath: String?
    let modelName: String?
    let language: String?
    
    init(modelPath: String?, modelName: String?, language: String?) {
        self.modelPath = modelPath
        self.modelName = modelName
        self.language = language
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
        config.computeOptions = ModelComputeOptions(
            melCompute: MLComputeUnits.all,
            audioEncoderCompute: MLComputeUnits.cpuAndNeuralEngine,
            textDecoderCompute: MLComputeUnits.cpuAndNeuralEngine,
            prefillCompute: MLComputeUnits.all
        )
        
        let pipe = try await WhisperKit(config)
        self.pipe = pipe
        return pipe
    }
}

@_cdecl("whisperkit_create_context")
public func whisperkit_create_context(
    modelPath: UnsafePointer<CChar>?,
    modelName: UnsafePointer<CChar>?,
    lang: UnsafePointer<CChar>?
) -> UnsafeMutableRawPointer {
    var modelPathStr: String? = nil
    if let modelPath = modelPath {
        modelPathStr = String(cString: modelPath)
    }
    var modelNameStr: String? = nil
    if let modelName = modelName {
        modelNameStr = String(cString: modelName)
    }
    var langStr: String? = nil
    if let lang = lang {
        langStr = String(cString: lang)
    }
    
    let context = WhisperContext(modelPath: modelPathStr, modelName: modelNameStr, language: langStr)
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

            // Decoding Options
            var decodingOptions = DecodingOptions()
            if let lang = whisperContext.language {
                decodingOptions.language = lang
            }

            // Set concurrent worker count to 0 (unlimited)
            decodingOptions.concurrentWorkerCount = 0

            // Set chunking strategy to vad
            decodingOptions.chunkingStrategy = .vad

            // Set decoding options to suppress special tokens and timestamps
            decodingOptions.skipSpecialTokens = true
            decodingOptions.suppressBlank = true

            // Some options that are not used currently (for future use)
            // decodingOptions.withoutTimestamps = true
            // decodingOptions.wordTimestamps = true
            // decodingOptions.sampleLength = 180

            // Set temperature to 0.0
            decodingOptions.temperature = 0.0
            decodingOptions.temperatureFallbackCount = 0
            // decodingOptions.temperatureIncrementOnFallback = 0.1

            // Transcribe
            let results = try await pipe.transcribe(
                audioArray: audioSamples,
                decodeOptions: decodingOptions,
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
