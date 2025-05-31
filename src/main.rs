use std::net::SocketAddr;

use bluest::{Adapter, AdvertisingDevice};
use futures_lite::stream::StreamExt;
use tokio::sync::watch::{self, Receiver, Sender};
use warp::Filter;

#[tokio::main]
async fn main() {
    let (tx, rx) = watch::channel(0);
    tokio::join!(ble_scanner(tx), web_server(rx));
}

async fn web_server(rx: Receiver<i32>) {
    let root = warp::path::end().map(|| warp::reply::html(include_str!("../web/index.html")));
    let heartrate = warp::path!("heartrate").then(move || {
        let mut rx = rx.clone();
        async move {
            drop(rx.borrow_and_update());
            rx.changed().await.unwrap();
            warp::reply::json(&*rx.borrow())
        }
    });

    let socket_addr: SocketAddr = ([127, 0, 0, 1], 3030).into();
    println!("开始监听于 http://{socket_addr:?}");

    warp::serve(warp::get().and(root).or(heartrate))
        .run(socket_addr)
        .await
}

async fn ble_scanner(tx: Sender<i32>) {
    let adapter = Adapter::default()
        .await
        .ok_or("未找到蓝牙适配器")
        .unwrap();
    adapter.wait_available().await.unwrap();

    println!("开始搜索小米手环");
    let mut scan = adapter.scan(&[]).await.unwrap();

    while let Some(discovered_device) = scan.next().await {
        if let Some(heart_rate) = handle_device(discovered_device) {
            tx.send(heart_rate).unwrap();
        }
    }
}

fn handle_device(discovered_device: AdvertisingDevice) -> Option<i32> {
    let manufacturer_data = discovered_device.adv_data.manufacturer_data?;
    if manufacturer_data.company_id != 0x0157 {
        return None;
    }
    let mut name = discovered_device
        .device
        .name()
        .unwrap_or(String::from("(未知)"));
    if name != "Mi Smart Band 4" {
        return None;
    }
    else {
        name = "小米手环 4".to_string();
    }
    // let id = discovered_device.device.id();
    let rssi = discovered_device.rssi.unwrap_or_default();
    let heart_rate = manufacturer_data.data[3];
    let heart_rate = match heart_rate {
        0xFF => None,
        x => Some(x.into()),
    };
    println!("{name} ({rssi}dBm) 心率: {heart_rate:?}",);
    heart_rate
}
