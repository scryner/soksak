import Foundation
import Translation
import NaturalLanguage

@available(macOS 15.0, *)
@_cdecl("apple_translate")
public func apple_translate(
    text: UnsafePointer<CChar>,
    source_lang: UnsafePointer<CChar>,
    target_lang: UnsafePointer<CChar>,
    context: UnsafeMutableRawPointer,
    callback: @convention(c) (UnsafeMutableRawPointer, UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> Void
) {
    let textStr = String(cString: text)
    let targetLangStr = String(cString: target_lang)
    let target = Locale.Language(identifier: targetLangStr)

    let source: Locale.Language
    
    if source_lang == nil {
        // Detect language
        let recognizer = NLLanguageRecognizer()
        recognizer.processString(textStr)
        guard let detectedLang = recognizer.dominantLanguage else {
            let errorStr = "Could not detect source language"
            errorStr.withCString { ptr in
                callback(context, nil, ptr)
            }
            return
        }
        source = Locale.Language(identifier: detectedLang.rawValue)

    } else {
        let sourceLangStr = String(cString: source_lang)
        source = Locale.Language(identifier: sourceLangStr)
    }

    Task {
        do {
            // Use installedSource:target:
            let session = TranslationSession(installedSource: source, target: target)
            try await session.prepareTranslation()
            
            let response = try await session.translate(textStr)
            
            response.targetText.withCString { ptr in
                callback(context, ptr, nil)
            }
        } catch {
            var errorStr = error.localizedDescription
            
            let errorDesc = String(describing: error)
            if errorDesc.contains("notInstalled") {
                errorStr = "Language model not installed. Please download it in System Settings > General > Language & Region > Translation Languages."
            }
            
            errorStr.withCString { ptr in
                callback(context, nil, ptr)
            }
        }
    }
}
