pub trait Progress: Send + Sync {
    fn inc(&self, delta: u64);
    fn set_position(&self, pos: u64);
    fn finish(&self) {}
    fn finish_with_message(&self, msg: &str) {
        let _ = msg;
    }
}
