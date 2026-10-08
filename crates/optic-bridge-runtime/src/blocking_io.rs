use std::sync::Arc;

use thiserror::Error;
use tokio::sync::Semaphore;

#[derive(Clone, Debug)]
pub struct BlockingIoGovernor {
    permits: Arc<Semaphore>,
}

impl BlockingIoGovernor {
    pub fn new(max_operations: u32) -> Result<Self, BlockingIoError> {
        let permits = usize::try_from(max_operations).map_err(|_| BlockingIoError::InvalidLimit)?;
        if permits == 0 {
            return Err(BlockingIoError::InvalidLimit);
        }
        Ok(Self {
            permits: Arc::new(Semaphore::new(permits)),
        })
    }

    pub async fn run<F, T>(&self, operation: F) -> Result<T, BlockingIoError>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let permit = Arc::clone(&self.permits)
            .acquire_owned()
            .await
            .map_err(|_| BlockingIoError::Closed)?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            operation()
        })
        .await
        .map_err(|_| BlockingIoError::JoinFailed)
    }

    #[cfg(test)]
    fn available_permits(&self) -> usize {
        self.permits.available_permits()
    }
}

#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum BlockingIoError {
    #[error("blocking I/O concurrency limit is invalid")]
    InvalidLimit,
    #[error("blocking I/O governor is closed")]
    Closed,
    #[error("blocking I/O worker failed to join")]
    JoinFailed,
}

#[cfg(test)]
mod tests {
    use std::{thread, time::Duration};

    use super::*;

    #[tokio::test]
    async fn timed_out_waiter_does_not_release_running_blocking_capacity() {
        let governor = BlockingIoGovernor::new(1).expect("governor");
        let first = governor.clone();
        let timed_out = tokio::time::timeout(
            Duration::from_millis(20),
            first.run(|| {
                thread::sleep(Duration::from_millis(120));
                1_u8
            }),
        )
        .await;
        assert!(timed_out.is_err());
        assert_eq!(governor.available_permits(), 0);

        let second = tokio::time::timeout(Duration::from_millis(20), governor.run(|| 2_u8)).await;
        assert!(
            second.is_err(),
            "running blocking task must retain its permit"
        );

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert_eq!(governor.available_permits(), 1);
        assert_eq!(governor.run(|| 3_u8).await.expect("capacity restored"), 3);
    }

    #[test]
    fn zero_capacity_is_rejected() {
        assert_eq!(
            BlockingIoGovernor::new(0).expect_err("zero must fail"),
            BlockingIoError::InvalidLimit
        );
    }
}
