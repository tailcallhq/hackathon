use crate::app_ctx::AppCtx;
use crate::blueprint::Blueprint;
use crate::config::ConfigReader;
use crate::run;
use crate::target_runtime::TargetRuntime;
use std::sync::Arc;

pub async fn run() -> anyhow::Result<()> {
    let config_reader = ConfigReader::init();
    let path = std::env::args().collect::<Vec<_>>();
    let path = path.get(1).cloned().unwrap_or({
        let root = env!("CARGO_MANIFEST_DIR");
        format!("{}/schema/schema.graphql", root)
    });

    let config = config_reader.read(path)?;

    let blueprint = Blueprint::try_from(&config)?;
    let rt = TargetRuntime::new(&blueprint.upstream);
    let app_ctx = AppCtx::new(rt, Arc::new(blueprint));
    run::http1::start(app_ctx).await?;
    Ok(())
}
