#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use async_std::prelude::FutureExt;
use futures::future::join_all;
use futures::select;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use async_std::task;
use zenoh::config::{whatami::WhatAmI, Config, EndPoint};
use zenoh::prelude::r#async::*;

const TIMEOUT: Duration = Duration::from_secs(60);
const SLEEP: Duration = Duration::from_secs(1);

macro_rules! ztimeout {
    ($f:expr) => {
        $f.timeout(TIMEOUT).await.unwrap()
    };
}

#[derive(Debug)]
enum Action {
    Pub,
    Sub,
    Park,
}

#[derive(Debug)]
struct Task {
    topic: String,
    action: Action,
}

#[derive(Debug)]
struct Node {
    name: String,
    mode: WhatAmI,
    listen: Vec<String>,
    connect: Vec<String>,
    task: Vec<Task>,
}

#[async_std::main]
async fn main() {
    let nodes = vec![
        // Node {
        //     name: "C:Router".into(),
        //     mode: WhatAmI::Router,
        //     listen: vec!["tcp/127.0.0.1:17447".into()],
        //     connect: vec![],
        //     task: vec![],
        // },
        Node {
            name: "A:PubClient".into(),
            mode: WhatAmI::Peer,
            // listen: vec![],
            listen: vec!["tcp/127.0.0.1:17447".into()],
            // connect: vec!["tcp/127.0.0.1:17447".into()],
            connect: vec![],
            task: vec![Task {
                topic: "myTestTopic".into(),
                action: Action::Pub,
            }],
        },
        Node {
            name: "B:SubClient".into(),
            mode: WhatAmI::Client,
            listen: vec![],
            connect: vec!["tcp/127.0.0.1:17447".into()],
            task: vec![Task {
                topic: "myTestTopic".into(),
                action: Action::Sub,
            }],
        },
    ];

    let futures = nodes.into_iter().map(|node| async move {
        dbg!(node.name);
        let mut config = Config::default();
        config.set_mode(Some(node.mode)).unwrap();
        config.scouting.multicast.set_enabled(Some(false)).unwrap();
        config
            .listen
            .set_endpoints(node.listen.iter().map(|x| x.parse().unwrap()).collect())
            .unwrap();
        config
            .connect
            .set_endpoints(node.connect.iter().map(|x| x.parse().unwrap()).collect())
            .unwrap();
        let session = Arc::new(ztimeout!(zenoh::open(config).res_async()).unwrap());
        async_std::task::sleep(SLEEP).await;

        let futs = node.task.into_iter().map(|Task { topic, action }| {
            let c_session = session.clone();
            match action {
                Action::Sub => {
                    dbg!("Subscription passed.");
                    async_std::task::spawn(async move {
                        let sub =
                            ztimeout!(c_session.declare_subscriber(&topic).res_async()).unwrap();

                        while let Ok(sample) = sub.recv_async().await {
                            println!("Received : {:?}", sample);
                        }
                        dbg!("here...");
                    })
                }
                Action::Pub => {
                    dbg!("Publishment passed.");
                    async_std::task::spawn(async move {
                        loop {
                            // dbg!("looping...");
                            async_std::task::sleep(Duration::from_millis(300)).await;
                            ztimeout!(c_session
                                .put(&topic, vec![0u8; 8])
                                .congestion_control(CongestionControl::Block)
                                .res_async())
                            .unwrap();
                        }
                    })
                }
                Action::Park => {

                }
            }
        });
        join_all(futs).await
    });
    join_all(futures).await;
    dbg!("Done");
}
