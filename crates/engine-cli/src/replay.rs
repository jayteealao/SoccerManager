//! `engine-cli replay`: serve a recorded fixture over the same protocol, at a chosen speed.

use anyhow::Context;

use engine::observe::identity::{data_dir, load_or_create_owner_id};
use stream::{Replayer, Server, read_fixture};

use crate::cli::ReplayOpts;

pub fn run(opts: &ReplayOpts) -> anyhow::Result<i32> {
    let fixture = read_fixture(&opts.fixture)
        .with_context(|| format!("cannot replay {}", opts.fixture.display()))?;
    let shown = opts
        .fixture
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| opts.fixture.display().to_string());
    let data = data_dir();
    let owner_id = load_or_create_owner_id(&data)?;
    let replayer = Replayer::new(fixture, shown);
    let server = Server::bind(&data, &replayer.hello(&owner_id).match_id)?;
    println!("{}", server.port());
    let sent = replayer.serve(&server, &owner_id, opts.speed)?;
    Ok(if sent as usize == replayer.fixture().frames.len() {
        0
    } else {
        2
    })
}
