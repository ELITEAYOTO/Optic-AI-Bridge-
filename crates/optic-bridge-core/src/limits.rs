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
    pub max_request_duration_ms: u64,
    pub max_concurrent_requests: u32,
    pub max_active_output_ram_bytes: u64,
    pub max_fs_read_bytes: u64,
    pub max_fs_mutation_bytes: u64,
    pub max_mutation_journal_file_bytes: u64,
    pub max_mutation_recovery_records: u32,
    pub max_fs_list_page_entries: u32,
    pub max_fs_directory_scan_entries: u32,
    pub max_git_read_bytes: u64,
    pub max_git_log_entries: u32,
    pub max_active_process_jobs: u32,
    pub max_process_records: u32,
    pub max_process_read_bytes: u64,
    pub max_process_budget: ResourceBudget,
}

impl Default for HardLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: 1024 * 1024,
            max_response_bytes: 256 * 1024,
            max_request_duration_ms: 30_000,
            max_concurrent_requests: 16,
            max_active_output_ram_bytes: 16 * 1024 * 1024,
            max_fs_read_bytes: 256 * 1024,
            max_fs_mutation_bytes: 8 * 1024 * 1024,
            max_mutation_journal_file_bytes: 64 * 1024,
            max_mutation_recovery_records: 256,
            max_fs_list_page_entries: 256,
            max_fs_directory_scan_entries: 4096,
            max_git_read_bytes: 256 * 1024,
            max_git_log_entries: 256,
            max_active_process_jobs: 8,
            max_process_records: 64,
            max_process_read_bytes: 64 * 1024,
            max_process_budget: ResourceBudget {
                timeout_ms: 60 * 60 * 1000,
                output_bytes: 16 * 1024 * 1024,
                memory_bytes: 8 * 1024 * 1024 * 1024,
                process_count: 32,
            },
        }
    }
}

impl HardLimits {
    pub fn validate_nonzero(self) -> Result<Self, LimitError> {
        if self.max_request_bytes == 0
            || self.max_response_bytes == 0
            || self.max_request_duration_ms == 0
            || self.max_concurrent_requests == 0
            || self.max_active_output_ram_bytes == 0
            || self.max_fs_read_bytes == 0
            || self.max_fs_mutation_bytes == 0
            || self.max_mutation_journal_file_bytes == 0
            || self.max_mutation_recovery_records == 0
            || self.max_fs_list_page_entries == 0
            || self.max_fs_directory_scan_entries == 0
            || self.max_git_read_bytes == 0
            || self.max_git_log_entries == 0
            || self.max_active_process_jobs == 0
            || self.max_process_records == 0
            || self.max_process_read_bytes == 0
        {
            return Err(LimitError::ZeroIsNotUnlimited);
        }
        if self.max_process_records < self.max_active_process_jobs {
            return Err(LimitError::InvalidRelationship);
        }
        self.max_process_budget.validate_nonzero()?;
        Ok(self)
    }
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum LimitError {
    #[error("zero is not interpreted as unlimited")]
    ZeroIsNotUnlimited,
    #[error("requested resource budget exceeds its authorized ceiling")]
    ExceedsCeiling,
    #[error("hard-limit relationships are invalid")]
    InvalidRelationship,
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

    #[test]
    fn hard_limits_reject_zero_concurrency() {
        let limits = HardLimits {
            max_concurrent_requests: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn hard_limits_reject_zero_mutation_ceiling() {
        let limits = HardLimits {
            max_fs_mutation_bytes: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn hard_limits_reject_zero_recovery_journal_ceiling() {
        let limits = HardLimits {
            max_mutation_journal_file_bytes: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn hard_limits_reject_zero_recovery_record_ceiling() {
        let limits = HardLimits {
            max_mutation_recovery_records: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn hard_limits_reject_zero_git_read_ceiling() {
        let limits = HardLimits {
            max_git_read_bytes: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn hard_limits_reject_zero_git_log_entries() {
        let limits = HardLimits {
            max_git_log_entries: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn process_record_limit_must_cover_active_jobs() {
        let limits = HardLimits {
            max_active_process_jobs: 4,
            max_process_records: 3,
            ..HardLimits::default()
        };
        assert_eq!(
            limits
                .validate_nonzero()
                .expect_err("relationship must fail"),
            LimitError::InvalidRelationship
        );
    }
}
