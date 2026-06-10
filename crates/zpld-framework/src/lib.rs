use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, PartialEq)]
pub enum StabilityStatus {
    Stable,
    Unstable,
}

#[derive(Debug)]
pub enum WorkerError {
    Init(String),
    Heartbeat(String),
    Drain(String),
    Handoff(String),
    Io(std::io::Error),
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkerState {
    Starting,
    Running,
    UpdatePending,
    Draining,
    Dead,
}

#[derive(Debug)]
pub struct StabilityCondition {
    pub name: String,
    pub blocking: bool,
    pub status: StabilityStatus,
    pub reason: Option<String>,
}

#[derive(Debug)]
pub struct StateStoreSlot {}

#[derive(Debug)]
pub struct FdStore {}

#[derive(Debug)]
pub struct WorkerContext {
    pub worker_id: String,
    pub config_path: PathBuf,
    pub state_store: StateStoreSlot,
    pub fd_store: FdStore,
}

pub trait Worker {
    fn init(&mut self, ctx: WorkerContext) -> Result<(), WorkerError>;
    fn heartbeat(&mut self) -> Result<(), WorkerError>;
    fn stability(&self) -> Vec<StabilityCondition>;
    fn drain(&mut self) -> Result<(), WorkerError>;
    fn handoff(&mut self, store: &mut FdStore) -> Result<(), WorkerError>;
    fn status_blob(&self) -> HashMap<String, String>;
}

pub fn is_patch_blocked(conditions: &[StabilityCondition]) -> bool {
    conditions.iter().any(|c| c.blocking && c.status == StabilityStatus::Unstable)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubWorker;

    impl Worker for StubWorker {
        fn init(&mut self, _ctx: WorkerContext) -> Result<(), WorkerError> {
            todo!()
        }
        fn heartbeat(&mut self) -> Result<(), WorkerError> {
            todo!()
        }
        fn stability(&self) -> Vec<StabilityCondition> {
            todo!()
        }
        fn drain(&mut self) -> Result<(), WorkerError> {
            todo!()
        }
        fn handoff(&mut self, _store: &mut FdStore) -> Result<(), WorkerError> {
            todo!()
        }
        fn status_blob(&self) -> HashMap<String, String> {
            todo!()
        }
    }

    // FT-FR-01: Worker trait is object-safe — Box<dyn Worker> must compile
    #[test]
    fn test_ft_fr_01_worker_trait_object_safe() {
        let _: Box<dyn Worker> = Box::new(StubWorker);
    }

    // FT-FR-03: a blocking+Unstable condition must hold the patch
    #[test]
    fn test_ft_fr_03_blocking_holds_patch() {
        let conditions = vec![StabilityCondition {
            name: String::from("ike_exchange"),
            blocking: true,
            status: StabilityStatus::Unstable,
            reason: Some(String::from("IKEv2 exchange in progress")),
        }];
        assert!(
            is_patch_blocked(&conditions),
            "patch should be blocked when a blocking condition is Unstable"
        );
    }

    // FT-FR-03: a non-blocking+Unstable condition must NOT hold the patch
    #[test]
    fn test_ft_fr_03_nonblocking_does_not_hold() {
        let conditions = vec![StabilityCondition {
            name: String::from("sa_lifetime"),
            blocking: false,
            status: StabilityStatus::Unstable,
            reason: Some(String::from("SA expires in 8 minutes")),
        }];
        assert!(
            !is_patch_blocked(&conditions),
            "patch should not be blocked when only non-blocking conditions are Unstable"
        );
    }
}
