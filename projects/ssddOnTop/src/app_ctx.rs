use crate::blueprint::Blueprint;
use crate::target_runtime::TargetRuntime;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppCtx {
    pub runtime: TargetRuntime,
    pub blueprint: Arc<Blueprint>,
}

impl AppCtx {
    pub fn new(runtime: TargetRuntime, blueprint: Arc<Blueprint>) -> Self {
        Self { runtime, blueprint }
    }
}
