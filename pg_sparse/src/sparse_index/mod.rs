use pgrx::*;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Write};
use std::ffi::CStr;

const MAX_ENTRIES: usize = 10; // Adjust based on your needs

#[derive(PostgresType, Serialize, Deserialize, Copy, Clone, Debug)]
#[repr(C)]
#[pgvarlena_inoutfuncs]
pub struct Sparse {
    pub entries: [(i32, f64); MAX_ENTRIES], // Fixed size array
    pub n: i32,
}

impl Sparse {
    pub fn new(entries: &[(i32, f64)], n: i32) -> Self {
        let mut new_entries = [(0, 0.0); MAX_ENTRIES];
        let mut count = 0;
        for &entry in entries {
            new_entries[count] = entry;
            count += 1;
        }
        Self { entries: new_entries, n }
    }
}

impl Display for Sparse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut current_entry = 0;
        for i in 0..self.n {
            if current_entry < MAX_ENTRIES && self.entries[current_entry].0 == i {
                write!(f, "{}", self.entries[current_entry].1)?;
                current_entry += 1;
            } else {
                write!(f, "0")?;
            }
            if i < self.n - 1 {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl PgVarlenaInOutFuncs for Sparse {
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s = input.to_str().unwrap().trim_matches('[').trim_matches(']');
        let parts: Vec<&str> = s.split(',').collect();
        let mut result: PgVarlena<Sparse> = PgVarlena::new();
        
        let mut entries = [(0, 0.0); MAX_ENTRIES];
        let mut count = 0;
        for (position, value_str) in parts.iter().enumerate() {
            let value: f64 = value_str.parse().unwrap();
            if value != 0.0 {
                entries[count] = (position as i32, value);
                count += 1;
            }
        }

        let n = parts.len() as i32;
        result.entries = entries;
        result.n = 10;
        result
    }

    fn output(&self, buffer: &mut StringInfo) {
        let mut output_vec = Vec::new();
    
        for i in 0..self.n {
            let value = self.entries.iter().find(|&&(position, _)| position == i)
                .map(|&(_, value)| value) // If yes, get the value
                .unwrap_or(0.0); // If no, use 0
            
            output_vec.push(format!("{}", value));
        }
        
        let output_str = format!("[{}]", output_vec.join(","));
        buffer.write_fmt(format_args!("{}", output_str)).unwrap();
    }
}

pub struct SparseIndex {
    pub name: String,
}

impl SparseIndex {
    pub fn new(name: String) -> Self {
        info!("TODO: Create HNSW index");
        Self { name: name }
    }

    pub fn from_index_name(name: String) -> Self {
        info!("TODO: Retrieve HNSW index");
        Self { name: name }
    }

    pub fn insert(&mut self, sparse_vector: Sparse, heap_tid: pg_sys::ItemPointerData) {
        info!(
            "TODO: Insert {:?} with ID {:?} into index",
            sparse_vector, heap_tid
        );
    }

    pub fn search(self, sparse_vector: Sparse) -> Vec<pg_sys::ItemPointerData> {
        info!("TODO: Implement HNSW search to return results sorted by ID {:?}", sparse_vector);
        vec![]
    }

    pub fn bulk_delete(
        &self,
        stats_binding: *mut pg_sys::IndexBulkDeleteResult,
        callback: pg_sys::IndexBulkDeleteCallback,
        callback_state: *mut ::std::os::raw::c_void,
    ) {
        info!("TODO: Implement delete")
    }
}
