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

pub enum StoreResult {
    Stored,
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

pub trait ByteCache<K> {

    fn fetch<R>(&self, key: K) -> R
        where R: Read;

    fn store<R>(&self, key: K, value: R) -> StoreResult
        where R: Read;

}
