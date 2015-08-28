/*!
<a href="https://github.com/Nercury/bytecache-rs">
    <img style="position: absolute; top: 0; left: 0; border: 0;" src="https://s3.amazonaws.com/github/ribbons/forkme_left_orange_ff7600.png" alt="Fork me on GitHub">
</a>
<style>.sidebar { margin-top: 53px }</style>
*/

pub mod path;
pub mod mem;
pub mod file;
pub mod history;

use std::io::Read;
use std::io::Write;

#[derive(Debug, Eq, PartialEq)]
pub enum StoreResult {
    Stored,
    OutOfMemory,
}

#[derive(Debug)]
pub enum CreateReaderError {
    NotFound,
}

#[derive(Debug)]
pub enum CreateWriterError {
    OutOfMemory,
}

pub trait RequiredBytes {
    fn required_bytes(&self) -> u64;
}

impl RequiredBytes for Vec<u8> {
    fn required_bytes(&self) -> u64 {
        self.len() as u64
    }
}

pub trait Cache<K> {
    fn fetch<R: Read>(&self, key: K) -> Result<R, CreateReaderError>;
    fn store<W: Write>(&self, key: K, required_mem: u64) -> Result<W, CreateWriterError>;
}
