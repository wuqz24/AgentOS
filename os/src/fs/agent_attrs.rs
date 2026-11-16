//! Bounded flat-file attributes, inverted AND index, versioned summary cache,
//! and checksummed two-bank checkpoint/journal persistence on easy-fs.
use super::inode::ROOT_INODE;
use crate::sync::UPIntrFreeCell;
use agent_abi::{EINVAL, ENOSPC, EPERM, ESRCH};
use alloc::{collections::{BTreeMap, BTreeSet}, string::{String, ToString}, vec, vec::Vec};
use lazy_static::lazy_static;
use serde_json::{Value, json};

const BANKS: [&str; 2] = [".agent-meta-a", ".agent-meta-b"];
const MAX_FILES: usize = 1024;
const MAX_ATTRS: usize = 8;
const BANK_LIMIT: usize = 2 * 1024 * 1024;
const RECORD_LIMIT: usize = 1536 * 1024;

#[derive(Clone)]
struct Entry { id: u64, attrs: BTreeMap<String,String>, version: u64, summary: Option<String> }
struct Catalog {
    entries: BTreeMap<String,Entry>, index: BTreeMap<(String,String),BTreeSet<String>>,
    epoch: u64, active: usize, offset: usize, generation: u64, corrupt_tail: bool,
    rejected_banks: usize, next_id: u64,
    index_pairs: usize, index_payload_bytes: usize, index_allocated_bytes: usize,
}
impl Catalog {
    fn empty() -> Self { Self { entries:BTreeMap::new(),index:BTreeMap::new(),epoch:0,active:0,offset:0,
        generation:0,corrupt_tail:false,rejected_banks:0,next_id:1,index_pairs:0,index_payload_bytes:0,index_allocated_bytes:0 } }
    fn remove_index(&mut self, name: &str, entry: &Entry) {
        let before = crate::mm::allocated_bytes();
        for (k,v) in &entry.attrs {
            let key = (k.clone(),v.clone());
            if let Some(set) = self.index.get_mut(&key) {
                set.remove(name);
                if set.is_empty() { self.index.remove(&key); }
            }
        }
        self.index_allocated_bytes -= before.saturating_sub(crate::mm::allocated_bytes());
    }
    fn put(&mut self, name: String, entry: Entry) {
        if let Some(old) = self.entries.remove(&name) { self.remove_index(&name, &old); }
        let before = crate::mm::allocated_bytes();
        for (k,v) in &entry.attrs { self.index.entry((k.clone(),v.clone())).or_default().insert(name.clone()); }
        self.index_allocated_bytes += crate::mm::allocated_bytes().saturating_sub(before);
        self.next_id = self.next_id.max(entry.id + 1);
        self.generation = self.generation.max(entry.version);
        self.entries.insert(name,entry);
        self.index_pairs = self.index.values().map(|s| s.len()).sum();
        self.index_payload_bytes = self.index.iter().map(|((k,v),s)| k.len()+v.len()+s.iter().map(String::len).sum::<usize>()).sum();
    }
    fn row(name: &str, entry: &Entry) -> Value {
        json!({"path":name,"id":entry.id,"attrs":entry.attrs,"version":entry.version})
    }
    fn checkpoint(&mut self) -> Result<(),isize> {
        let rows: Vec<Value> = self.entries.iter().map(|(n,e)| Self::row(n,e)).collect();
        let data = encode(&json!({"schema":1,"epoch":self.epoch+1,"entries":rows}))?;
        let target = 1 - self.active;
        let inode = ROOT_INODE.find(BANKS[target]).or_else(|| ROOT_INODE.create(BANKS[target])).ok_or(ENOSPC)?;
        inode.clear();
        if inode.write_at(0,&data) != data.len() { return Err(ENOSPC); }
        self.epoch += 1; self.active = target; self.offset = data.len(); self.corrupt_tail = false;
        Ok(())
    }
    fn persist(&mut self, name: &str, entry: &Entry) -> Result<(),isize> {
        let data = encode(&json!({"entry":Self::row(name,entry)}))?;
        if self.epoch == 0 || self.corrupt_tail || self.offset + data.len() > BANK_LIMIT { self.checkpoint()?; }
        let inode = ROOT_INODE.find(BANKS[self.active]).ok_or(ESRCH)?;
        if inode.write_at(self.offset,&data) != data.len() { return Err(ENOSPC); }
        self.offset += data.len();
        Ok(())
    }
}

