//
// Copyright (c) 2023 ZettaScale Technology
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ZettaScale Zenoh Team, <zenoh@zettascale.tech>
//
use async_std::prelude::FutureExt;
use async_std::task::sleep;
use clap::{App, Arg};
use futures::prelude::*;
use futures::select;
use std::convert::TryFrom;
use std::str::FromStr;
use std::time::Duration;
use zenoh::config::{whatami::WhatAmIMatcher, Config, Locator, ModeDependentValue};
use zenoh::prelude::r#async::*;
use zenoh::runtime::Runtime;
use zenoh::scouting::WhatAmI;

#[async_std::main]
async fn main() {
    // Initiate logging
    env_logger::init();

    let (mut config, key_expr) = parse_args();
    config
        .scouting
        .multicast
        .set_autoconnect(Some(ModeDependentValue::Unique(WhatAmIMatcher::empty())))
        .unwrap();
    config
        .scouting
        .gossip
        .set_autoconnect(Some(ModeDependentValue::Unique(WhatAmIMatcher::empty())))
        .unwrap();

    println!("Opening session...");
    let runtime = Runtime::new(config).await.unwrap();
    let session = zenoh::init(runtime.clone()).res().await.unwrap();

    println!("Declaring Subscriber on '{}'...", &key_expr);
    let subscriber = session.declare_subscriber(&key_expr).res().await.unwrap();

    println!("Options");
    println!("- 'put <keyexpr> <value>'");
    println!("- 'connect peers'");
    println!("- 'connect routers'");
    println!("- 'connect <locator>'");
    println!("- 'disconnect peers'");
    println!("- 'disconnect routers'");
    println!("- 'disconnect <zid>'");
    println!("- 'quit'");
    let stdin = async_std::io::stdin();
    let mut input = String::new();
    loop {
        print!("> ");
        async_std::io::stdout().flush().await.unwrap_or_default();
        select!(
            sample = subscriber.recv_async() => {
                let sample = sample.unwrap();
                println!(">> [Subscriber] Received {} ('{}': '{}')",
                    sample.kind, sample.key_expr.as_str(), sample.value);
            },

            _ = stdin.read_line(&mut input).fuse() => {
                input.pop();
                match &*input.split(' ').collect::<Vec<&str>>() {
                    ["put", key_expr, value] => {
                        if let Ok(key_expr) = KeyExpr::try_from(*key_expr) {
                            println!("Putting Data ('{key_expr}': '{value}')...");
                            session.put(key_expr, *value).res().await.unwrap();
                        }
                    }
                    ["connect", "peers"] => {
                        println!("Connect peers...");
                        let receiver = zenoh::scout(WhatAmI::Peer, Config::default())
                            .res()
                            .await
                            .unwrap();
                        let _ = async {
                            while let Ok(hello) = receiver.recv_async().await {
                                if hello.zid != Some(session.info().zid().res().await) {
                                    for locator in hello.locators {
                                        let endpoint = locator.into();
                                        if let Ok(Ok(transport)) = runtime
                                            .manager()
                                            .open_transport(endpoint)
                                            .timeout(Duration::from_secs(1))
                                            .await
                                        {
                                            println!("Connected scouted peer {}...", transport.get_zid().unwrap());
                                            break
                                        }
... (142 lines left)
