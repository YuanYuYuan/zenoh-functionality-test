#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_macros)]

use zenoh::runtime::Runtime;
use futures::future::select_all;
use futures::FutureExt as _;
use std::sync::Arc;
use std::time::Duration;

use async_std::prelude::FutureExt;
use zenoh::config::{whatami::WhatAmI, Config, EndPoint};
use zenoh::prelude::r#async::*;
use zenoh::Result;
use zenoh_result::bail;
use zenoh::prelude::Locator;

const TIMEOUT: Duration = Duration::from_secs(10);

macro_rules! ztimeout {
    ($f:expr) => {
        $f.timeout(TIMEOUT).await?
    };
}

#[async_std::main]
async fn main() -> Result<()> {
    let endpoint = vec!["tcp/127.0.0.1:7450".parse()?];
    let peer_config = {
        let mut config = Config::default();
        config.set_mode(Some(WhatAmI::Peer)).unwrap();
        config.scouting.multicast.set_enabled(Some(false)).unwrap();
        config
            .connect
            .set_endpoints(endpoint)
            .unwrap();
        config
    };
    let runtime = Runtime::new(peer_config).await.unwrap();
    let manager = runtime.manager();
    let session = zenoh::init(runtime.clone());
    let receiver = zenoh::scout(WhatAmI::Peer, Config::default())
        .res()
        .await
        .unwrap();

    while let Ok(hello) = receiver.recv_async().await {
        dbg!(hello);
        // async_std::task::sleep(Duration::from_secs(1)).await;
    }

    // loop {
    //     dbg!(runtime.manager().get_locators());
    //     async_std::task::sleep(Duration::from_secs(1)).await;
    // }
    // let session = zenoh::open(peer_config.clone()).res_async().await?;
    // dbg!(session.runtime);
    Ok(())
}
