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
    pub max_sessions: u32,
    pub max_task_leases: u32,
    pub max_task_leases_per_session: u32,
    pub max_approval_grants: u32,
    pub max_approval_grants_per_session: u32,
    pub max_active_output_ram_bytes: u64,
    pub max_active_output_ram_bytes_per_session: u64,
    pub max_active_process_memory_bytes: u64,
    pub max_active_process_memory_bytes_per_session: u64,
    pub min_host_memory_headroom_bytes: u64,
    pub min_host_memory_headroom_percent: u32,
    pub max_fs_read_bytes: u64,
    pub max_fs_mutation_bytes: u64,
    pub max_mutation_journal_file_bytes: u64,
    pub max_mutation_recovery_records: u32,
    pub max_fs_list_page_entries: u32,
    pub max_fs_directory_scan_entries: u32,
    pub max_git_read_bytes: u64,
    pub max_git_log_entries: u32,
    pub max_active_process_jobs: u32,
    pub max_active_process_jobs_per_session: u32,
    pub max_active_heavy_process_jobs: u32,
    pub max_active_heavy_process_jobs_per_session: u32,
    pub max_process_records: u32,
    pub max_process_records_per_session: u32,
    pub max_process_read_bytes: u64,
    pub max_process_cpu_percent_per_job: u32,
    pub max_active_process_cpu_percent: u32,
    pub max_active_process_cpu_percent_per_session: u32,
    pub max_process_budget: ResourceBudget,
}