pub fn reserved(name: &str) -> bool { BANKS.contains(&name) }
fn valid_path(name: &str) -> bool { !name.is_empty() && name.len() <= 27 && !name.bytes().any(|b| b==0 || b==b'/' || b==b'\\') && !reserved(name) }
fn valid_key(key: &str) -> bool { !key.is_empty() && key.len() <= 23 && key.bytes().all(|b| b.is_ascii_alphanumeric() || b==b'_' || b==b'-') }
fn checksum(bytes: &[u8]) -> u32 { bytes.iter().fold(2166136261u32,|h,b| (h ^ *b as u32).wrapping_mul(16777619)) }
fn encode(value: &Value) -> Result<Vec<u8>,isize> {
    let payload = value.to_string();
    if payload.len() > RECORD_LIMIT { return Err(ENOSPC); }
    let mut bytes = Vec::with_capacity(payload.len()+8);
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&checksum(payload.as_bytes()).to_le_bytes());
    bytes.extend_from_slice(payload.as_bytes());
    Ok(bytes)
}
fn decode_entry(row: &Value) -> Option<(String,Entry)> {
    let name = row["path"].as_str()?;
    if !valid_path(name) { return None; }
    let attrs = row["attrs"].as_object()?;
    if attrs.len() > MAX_ATTRS { return None; }
    let mut map = BTreeMap::new();
    for (key,value) in attrs {
        let value = value.as_str()?;
        if !valid_key(key) || value.len() > 63 || value.contains('\0') { return None; }
        map.insert(key.clone(),value.to_string());
    }
    let id = row["id"].as_u64()?;
    let version = row["version"].as_u64()?;
    if id == 0 || id > MAX_FILES as u64 || version == 0 { return None; }
    Some((name.to_string(),Entry {id,attrs:map,version,summary:None}))
}
fn load_bank(bank: usize) -> Option<Catalog> {
    let inode = ROOT_INODE.find(BANKS[bank])?;
    let mut catalog = Catalog::empty();
    let mut offset = 0;
    loop {
        let mut header = [0u8;8];
        let read = inode.read_at(offset,&mut header);
        if read == 0 { break; }
        if read != 8 { catalog.corrupt_tail = true; break; }
        let len = u32::from_le_bytes(header[..4].try_into().ok()?) as usize;
        if len == 0 || len > RECORD_LIMIT || offset+8+len > BANK_LIMIT { catalog.corrupt_tail=true; break; }
        let mut bytes = vec![0;len];
        if inode.read_at(offset+8,&mut bytes) != len || checksum(&bytes) != u32::from_le_bytes(header[4..].try_into().ok()?) {
            catalog.corrupt_tail=true; break;
        }
        let Ok(value) = serde_json::from_slice::<Value>(&bytes) else { catalog.corrupt_tail=true; break; };
        if offset == 0 {
            if value["schema"] != 1 { return None; }
            catalog.epoch = value["epoch"].as_u64()?;
            if catalog.epoch == 0 { return None; }
            let entries = value["entries"].as_array()?;
            if entries.len() > MAX_FILES { return None; }
            let mut ids = BTreeSet::new();
            for row in entries {
                let (name,entry) = decode_entry(row)?;
                if !ids.insert(entry.id) || catalog.entries.contains_key(&name) { return None; }
                catalog.put(name,entry);
            }
        } else {
            let Some((name,entry)) = decode_entry(&value["entry"]) else {catalog.corrupt_tail=true;break;};
            if catalog.entries.len() >= MAX_FILES && !catalog.entries.contains_key(&name) {catalog.corrupt_tail=true;break;}
            if entry.version <= catalog.generation {catalog.corrupt_tail=true;break;}
            catalog.put(name,entry);
        }
        offset += 8 + len;
    }
    if offset == 0 { return None; }
    catalog.active=bank;catalog.offset=offset;
    Some(catalog)
}
fn load() -> Catalog {
    let mut best = Catalog::empty();
    let mut rejected = 0;
    for bank in 0..2 {
        match load_bank(bank) {
            Some(c) if c.epoch > best.epoch => best=c,
            None if ROOT_INODE.find(BANKS[bank]).is_some() => rejected+=1,
            _ => (),
        }
    }
    best.rejected_banks=rejected;
    best
}
lazy_static! { static ref CATALOG: UPIntrFreeCell<Catalog> = unsafe { UPIntrFreeCell::new(load()) }; }

