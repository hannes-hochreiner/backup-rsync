use anyhow::Context;
use backup_rsync::{config::Config, sync::Sync};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // init logger
    env_logger::init();
    // get parameters
    let config_path = std::env::var("BACK_UP_RSYNC_CONFIG")?;
    let config =
        Config::read_from_file(Path::new(&config_path)).context("could not read config file")?;
    // create sync object
    let sync = Sync::new(config);

    sync.execute().context("error executing the sync")
}