impl Default for HardLimits {
    fn default() -> Self {
        Self {
            max_request_bytes: 1024 * 1024,
            max_response_bytes: 256 * 1024,
            max_request_duration_ms: 30_000,
            max_concurrent_requests: 16,
            max_sessions: 16,
            max_task_leases: 128,
            max_task_leases_per_session: 32,
            max_approval_grants: 128,
            max_approval_grants_per_session: 32,
            max_active_output_ram_bytes: 16 * 1024 * 1024,
            max_active_output_ram_bytes_per_session: 8 * 1024 * 1024,
            max_active_process_memory_bytes: 16 * 1024 * 1024 * 1024,
            max_active_process_memory_bytes_per_session: 8 * 1024 * 1024 * 1024,
            min_host_memory_headroom_bytes: 1024 * 1024 * 1024,
            min_host_memory_headroom_percent: 10,
            max_fs_read_bytes: 256 * 1024,
            max_fs_mutation_bytes: 8 * 1024 * 1024,
            max_mutation_journal_file_bytes: 64 * 1024,
            max_mutation_recovery_records: 256,
            max_fs_list_page_entries: 256,
            max_fs_directory_scan_entries: 4096,
            max_git_read_bytes: 256 * 1024,
            max_git_log_entries: 256,
            max_active_process_jobs: 8,
            max_active_process_jobs_per_session: 4,
            max_active_heavy_process_jobs: 2,
            max_active_heavy_process_jobs_per_session: 1,
            max_process_records: 64,
            max_process_records_per_session: 32,
            max_process_read_bytes: 64 * 1024,
            max_process_cpu_percent_per_job: 25,
            max_active_process_cpu_percent: 75,
            max_active_process_cpu_percent_per_session: 50,
            max_process_budget: ResourceBudget {
                timeout_ms: 60 * 60 * 1000,
                output_bytes: 8 * 1024 * 1024,
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
            || self.max_sessions == 0
            || self.max_task_leases == 0
            || self.max_task_leases_per_session == 0
            || self.max_approval_grants == 0
            || self.max_approval_grants_per_session == 0
            || self.max_active_output_ram_bytes == 0
            || self.max_active_output_ram_bytes_per_session == 0
            || self.max_active_process_memory_bytes == 0
            || self.max_active_process_memory_bytes_per_session == 0
            || self.min_host_memory_headroom_bytes == 0
            || self.min_host_memory_headroom_percent == 0
            || self.max_fs_read_bytes == 0
            || self.max_fs_mutation_bytes == 0
            || self.max_mutation_journal_file_bytes == 0
            || self.max_mutation_recovery_records == 0
            || self.max_fs_list_page_entries == 0
            || self.max_fs_directory_scan_entries == 0
            || self.max_git_read_bytes == 0
            || self.max_git_log_entries == 0
            || self.max_active_process_jobs == 0
            || self.max_active_process_jobs_per_session == 0
            || self.max_active_heavy_process_jobs == 0
            || self.max_active_heavy_process_jobs_per_session == 0
            || self.max_process_records == 0
            || self.max_process_records_per_session == 0
            || self.max_process_read_bytes == 0
            || self.max_process_cpu_percent_per_job == 0
            || self.max_active_process_cpu_percent == 0
            || self.max_active_process_cpu_percent_per_session == 0
        {
            return Err(LimitError::ZeroIsNotUnlimited);
        }
        if self.max_task_leases_per_session > self.max_task_leases
            || self.max_approval_grants_per_session > self.max_approval_grants
            || self.max_process_records < self.max_active_process_jobs
            || self.max_active_process_jobs_per_session > self.max_active_process_jobs
            || self.max_active_heavy_process_jobs > self.max_active_process_jobs
            || self.max_active_heavy_process_jobs_per_session > self.max_active_heavy_process_jobs
            || self.max_active_heavy_process_jobs_per_session
                > self.max_active_process_jobs_per_session
            || self.max_process_records_per_session > self.max_process_records
            || self.max_process_records_per_session < self.max_active_process_jobs_per_session
            || self.max_active_output_ram_bytes_per_session > self.max_active_output_ram_bytes
            || self.max_process_budget.output_bytes > self.max_active_output_ram_bytes_per_session
            || self.max_active_process_memory_bytes_per_session
                > self.max_active_process_memory_bytes
            || self.max_process_budget.memory_bytes
                > self.max_active_process_memory_bytes_per_session
            || self.min_host_memory_headroom_percent > 100
            || self.max_process_cpu_percent_per_job > 100
            || self.max_active_process_cpu_percent > 100
            || self.max_active_process_cpu_percent_per_session > 100
            || self.max_process_cpu_percent_per_job > self.max_active_process_cpu_percent
            || self.max_process_cpu_percent_per_job
                > self.max_active_process_cpu_percent_per_session
            || self.max_active_process_cpu_percent_per_session > self.max_active_process_cpu_percent
        {
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
    fn hard_limits_reject_zero_task_lease_capacity() {
        let limits = HardLimits {
            max_task_leases: 0,
            ..HardLimits::default()
        };
        assert_eq!(
            limits.validate_nonzero().expect_err("zero must fail"),
            LimitError::ZeroIsNotUnlimited
        );
    }

    #[test]
    fn task_lease_per_session_capacity_must_fit_global_capacity() {
        let limits = HardLimits {
            max_task_leases: 2,
            max_task_leases_per_session: 3,
            ..HardLimits::default()
        };
        assert_eq!(
            limits
                .validate_nonzero()
                .expect_err("relationship must fail"),
            LimitError::InvalidRelationship
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

    #[test]
    fn per_session_process_limits_must_fit_global_limits() {
        let limits = HardLimits {
            max_active_process_jobs_per_session: 9,
            ..HardLimits::default()
        };
        assert_eq!(
            limits
                .validate_nonzero()
                .expect_err("relationship must fail"),
            LimitError::InvalidRelationship
        );

        let limits = HardLimits {
            max_process_records_per_session: 3,
            max_active_process_jobs_per_session: 4,
            ..HardLimits::default()
        };
        assert_eq!(
            limits
                .validate_nonzero()
                .expect_err("relationship must fail"),
            LimitError::InvalidRelationship
        );

        let limits = HardLimits {
            max_active_output_ram_bytes_per_session: 17 * 1024 * 1024,
            ..HardLimits::default()
        };
        assert_eq!(
            limits
                .validate_nonzero()
                .expect_err("relationship must fail"),
            LimitError::InvalidRelationship
        );

        let limits = HardLimits {
            max_active_output_ram_bytes_per_session: 4 * 1024 * 1024,
            ..HardLimits::default()
        };
        assert_eq!(
            limits
                .validate_nonzero()
                .expect_err("per-job output budget must fit the session ceiling"),
            LimitError::InvalidRelationship
        );
    }

    #[test]
    fn process_memory_reservation_limits_are_bounded() {
        let defaults = HardLimits::default();
        assert_eq!(
            defaults.max_active_process_memory_bytes,
            16 * 1024 * 1024 * 1024
        );
        assert_eq!(
            defaults.max_active_process_memory_bytes_per_session,
            8 * 1024 * 1024 * 1024
        );
        assert!(defaults.validate_nonzero().is_ok());

        let zero = HardLimits {
            max_active_process_memory_bytes: 0,
            ..defaults
        };
        assert_eq!(
            zero.validate_nonzero()
                .expect_err("zero aggregate memory must fail"),
            LimitError::ZeroIsNotUnlimited
        );

        let session_exceeds_global = HardLimits {
            max_active_process_memory_bytes: 4 * 1024 * 1024 * 1024,
            max_active_process_memory_bytes_per_session: 8 * 1024 * 1024 * 1024,
            ..defaults
        };
        assert_eq!(
            session_exceeds_global
                .validate_nonzero()
                .expect_err("session memory must fit global memory"),
            LimitError::InvalidRelationship
        );

        let job_exceeds_session = HardLimits {
            max_active_process_memory_bytes_per_session: 4 * 1024 * 1024 * 1024,
            ..defaults
        };
        assert_eq!(
            job_exceeds_session
                .validate_nonzero()
                .expect_err("per-job memory must fit session aggregate memory"),
            LimitError::InvalidRelationship
        );
    }

    #[test]
    fn host_memory_headroom_limits_are_bounded() {
        let defaults = HardLimits::default();
        assert_eq!(defaults.min_host_memory_headroom_bytes, 1024 * 1024 * 1024);
        assert_eq!(defaults.min_host_memory_headroom_percent, 10);
        assert!(defaults.validate_nonzero().is_ok());

        for limits in [
            HardLimits {
                min_host_memory_headroom_bytes: 0,
                ..defaults
            },
            HardLimits {
                min_host_memory_headroom_percent: 0,
                ..defaults
            },
        ] {
            assert_eq!(
                limits
                    .validate_nonzero()
                    .expect_err("zero headroom must fail"),
                LimitError::ZeroIsNotUnlimited
            );
        }

        let invalid_percent = HardLimits {
            min_host_memory_headroom_percent: 101,
            ..defaults
        };
        assert_eq!(
            invalid_percent
                .validate_nonzero()
                .expect_err("headroom percent above 100 must fail"),
            LimitError::InvalidRelationship
        );
    }

    #[test]
    fn heavy_process_slot_limits_are_bounded() {
        let defaults = HardLimits::default();
        assert_eq!(defaults.max_active_heavy_process_jobs, 2);
        assert_eq!(defaults.max_active_heavy_process_jobs_per_session, 1);
        assert!(defaults.validate_nonzero().is_ok());

        for limits in [
            HardLimits {
                max_active_heavy_process_jobs: 0,
                ..defaults
            },
            HardLimits {
                max_active_heavy_process_jobs_per_session: 0,
                ..defaults
            },
        ] {
            assert_eq!(
                limits
                    .validate_nonzero()
                    .expect_err("zero heavy-process slots must fail"),
                LimitError::ZeroIsNotUnlimited
            );
        }

        for limits in [
            HardLimits {
                max_active_heavy_process_jobs: defaults.max_active_process_jobs + 1,
                ..defaults
            },
            HardLimits {
                max_active_heavy_process_jobs: 1,
                max_active_heavy_process_jobs_per_session: 2,
                ..defaults
            },
            HardLimits {
                max_active_heavy_process_jobs_per_session: defaults
                    .max_active_process_jobs_per_session
                    + 1,
                ..defaults
            },
        ] {
            assert_eq!(
                limits
                    .validate_nonzero()
                    .expect_err("heavy-process slot relationship must fail"),
                LimitError::InvalidRelationship
            );
        }
    }

    #[test]
    fn process_cpu_limits_are_bounded_and_leave_default_headroom() {
        let defaults = HardLimits::default();
        assert_eq!(defaults.max_process_cpu_percent_per_job, 25);
        assert_eq!(defaults.max_active_process_cpu_percent, 75);
        assert_eq!(defaults.max_active_process_cpu_percent_per_session, 50);
        assert!(defaults.validate_nonzero().is_ok());

        for limits in [
            HardLimits {
                max_process_cpu_percent_per_job: 0,
                ..defaults
            },
            HardLimits {
                max_active_process_cpu_percent: 101,
                ..defaults
            },
            HardLimits {
                max_process_cpu_percent_per_job: 60,
                ..defaults
            },
            HardLimits {
                max_active_process_cpu_percent_per_session: 80,
                ..defaults
            },
        ] {
            assert!(limits.validate_nonzero().is_err());
        }
    }

    #[test]
    fn approval_grant_limits_are_bounded() {
        let defaults = HardLimits::default();
        assert_eq!(defaults.max_approval_grants, 128);
        assert_eq!(defaults.max_approval_grants_per_session, 32);
        assert!(defaults.validate_nonzero().is_ok());

        let zero = HardLimits {
            max_approval_grants: 0,
            ..defaults
        };
        assert_eq!(
            zero.validate_nonzero()
                .expect_err("zero approval capacity must fail"),
            LimitError::ZeroIsNotUnlimited
        );

        let invalid = HardLimits {
            max_approval_grants: 1,
            max_approval_grants_per_session: 2,
            ..defaults
        };
        assert_eq!(
            invalid
                .validate_nonzero()
                .expect_err("per-session approvals must fit global capacity"),
            LimitError::InvalidRelationship
        );
    }
}
