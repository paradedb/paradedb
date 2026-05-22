use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct SuperKMeansIterationStats {
    pub iteration: usize,
    pub objective: f32,
    pub shift: f32,
    pub split: usize,
    pub recall: f32,
    pub not_pruned_pct: f32,
    pub partial_d: usize,
    pub is_gemm_only: bool,
}

impl Default for SuperKMeansIterationStats {
    fn default() -> Self {
        Self {
            iteration: 0,
            objective: 0.0,
            shift: 0.0,
            split: 0,
            recall: 0.0,
            not_pruned_pct: -1.0,
            partial_d: 0,
            is_gemm_only: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterBalanceStats {
    pub mean: f32,
    pub geometric_mean: f32,
    pub stdev: f32,
    pub cv: f32,
    pub min: usize,
    pub max: usize,
}

impl Default for ClusterBalanceStats {
    fn default() -> Self {
        Self {
            mean: 0.0,
            geometric_mean: 0.0,
            stdev: 0.0,
            cv: 0.0,
            min: 0,
            max: 0,
        }
    }
}

impl ClusterBalanceStats {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"mean\":{},\"geometric_mean\":{},\"stdev\":{},\"cv\":{},\"min\":{},\"max\":{}}}",
            self.mean, self.geometric_mean, self.stdev, self.cv, self.min, self.max
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuperKMeansError {
    InvalidArgument(String),
    Runtime(String),
}

impl fmt::Display for SuperKMeansError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SuperKMeansError::InvalidArgument(msg) => write!(f, "{msg}"),
            SuperKMeansError::Runtime(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SuperKMeansError {}

pub type Result<T> = std::result::Result<T, SuperKMeansError>;

pub fn ensure_positive_usize(value: usize, name: &str) -> Result<()> {
    if value == 0 {
        return Err(SuperKMeansError::InvalidArgument(format!(
            "Value must be positive: {name}"
        )));
    }
    Ok(())
}
