use std::process::Child;

#[derive(Debug)]
pub struct SafeChild {
    pub process: Child,
}

impl SafeChild {
    pub fn id(&self) -> u32 {
        self.process.id()
    }
}

/// By implementing Drop, we ensure there are no zombie background Devnet processes
/// in case of an early test failure
impl Drop for SafeChild {
    fn drop(&mut self) {
        self.process.kill().expect("Cannot kill process");
        self.process.wait().expect("Should be dead");
    }
}
