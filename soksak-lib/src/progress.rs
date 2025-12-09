pub trait Progress: Send + Sync {
    fn inc(&self, delta: u64);
    fn set_position(&self, pos: u64);
    fn finish(&self) {}
    fn set_message(&self, msg: &str);
    fn finish_with_message(&self, msg: &str) {
        let _ = msg;
    }
    // New method for determinite progress with total count
    fn set_length(&self, len: u64) {
        let _ = len;
    }
}
