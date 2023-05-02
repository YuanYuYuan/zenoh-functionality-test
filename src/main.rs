#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use anyhow::bail;
use futures::future::{join_all, select_all};
use futures::{
    select,
    FutureExt as _,
};
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use async_std::task;
use async_std::prelude::FutureExt;
use zenoh::config::{whatami::WhatAmI, Config, EndPoint};
use zenoh::prelude::r#async::*;
use zenoh::Result;

const TIMEOUT: Duration = Duration::from_secs(10);
const SLEEP: Duration = Duration::from_secs(1);

macro_rules! ztimeout {
    ($f:expr) => {
        $f.timeout(TIMEOUT).await?
    };
}

#[derive(Debug)]
enum Task {
    Pub(String, usize),
    Sub(String, usize),
    Sleep,
}

#[derive(Debug)]
struct Node {
    name: String,
    mode: WhatAmI,
    listen: Vec<String>,
    connect: Vec<String>,
    task: Vec<Task>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            name: "TestNode".into(),
            mode: WhatAmI::Peer,
            listen: vec![],
            connect: vec![],
            task: vec![],
        }
    }
}


#[async_std::main]
async fn main() -> Result<()> {
    let nodes = vec![
        Node {
            name: "C:Router".into(),
            mode: WhatAmI::Router,
            listen: vec!["tcp/127.0.0.1:17447".into()],
            task: vec![Task::Sleep],
            ..Default::default()
        },
        Node {
            name: "A:PubClient".into(),
            connect: vec!["tcp/127.0.0.1:17447".into()],
            mode: WhatAmI::Client,
            task: vec![Task::Pub("myTestTopic".into(), 8)],
            ..Default::default()
        },
        Node {
            name: "B:SubClient".into(),
            mode: WhatAmI::Client,
            connect: vec!["tcp/127.0.0.1:17447".into()],
            task: vec![Task::Sub("myTestTopic".into(), 8)],
            ..Default::default()
        },
    ];

    // let terminated = Arc::new(AtomicBool::new(false));

    let futures = nodes.into_iter().map(|node| async move {
        dbg!(node.name);

        // Load the config and build up a session
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
        let session = Arc::new(ztimeout!(zenoh::open(config).res_async())?);

        async_std::task::sleep(SLEEP).await;

        let futs = node.task.into_iter().map(|task| {
            let c_session = session.clone();
            let fut = match task {
                Task::Sub(topic, payload_size) => {
                    dbg!("Subscription passed.");
                    async_std::task::spawn(async move {
                        let sub =
                            ztimeout!(c_session.declare_subscriber(&topic).res_async())?;

                        let mut counter = 0;
                        while let Ok(sample) = sub.recv_async().await {
                            assert_eq!(sample.value.payload.len(), payload_size);
                            counter += 1;
                            if counter >= 5 {
                                // terminated.store(true, Ordering::Relaxed);
                                break;
                            }
                            println!("Received : {:?}", sample);
                        }
                        println!("Terminated");
                        Result::Ok(())
                    })
                }
                Task::Pub(topic, payload_size) => {
                    dbg!("Publishment passed.");
                    async_std::task::spawn(async move {
                        // while terminated.into_inner() {
                        loop {
                            // dbg!("looping...");
                            async_std::task::sleep(Duration::from_millis(300)).await;
                            ztimeout!(c_session
                                .put(&topic, vec![0u8; payload_size])
                                .congestion_control(CongestionControl::Block)
                                .res_async())?;
                        }
                        Ok(())
                    })
                }
                Task::Sleep => async_std::task::spawn(async move {
                    async_std::task::sleep(Duration::from_secs(30)).await;
                    Ok(())
                }),
            };
            fut
        });

        // futs
        // join_all(futs).await;
        // let (x, y, z) = select_all(futs).boxed();
        let (state, _, _) = select_all(futs).await;
        state
    }.boxed());

    let (state, _, _) = select_all(futures).await;
    if let Ok(state) = state {
        println!("Done");
    } else {
        println!("Failed");
    }
    Ok(())
}
