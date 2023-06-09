#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_macros)]
#![allow(unreachable_code)]

use futures::future::select_all;
use futures::FutureExt as _;
use std::sync::Arc;
use std::time::Duration;
use zenoh::runtime::Runtime;

use async_std::prelude::FutureExt;
use zenoh::config::{whatami::WhatAmI, Config, EndPoint};
use zenoh::prelude::r#async::*;
use zenoh::prelude::Locator;
use zenoh::Result;
use zenoh_result::bail;

const TIMEOUT: Duration = Duration::from_secs(10);

macro_rules! ztimeout {
    ($f:expr) => {
        $f.timeout(TIMEOUT).await?
    };
}

#[async_std::main]
async fn main() -> Result<()> {
    let connect_endpoint = vec!["tcp/127.0.0.1:7450".parse()?];
    let listen_endpoint = vec!["tcp/127.0.0.1:7452".parse()?];
    let peer_config = {
        let mut config = Config::default();
        config.set_mode(Some(WhatAmI::Peer)).unwrap();
        config.scouting.multicast.set_enabled(Some(false)).unwrap();
        config.connect.set_endpoints(connect_endpoint).unwrap();
        config.listen.set_endpoints(listen_endpoint).unwrap();
        config
    };
    let runtime = Runtime::new(peer_config).await.unwrap();
    let manager = runtime.manager();
    let session = zenoh::init(runtime.clone()).res_async().await?;
    let receiver = zenoh::scout(WhatAmI::Peer, Config::default())
        .res()
        .await
        .unwrap();

    let sub = session
        .declare_subscriber("demo/example/zenoh-rs-pub")
        .callback(|msg| {
            // dbg!("recv");
        })
        .res_async()
        .await?;

    async_std::task::sleep(Duration::from_secs(1)).await;
    dbg!(session
        .info()
        .routers_zid()
        .res_async()
        .await
        .collect::<Vec<ZenohId>>());
    dbg!(session
        .info()
        .peers_zid()
        .res_async()
        .await
        .collect::<Vec<ZenohId>>());

    // dbg!(runtime.get_locators());
    // dbg!(runtime.manager().get_listeners());
    // dbg!(runtime.manager().get_transports());
    // while let Ok(hello) = receiver.recv_async().await {
    //     dbg!(hello);
    //     // async_std::task::sleep(Duration::from_secs(1)).await;
    // }

    let mut cnt = 0;
    loop {
        // dbg!(runtime.manager().get_locators());
        dbg!(runtime.manager().get_transports());
        session.put("demo/example/put", "test").res_async().await?;
        for trans in runtime.manager().get_transports() {
            if cnt <= 3 && trans.get_whatami()? == WhatAmI::Peer {
                trans.close().await?;
                cnt += 1;
            }
        }
        async_std::task::sleep(Duration::from_secs(1)).await;
    }
    // let session = zenoh::open(peer_config.clone()).res_async().await?;
    // dbg!(session.runtime);
    Ok(())
}
