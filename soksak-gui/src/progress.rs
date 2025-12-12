use rust_i18n::t;
use soksak_lib::progress::Progress;
use std::sync::mpsc::Sender;

pub enum ProgressEvent {
    Inc(u64),
    SetPosition(u64),
    SetMessage(String),
    Finish,
    FinishWithMessage(String),
    SetLength(u64),
}

#[derive(Clone)]
pub struct GuiProgress {
    tx: Sender<ProgressEvent>,
}

impl GuiProgress {
    pub fn new(tx: Sender<ProgressEvent>) -> Self {
        Self { tx }
    }
}

impl Progress for GuiProgress {
    fn inc(&self, delta: u64) {
        let _ = self.tx.send(ProgressEvent::Inc(delta));
    }

    fn set_position(&self, pos: u64) {
        let _ = self.tx.send(ProgressEvent::SetPosition(pos));
    }

    fn finish(&self) {
        let _ = self.tx.send(ProgressEvent::Finish);
    }

    fn set_message(&self, msg: &str) {
        match msg {
            "Extracting audio..." => {
                let _ = self.tx.send(ProgressEvent::SetMessage(
                    (t!("progress.extracting_audio")).to_string(),
                ));
            }
            "Transcribing..." => {
                let _ = self.tx.send(ProgressEvent::SetMessage(
                    (t!("progress.transcribing")).to_string(),
                ));
            }
            "Translating..." => {
                let _ = self.tx.send(ProgressEvent::SetMessage(
                    (t!("progress.translating")).to_string(),
                ));
            }
            _ => (),
        }
    }

    fn finish_with_message(&self, _msg: &str) {
        // let _ = self
        //     .tx
        //     .send(ProgressEvent::FinishWithMessage(msg.to_string()));
    }

    fn set_length(&self, len: u64) {
        let _ = self.tx.send(ProgressEvent::SetLength(len));
    }
}
