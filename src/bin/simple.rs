use std::time::Duration;
use zenoh::runtime::Runtime;
use zenoh::config::{whatami::WhatAmI, Config};
use zenoh::prelude::r#async::*;
use zenoh::Result;

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
    let _manager = runtime.manager();
    let session = zenoh::init(runtime.clone()).res_async().await?;
    let _receiver = zenoh::scout(WhatAmI::Peer, Config::default())
        .res()
        .await
        .unwrap();

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

    for cnt in 0..30 {
        dbg!(runtime.manager().get_transports());
        session.put("demo/example/put", "test").res_async().await?;
        for trans in runtime.manager().get_transports() {
            if 5 <= cnt && cnt <= 10 && trans.get_whatami()? == WhatAmI::Peer {
                trans.close().await?;
            }
        }
        async_std::task::sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}
