use pgrx::*;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::fmt::{Display, Formatter, Write};

#[derive(PostgresType, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[repr(C)]
#[inoutfuncs]
pub struct Sparse {
    pub entries: Vec<(usize, f32)>,
    pub n: usize,
}

impl Display for Sparse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        let mut current_entry_index = 0;
        for i in 0..self.n {
            if current_entry_index < self.entries.len() && self.entries[current_entry_index].0 == i
            {
                write!(f, "{}", self.entries[current_entry_index].1)?;
                current_entry_index += 1;
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

impl InOutFuncs for Sparse {
    fn input(input: &CStr) -> Sparse {
        let s = input.to_str().unwrap().trim_matches('[').trim_matches(']');
        let parts: Vec<&str> = s.split(',').collect();

        let mut entries = Vec::new();
        for (position, value_str) in parts.iter().enumerate() {
            let value: f32 = value_str.trim().parse().expect("Could not parse value");
            if value != 0.0 {
                entries.push((position, value));
            }
        }

        let n = parts.len();
        Sparse { entries, n }
    }

    fn output(&self, buffer: &mut StringInfo) {
        let mut output_vec = Vec::new();

        for i in 0..self.n {
            let value = self
                .entries
                .iter()
                .find(|&&(position, _)| position == i)
                .map(|&(_, value)| value)
                .unwrap_or(0.0);

            output_vec.push(format!("{}", value));
        }

        let output_str = format!("[{}]", output_vec.join(","));
        buffer.write_fmt(format_args!("{}", output_str)).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_sparse_display() {
        let sparse = Sparse {
            entries: vec![(0, 1.0), (2, 3.0)],
            n: 4,
        };

        let output = format!("{}", sparse);
        assert_eq!(output, "[1,0,3,0]");
    }

    #[test]
    fn test_sparse_input() {
        let input_str = CString::new("[1,0,3,0]").unwrap();
        let sparse = Sparse::input(&input_str);

        assert_eq!(
            sparse,
            Sparse {
                entries: vec![(0, 1.0), (2, 3.0)],
                n: 4,
            }
        );
    }
}