fn ensure(catalog: &Catalog, path: &str) -> Result<Entry,isize> {
    if !valid_path(path) {return Err(EINVAL);}
    if ROOT_INODE.find(path).is_none() {return Err(ESRCH);}
    if let Some(entry) = catalog.entries.get(path) {return Ok(entry.clone());}
    if catalog.entries.len() >= MAX_FILES {return Err(ENOSPC);}
    Ok(Entry {id:catalog.next_id,attrs:BTreeMap::new(),version:catalog.generation+1,summary:None})
}
pub fn set(path: &str, key: &str, value: Option<&str>) -> Result<Value,isize> {
    if reserved(path) {return Err(EPERM);}
    if !valid_key(key) || value.is_some_and(|v| v.len()>63 || v.contains('\0')) {return Err(EINVAL);}
    let mut c=CATALOG.exclusive_access();
    let mut entry=ensure(&c,path)?;
    if let Some(value)=value {
        if entry.attrs.len() >= MAX_ATTRS && !entry.attrs.contains_key(key) {return Err(ENOSPC);}
        entry.attrs.insert(key.to_string(),value.to_string());
    } else if entry.attrs.remove(key).is_none() {return Err(ESRCH);}
    entry.version=c.generation+1;
    c.persist(path,&entry)?;
    let row=Catalog::row(path,&entry);
    c.put(path.to_string(),entry);
    Ok(row)
}
pub fn attributes(path: &str) -> Result<Value,isize> {
    let mut c=CATALOG.exclusive_access();
    let entry=ensure(&c,path)?;
    if !c.entries.contains_key(path) { c.persist(path,&entry)?; c.put(path.to_string(),entry.clone()); }
    Ok(Catalog::row(path,&entry))
}
/// Ordinary writes invalidate cached summaries. Content is authoritative even
/// when journal allocation fails; summaries are rebuilt from content on reboot.
pub fn content_changed(path: &str) {
    let mut c=CATALOG.exclusive_access();
    if let Some(mut entry)=c.entries.get(path).cloned() {
        entry.version=c.generation+1;entry.summary=None;
        if c.persist(path,&entry).is_err() {c.corrupt_tail=true;}
        c.put(path.to_string(),entry);
    }
}
fn summary(entry: &mut Entry, path: &str) -> Result<(String,bool),isize> {
    if let Some(text)=&entry.summary {return Ok((text.clone(),true));}
    let inode=ROOT_INODE.find(path).ok_or(ESRCH)?;
    let mut bytes=[0u8;128];
    let n=inode.read_at(0,&mut bytes);
    let mut text=String::from_utf8_lossy(&bytes[..n]).into_owned();
    let mut end=text.len().min(127);
    while !text.is_char_boundary(end) {end-=1;}
    text.truncate(end);
    entry.summary=Some(text.clone());
    Ok((text,false))
}
pub fn read_summary(path: &str) -> Result<Value,isize> {
    let mut c=CATALOG.exclusive_access();
    let mut entry=ensure(&c,path)?;
    if !c.entries.contains_key(path) { c.persist(path,&entry)?; c.put(path.to_string(),entry.clone()); }
    let (text,hit)=summary(&mut entry,path)?;
    if c.entries.contains_key(path) {c.entries.insert(path.to_string(),entry.clone());}
    Ok(json!({"path":path,"id":entry.id,"version":entry.version,"summary":text,"cache_hit":hit,"summary_limit_bytes":127}))
}
pub fn query(filter: &str, keyword: &str, mode: &str, limit: i64, cursor: i64) -> Result<Value,isize> {
    if !["","index","scan"].contains(&mode) || !(1..=4).contains(&limit) || cursor<0 {return Err(EINVAL);}
    let mut conditions=Vec::new();
    for part in filter.split(',').filter(|s| !s.is_empty()) {
        let (k,v)=part.split_once('=').ok_or(EINVAL)?;
        if !valid_key(k) || v.len()>63 || conditions.len()>=4 {return Err(EINVAL);}
        conditions.push((k.to_string(),v.to_string()));
    }
    let start=crate::timer::get_time();
    let mut c=CATALOG.exclusive_access();
    let mut checked=0;
    let candidates: Vec<String> = if mode=="scan" || conditions.is_empty() {
        c.entries.keys().cloned().collect()
    } else {
        let smallest=conditions.iter().min_by_key(|pair| c.index.get(*pair).map_or(0,|s| s.len())).unwrap();
        c.index.get(smallest).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    };
    let candidate_count=candidates.len();
    let mut matches=Vec::new();
    for name in candidates {
        checked+=1;
        let entry=c.entries.get_mut(&name).unwrap();
        if !conditions.iter().all(|(k,v)| entry.attrs.get(k)==Some(v)) {continue;}
        if !keyword.is_empty() && !summary(entry,&name)?.0.contains(keyword) {continue;}
        matches.push((entry.id,name));
    }
    matches.sort_by_key(|(id,_)| *id);
    let matched=matches.len();
    let selected: Vec<_>=matches.into_iter().filter(|(id,_)| *id>=cursor as u64).collect();
    let next=selected.get(limit as usize).map(|(id,_)| *id);
    let mut rows=Vec::new();
    for (id,name) in selected.into_iter().take(limit as usize) {
        let entry=c.entries.get_mut(&name).unwrap();
        let (text,hit)=summary(entry,&name)?;
        let matched_attrs: BTreeMap<_,_>=conditions.iter().cloned().collect();
        rows.push(json!({"id":id,"path":name,"version":entry.version,"summary":text,"cache_hit":hit,"matched_attrs":matched_attrs}));
    }
    Ok(json!({"files":rows,"matched":matched,"next_cursor":next,"checked":checked,"candidates":candidate_count,
        "catalog_files":c.entries.len(),"catalog_version":c.generation,"index_pairs":c.index_pairs,
        "index_payload_bytes":c.index_payload_bytes,"index_allocated_bytes":c.index_allocated_bytes,"mode":if mode=="scan" {"scan"} else {"index"},
        "elapsed_ticks":crate::timer::get_time()-start,"clock_hz":crate::device_tree::clock_freq()}))
}
pub fn status() -> Value {
    let c=CATALOG.exclusive_access();
    json!({"files":c.entries.len(),"generation":c.generation,"epoch":c.epoch,"active_bank":c.active,
        "journal_bytes":c.offset,"corrupt_tail":c.corrupt_tail,"rejected_banks":c.rejected_banks,
        "max_files":MAX_FILES,"max_attrs":MAX_ATTRS,"bank_limit_bytes":BANK_LIMIT})
}

#[cfg(feature="agent-test")]
pub fn corrupt_tail_for_test() -> isize {
    let c=CATALOG.exclusive_access();
    if let Some(inode)=ROOT_INODE.find(BANKS[c.active]) {inode.write_at(c.offset,&[2,0,0,0,0,0,0,0,b'{',b'}']);0} else {ESRCH}
}
