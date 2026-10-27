use crate::paths::AppPaths;

pub(super) fn run_app(paths: &AppPaths) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(crate::api::run_server(paths))
}
