pub mod path;
pub mod mem;
pub mod file;

pub struct Cache;

pub trait RequiredBytes {
    fn required_bytes(&self) -> u64;
}

impl RequiredBytes for Vec<u8> {
    fn required_bytes(&self) -> u64 {
        self.len() as u64
    }
}
