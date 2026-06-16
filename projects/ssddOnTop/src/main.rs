fn main() -> anyhow::Result<()> {
    // tracing_subscriber::fmt::init();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get())
        .enable_all()
        .build()?;
    rt.block_on(ssddOnTop::run::run())?;
    Ok(())
}
