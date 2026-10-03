use thiserror::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub timeout_ms: u64,
    pub output_bytes: u64,
    pub memory_bytes: u64,
    pub process_count: u32,
}

impl ResourceBudget {
    #[must_use]
    pub const fn fits_within(self, ceiling: Self) -> bool {
        self.timeout_ms <= ceiling.timeout_ms
            && self.output_bytes <= ceiling.output_bytes
            && self.memory_bytes <= ceiling.memory_bytes
            && self.process_count <= ceiling.process_count
    }

    pub fn validate_nonzero(self) -> Result<Self, LimitError> {
        if self.timeout_ms == 0
            || self.output_bytes == 0
            || self.memory_bytes == 0
            || self.process_count == 0
        {
            return Err(LimitError::ZeroIsNotUnlimited);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HardLimits {
    pub max_request_bytes: u64,
    pub max_response_bytes: u64,
    pub max_active_output_ram_bytes: u64,
    pub max_process_budget: ResourceBudget,
}

impl Default for HardLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: 1024 * 1024,
            max_response_bytes: 256 * 1024,
            max_active_output_ram_bytes: 16 * 1024 * 1024,
            max_process_budget: ResourceBudget {
                timeout_ms: 60 * 60 * 1000,
                output_bytes: 16 * 1024 * 1024,
                memory_bytes: 8 * 1024 * 1024 * 1024,
                process_count: 32,
            },
        }
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LimitError {
    #[error("zero is not interpreted as unlimited")]
    ZeroIsNotUnlimited,
    #[error("requested resource budget exceeds its authorized ceiling")]
    ExceedsCeiling,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_never_means_unlimited() {
        let budget = ResourceBudget {
            timeout_ms: 0,
            output_bytes: 1,
            memory_bytes: 1,
            process_count: 1,
        };
        assert_eq!(
            budget.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }
}
