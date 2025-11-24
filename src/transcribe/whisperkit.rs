use crate::ffmpeg_decoder;
use crate::transcribe::TranscriptSegment;
use anyhow::{Result, anyhow};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::path::Path;
use std::sync::mpsc::{Sender, channel};

enum BridgeMessage {
    Segment(TranscriptSegment),
    Error(String),
    Progress(i32),
    Done,
}

extern "C" fn whisperkit_progress_callback(progress: f64, context: *mut c_void) {
    unsafe {
        let sender_ptr = context as *mut Sender<BridgeMessage>;
        let sender = &*sender_ptr;
        let _ = sender.send(BridgeMessage::Progress(progress as i32));
    }
}

extern "C" fn whisperkit_callback(
    text: *const c_char,
    error: *const c_char,
    start: f64,
    end: f64,
    context: *mut c_void,
) {
    unsafe {
        let sender_ptr = context as *mut Sender<BridgeMessage>;
        let sender = &*sender_ptr;

        if !error.is_null() {
            let err_str = CStr::from_ptr(error).to_string_lossy().into_owned();
            let _ = sender.send(BridgeMessage::Error(err_str));
        } else if !text.is_null() {
            let text_str = CStr::from_ptr(text).to_string_lossy().into_owned();
            let segment = TranscriptSegment {
                start: (start * 100.0) as i64, // s to cs
                end: (end * 100.0) as i64,     // s to cs
                text: text_str,
            };
            let _ = sender.send(BridgeMessage::Segment(segment));
        } else {
            // Done
            let _ = sender.send(BridgeMessage::Done);
        }
    }
}

#[link(name = "SoksakBridge", kind = "static")]
unsafe extern "C" {
    fn whisperkit_create_context(
        model_path: *const c_char,
        model_name: *const c_char,
    ) -> *mut c_void;

    fn whisperkit_release_context(context: *mut c_void);

    fn whisperkit_transcribe(
        context: *mut c_void,
        audio_path: *const c_char,
        callback: extern "C" fn(*const c_char, *const c_char, f64, f64, *mut c_void),
        progress_callback: extern "C" fn(f64, *mut c_void),
        callback_context: *mut c_void,
    );
}

pub struct WhisperKit {
    context: *mut c_void,
}

unsafe impl Send for WhisperKit {}
unsafe impl Sync for WhisperKit {}

impl Drop for WhisperKit {
    fn drop(&mut self) {
        unsafe {
            whisperkit_release_context(self.context);
        }
    }
}

impl WhisperKit {
    #[allow(dead_code)]
    pub fn new(model_path: &str) -> Self {
        let model_path_c = CString::new(model_path).unwrap();
        unsafe {
            let context = whisperkit_create_context(model_path_c.as_ptr(), std::ptr::null());
            Self { context }
        }
    }

    pub fn new_with_model_name(model_name: &str) -> Self {
        let model_name_c = CString::new(model_name).unwrap();
        unsafe {
            let context = whisperkit_create_context(std::ptr::null(), model_name_c.as_ptr());
            Self { context }
        }
    }

    pub fn transcribe<P: AsRef<Path>>(
        &self,
        audio: P,
        pb: &mut indicatif::ProgressBar,
    ) -> Result<Vec<TranscriptSegment>> {
        let audio = ffmpeg_decoder::file(audio)?;

        let audio_path = audio
            .path()
            .to_str()
            .ok_or_else(|| anyhow!("Invalid audio path"))?;

        let audio_c = CString::new(audio_path)?;

        let (tx, rx) = channel::<BridgeMessage>();
        let tx_ptr = Box::into_raw(Box::new(tx));

        unsafe {
            whisperkit_transcribe(
                self.context,
                audio_c.as_ptr(),
                whisperkit_callback,
                whisperkit_progress_callback,
                tx_ptr as *mut c_void,
            );
        }

        let mut segments = Vec::new();

        // Wait for messages
        for msg in rx {
            match msg {
                BridgeMessage::Segment(seg) => {
                    segments.push(seg);
                }
                BridgeMessage::Progress(p) => {
                    pb.set_position(p as u64);
                }
                BridgeMessage::Error(e) => {
                    // Reclaim the box
                    unsafe {
                        let _ = Box::from_raw(tx_ptr);
                    }
                    return Err(anyhow!(e));
                }
                BridgeMessage::Done => {
                    break;
                }
            }
        }

        // Reclaim the box
        unsafe {
            let _ = Box::from_raw(tx_ptr);
        }

        Ok(segments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_whisperkit_transcribe() {
        // Define paths relative to the project root
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let root = PathBuf::from(manifest_dir);
        // let model_path = root.join("src/transcribe/models/large-v3-turbo-coreml");
        let model_path = "openai_whisper-large-v3_turbo";
        let audio_path = root.join("src/transcribe/harvard.wav");

        // Ensure audio file exists
        if !audio_path.exists() {
            eprintln!("Skipping test: Audio file not found at {:?}", audio_path);
            return;
        }

        let mut pb = indicatif::ProgressBar::new(0);

        // Try with local model first
        let whisper = WhisperKit::new_with_model_name(model_path);
        let result = whisper.transcribe(&audio_path, &mut pb);

        match result {
            Ok(segments) => {
                println!(
                    "Transcription success with local model. Segments: {}",
                    segments.len()
                );
                assert!(!segments.is_empty(), "Should return at least one segment");
            }
            Err(e) => {
                println!("Local model failed (expected if model is invalid): {}", e);
                println!("Attempting to download and use 'openai_whisper-tiny'...");

                let whisper_tiny = WhisperKit::new_with_model_name("openai_whisper-tiny");
                let result_tiny = whisper_tiny.transcribe(&audio_path, &mut pb);

                match result_tiny {
                    Ok(segments) => {
                        println!(
                            "Transcription success with downloaded tiny model. Segments: {}",
                            segments.len()
                        );
                        for segment in &segments {
                            println!("[{} - {}] {}", segment.start, segment.end, segment.text);
                        }
                        assert!(!segments.is_empty(), "Should return at least one segment");

                        // Verify some expected text content if possible
                        let full_text = segments
                            .iter()
                            .map(|s| s.text.clone())
                            .collect::<Vec<_>>()
                            .join(" ");
                        assert!(
                            full_text.to_lowercase().contains("beer"),
                            "Transcription should contain expected text"
                        );
                    }
                    Err(e_tiny) => {
                        panic!("Transcription failed with tiny model as well: {}", e_tiny);
                    }
                }
            }
        }
    }
}
