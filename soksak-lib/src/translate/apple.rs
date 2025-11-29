use crate::translate::{BatchItem, BatchTranslationResponse};
use anyhow::Result;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use tokio::sync::oneshot;

#[link(name = "apple_bridge", kind = "static")]
unsafe extern "C" {
    fn apple_translate(
        text: *const c_char,
        source_lang: *const c_char,
        target_lang: *const c_char,
        context: *mut c_void,
        callback: extern "C" fn(*mut c_void, *const c_char, *const c_char),
    );
}

extern "C" fn translate_callback(
    context: *mut c_void,
    result: *const c_char,
    error: *const c_char,
) {
    unsafe {
        let tx_ptr = context as *mut oneshot::Sender<Result<String, String>>;
        let tx = Box::from_raw(tx_ptr);

        if !error.is_null() {
            let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
            let _ = tx.send(Err(err_str));
        } else if !result.is_null() {
            let res_str = CStr::from_ptr(result).to_string_lossy().into_owned();
            let _ = tx.send(Ok(res_str));
        } else {
            let _ = tx.send(Err(
                "Unknown error: result and error are both null".to_string()
            ));
        }
    }
}

pub async fn translate(
    text: &str,
    source_lang: Option<&str>,
    target_lang: &str,
) -> Result<String, String> {
    let text_c = CString::new(text).map_err(|e| e.to_string())?;
    let target_lang_c = CString::new(target_lang).map_err(|e| e.to_string())?;

    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx_ptr = Box::into_raw(Box::new(tx));

    // Bind CString to a variable so it lives long enough for the FFI call
    let source_lang_c = match source_lang {
        Some(lang) => Some(CString::new(lang).map_err(|e| e.to_string())?),
        None => None,
    };

    unsafe {
        let source_lang_ptr = match &source_lang_c {
            Some(c_str) => c_str.as_ptr(),
            None => std::ptr::null(),
        };

        apple_translate(
            text_c.as_ptr(),
            source_lang_ptr,
            target_lang_c.as_ptr(),
            tx_ptr as *mut c_void,
            translate_callback,
        );
    }

    match rx.await {
        Ok(result) => result,
        Err(_) => Err("Translation task cancelled or panicked".to_string()),
    }
}

pub async fn translate_batch(
    batch_items: &[BatchItem],
    source_lang: Option<&str>,
    target_lang: &str,
) -> Result<Vec<BatchTranslationResponse>> {
    let mut results = Vec::new();

    for item in batch_items {
        if item.text.is_empty() {
            results.push(BatchTranslationResponse {
                id: item.id,
                translated_text: String::new(),
            });
            continue;
        }

        // Map String error to anyhow::Error
        let translated_text = translate(&item.text, source_lang, target_lang)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;

        results.push(BatchTranslationResponse {
            id: item.id,
            translated_text,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignored by default as it requires macOS 15+ and specific environment
    async fn test_apple_translate() {
        let text = "Hello, world!";
        let target = "ko";
        match translate(text, None, target).await {
            Ok(translated) => {
                println!("Translated: {}", translated);
                assert!(!translated.is_empty());
            }
            Err(e) => {
                eprintln!(
                    "Translation failed (expected if models not installed): {}",
                    e
                );
            }
        }
    }
}
