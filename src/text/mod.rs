pub mod file;

pub struct Source {
    pub text: Vec<u8>,
}

impl Source {
    pub fn new(t: Vec<u8>) -> Self {
        Self { text: t }
    }
}