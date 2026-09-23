//! `engine-cli replay`: serve a recorded fixture over the same protocol, at a chosen speed.

use anyhow::Context;

use engine::observe::identity::data_dir;
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
    let replayer = Replayer::new(fixture, shown)?;
    let server = Server::bind(&data, &replayer.hello().match_id)?;
    println!("{}", server.port());
    // The page address is printed after the port, because it is the line a reader copies.
    let page = match opts.web.as_deref() {
        Some(dir) => Some(crate::web::start(
            dir,
            std::sync::Arc::new(crate::web::Fixed {
                socket_port: server.port(),
                match_id: replayer.hello().match_id.clone(),
            }),
        )?),
        None => None,
    };
    if let Some(page) = &page {
        println!("{}", page.address());
    }
    let sent = replayer.serve(&server, opts.speed, opts.sustain)?;
    Ok(if sent as usize == replayer.fixture().frames.len() {
        0
    } else {
        2
    })
}
