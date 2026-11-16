#![no_std]

extern crate alloc;

mod bitmap;
mod block_cache;
mod block_dev;
mod efs;
mod layout;
mod vfs;

pub const BLOCK_SZ: usize = 512;
use bitmap::Bitmap;
use block_cache::{block_cache_sync_all, get_block_cache};
pub use block_dev::BlockDevice;
pub use efs::EasyFileSystem;
use layout::*;
pub use vfs::Inode;

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use alloc::{sync::Arc,vec,vec::Vec};
    use std::sync::Mutex;
    struct Disk(Mutex<Vec<[u8;BLOCK_SZ]>>);
    impl BlockDevice for Disk {
        fn handle_irq(&self) {}
        fn read_block(&self,id:usize,buf:&mut[u8]) {buf.copy_from_slice(&self.0.lock().unwrap()[id]);}
        fn write_block(&self,id:usize,buf:&[u8]) {self.0.lock().unwrap()[id].copy_from_slice(buf);}
    }
    #[test]
    fn exhausted_disk_rolls_back_without_out_of_range_blocks() {
        let disk=Arc::new(Disk(Mutex::new(vec![[0;BLOCK_SZ];2048])));
        let fs=EasyFileSystem::create(disk,2048,1);
        let root=EasyFileSystem::root_inode(&fs);
        let file=root.create("proof").unwrap();
        assert_eq!(file.write_at(0,b"preserved"),9);
        let too_large=vec![3u8;1024*1024];
        for _ in 0..3 {assert_eq!(file.write_at(0,&too_large),0);}
        let mut out=[0;9];assert_eq!(file.read_at(0,&mut out),9);assert_eq!(&out,b"preserved");
        // A failed growth must not leak the blocks it tentatively reserved.
        let fitting=vec![4u8;300*1024];assert_eq!(file.write_at(0,&fitting),fitting.len());
        file.clear();assert_eq!(file.write_at(0,&fitting),fitting.len());
        assert!(root.create("filename-that-exceeds-27-bytes").is_none());
    }
}
