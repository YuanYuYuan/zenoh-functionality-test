#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use futures::future::{join_all, select_all};
use futures::{select, FutureExt as _};
use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

use async_std::prelude::FutureExt;
use async_std::task;
use zenoh::config::{whatami::WhatAmI, Config, EndPoint};
use zenoh::prelude::r#async::*;
use zenoh::Result;
use zenoh_result::bail;
// use zenoh::Result;

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
    Sleep(Duration),
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

async fn run_recipe(receipe: impl IntoIterator<Item = Node>) -> Result<()> {
    let futures = receipe.into_iter().map(|node| async move {
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

        // In case of client can't connect to some peers/routers
        let session = if let Ok(session) = zenoh::open(config.clone()).res_async().await {
            Arc::new(session)
        } else {
            // Sleep one second and retry
            async_std::task::sleep(Duration::from_secs(1)).await;
            Arc::new(zenoh::open(config).res_async().await?)
        };

        // Each node consists of a specified session associated with tasks to run
        let futs = node.task.into_iter().map(|task| {

            // Each session spawns several given task(s)
            let c_session = session.clone();
            let fut = match task {

                // Subscription task
                Task::Sub(topic, expected_size) => {
                    async_std::task::spawn(async move {
                        let sub =
                            ztimeout!(c_session.declare_subscriber(&topic).res_async())?;

                        let mut counter = 0;
                        while let Ok(sample) = sub.recv_async().await {
                            let recv_size = sample.value.payload.len();
                            if recv_size != expected_size {
                                bail!("Received payload size {recv_size} mismatches the expected {expected_size}");
                            }
                            counter += 1;
                            println!("Received : {:?}", sample);
                            if counter >= 5 {
                                break;
                            }
                        }
                        Result::Ok(())
                    })
                }

                // Publishment task
                Task::Pub(topic, payload_size) => {
                    async_std::task::spawn(async move {
                        loop {
                            async_std::task::sleep(Duration::from_millis(300)).await;
                            ztimeout!(c_session
                                .put(&topic, vec![0u8; payload_size])
                                .congestion_control(CongestionControl::Block)
                                .res_async())?;
                        }
                    })
                }

                // Sleep a while according to the given duration
                Task::Sleep(dur) => async_std::task::spawn(async move {
                    async_std::task::sleep(dur).await;
                    Ok(())
                }),
            };
            fut
        });

        let (state, _, _) = select_all(futs).await;
        state
    }.boxed());

    let (state, _, _) = select_all(futures).timeout(TIMEOUT).await?;
    Ok(state?)
}

#[async_std::main]
async fn main() -> Result<()> {
    let locator = String::from("tcp/127.0.0.1:17447");
    let topic = String::from("testTopic");
    let recipe = [
        Node {
            name: "C:Router".into(),
            mode: WhatAmI::Router,
            listen: vec![locator.clone()],
            task: vec![Task::Sleep(Duration::from_secs(30))],
            ..Default::default()
        },
        Node {
            name: "A:PubClient".into(),
            connect: vec![locator.clone()],
            mode: WhatAmI::Client,
            task: vec![Task::Pub(topic.clone(), 8)],
            ..Default::default()
        },
        Node {
            name: "B:SubClient".into(),
            mode: WhatAmI::Client,
            connect: vec![locator.clone()],
            task: vec![Task::Sub(topic.clone(), 8)],
            ..Default::default()
        },
    ];

    run_recipe(recipe).await?;
    println!("Test passed.");
    Ok(())
}
