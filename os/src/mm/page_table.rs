use super::{FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum, frame_alloc};
use crate::config::PAGE_SIZE;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use bitflags::*;

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Count missing intermediate tables before any mutation (Sv39, 4 KiB leaves).
    pub fn missing_tables(&self, range: super::VPNRange) -> usize {
        let mut keys = Vec::new();
        for vpn in range {
            let ids = vpn.indexes();
            let first = self.root_ppn.get_pte_array()[ids[0]];
            if !first.is_valid() {
                if !keys.contains(&ids[0]) { keys.push(ids[0]); }
                let key = 512 + ids[0] * 512 + ids[1];
                if !keys.contains(&key) { keys.push(key); }
            } else {
                let second = first.ppn().get_pte_array()[ids[1]];
                let key = 512 + ids[0] * 512 + ids[1];
                if !second.is_valid() && !keys.contains(&key) { keys.push(key); }
            }
        }
        keys.len()
    }

    pub fn reserve_frame_slots(&mut self, count: usize) -> Result<(), ()> {
        self.frames.try_reserve(count).map_err(|_| ())
    }

    /// All intermediate physical frames have been reserved by the caller.
    pub fn map_reserved(&mut self, vpn: VirtPageNum, leaf: PhysPageNum, flags: PTEFlags,
                        reserved: &mut Vec<FrameTracker>) {
        let ids = vpn.indexes();
        let mut ppn = self.root_ppn;
        for (level, id) in ids.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*id];
            if level == 2 {
                *pte = PageTableEntry::new(leaf, flags | PTEFlags::V);
                break;
            }
            if !pte.is_valid() {
                let frame = reserved.pop().expect("reserved Sv39 table count");
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
    }

    pub fn prune_empty_tables(&mut self, vpn: VirtPageNum) {
        let ids = vpn.indexes();
        let root = &mut self.root_ppn.get_pte_array()[ids[0]];
        if !root.is_valid() { return; }
        let middle_ppn = root.ppn();
        let middle = &mut middle_ppn.get_pte_array()[ids[1]];
        if middle.is_valid() {
            let leaf_ppn = middle.ppn();
            if leaf_ppn.get_pte_array().iter().all(|p| !p.is_valid()) {
                *middle = PageTableEntry::empty();
                self.frames.retain(|f| f.ppn != leaf_ppn);
            }
        }
        if middle_ppn.get_pte_array().iter().all(|p| !p.is_valid()) {
            *root = PageTableEntry::empty();
            self.frames.retain(|f| f.ppn != middle_ppn);
        }
    }

    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

/// Load a string from other address spaces into kernel space without an end `\0`.
pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut bytes = Vec::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        }
        bytes.push(ch);
        va += 1;
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

pub fn translated_ref<T>(token: usize, ptr: *const T) -> &'static T {
    let page_table = PageTable::from_token(token);
    page_table
        .translate_va(VirtAddr::from(ptr as usize))
        .unwrap()
        .get_ref()
}

pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}

/// Copy from a user buffer while validating every touched page and permission.
/// This is used by Agent ABI records, which may legally straddle page edges.
pub fn copy_from_user(token: usize, ptr: *const u8, dst: &mut [u8]) -> Result<(), ()> {
    validate_user_buffer(token, ptr as usize, dst.len(), false)?;
    copy_from_user_bytes(token, ptr as usize, dst)
}

/// Copy to a user buffer while validating every touched page and permission.
pub fn copy_to_user(token: usize, ptr: *mut u8, src: &[u8]) -> Result<(), ()> {
    validate_user_buffer(token, ptr as usize, src.len(), true)?;
    copy_to_user_bytes(token, ptr as usize, src)
}

/// Reject masked/noncanonical aliases and validate the entire buffer before copying.
pub fn validate_user_buffer(token: usize, start: usize, len: usize, write: bool) -> Result<(), ()> {
    if len == 0 { return Ok(()); }
    let end = start.checked_add(len - 1).ok_or(())?;
    if start == 0 || end >= (1usize << 38) { return Err(()); }
    let table = PageTable::from_token(token);
    for page in (start / PAGE_SIZE)..=(end / PAGE_SIZE) {
        let pte = table.translate(VirtPageNum::from(page)).ok_or(())?;
        if !pte.is_valid() || !pte.flags().contains(PTEFlags::U)
            || (write && !pte.writable()) || (!write && !pte.readable()) { return Err(()); }
    }
    Ok(())
}

fn copy_from_user_bytes(token: usize, mut va: usize, bytes: &mut [u8]) -> Result<(), ()> {
    if bytes.is_empty() { return Ok(()); }
    va.checked_add(bytes.len() - 1).ok_or(())?;
    let page_table = PageTable::from_token(token);
    let mut copied = 0;
    while copied < bytes.len() {
        let addr = VirtAddr::from(va);
        let pte = page_table.translate(addr.floor()).ok_or(())?;
        let flags = pte.flags();
        if !pte.is_valid() || !flags.contains(PTEFlags::U) || !pte.readable() {
            return Err(());
        }
        let count = (PAGE_SIZE - addr.page_offset()).min(bytes.len() - copied);
        let page = &mut pte.ppn().get_bytes_array()[addr.page_offset()..addr.page_offset() + count];
        bytes[copied..copied + count].copy_from_slice(page);
        copied += count;
        va += count;
    }
    Ok(())
}

fn copy_to_user_bytes(token: usize, mut va: usize, bytes: &[u8]) -> Result<(), ()> {
    if bytes.is_empty() { return Ok(()); }
    va.checked_add(bytes.len() - 1).ok_or(())?;
    let page_table = PageTable::from_token(token);
    let mut copied = 0;
    while copied < bytes.len() {
        let addr = VirtAddr::from(va);
        let pte = page_table.translate(addr.floor()).ok_or(())?;
        let flags = pte.flags();
        if !pte.is_valid() || !flags.contains(PTEFlags::U) || !pte.writable() {
            return Err(());
        }
        let count = (PAGE_SIZE - addr.page_offset()).min(bytes.len() - copied);
        let page = &mut pte.ppn().get_bytes_array()[addr.page_offset()..addr.page_offset() + count];
        page.copy_from_slice(&bytes[copied..copied + count]);
        copied += count;
        va += count;
    }
    Ok(())
}

pub struct UserBuffer {
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}

pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
