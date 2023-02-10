use crate::{list_all_pair, Kline, Symbol};
use rayon::prelude::*;
use reqwest::Response;
use std::collections::HashMap;
use std::fmt::format;
use std::fs::File;
use std::io;
use std::io::{Bytes, Cursor, Read};
use std::ops::Deref;
use std::sync::{Arc, RwLock};
use tokio::runtime::Runtime;

async fn try_get(kline_url: String) -> Response {
    let mut response;
    loop {
        match reqwest::get(&kline_url).await {
            Ok(res) => {
                //println!("url {},res {:?}", kline_url,res);
                response = res;
                break;
            }
            Err(error) => {
                warn!("reqwest get happened error {}", error.to_string());
            }
        }
        std::thread::sleep(std::time::Duration::from_secs_f32(1.0));
    }
    response
}

pub async fn download_history_data() {
    let dir = format!("./history_kline",);
    let all_pairs = list_all_pair().await;
    let url = "https://data.binance.vision/data/spot/monthly/klines".to_string();
    all_pairs.par_iter().for_each(|pair| {
        for month in 1..=1 {
            let file_name = format!("{}/{}-1m-2023-{:0>2}.zip", dir, pair.symbol, month);
            let url = format!(
                "{}/{}/1m/{}-1m-2023-{:0>2}.zip",
                url, pair.symbol, pair.symbol, month
            );
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                println!("start download {}", url);
                let response = try_get(url.clone()).await;
                let mut file = std::fs::File::create(file_name.clone()).unwrap();
                let mut content = Cursor::new(response.bytes().await.unwrap());
                std::io::copy(&mut content, &mut file).unwrap();
                println!("finished download {}", url);
                std::thread::sleep(std::time::Duration::from_secs_f32(0.2));
            });
        }
    })
}

//by month time to back testing
pub async fn load_history_data(month: u8) -> HashMap<Symbol, Vec<Kline>> {
    let dir = "./history_kline";
    let all_pairs = list_all_pair().await;
    let mut datas: HashMap<Symbol, Vec<Kline>> = HashMap::new();
    let arc_datas = Arc::new(RwLock::new(datas));

    all_pairs
        .par_iter()
        .filter(|x| x.symbol != "HNTUSDT")
        .for_each(|x| {
            let arc_datas = arc_datas.clone();
            let file_name = format!("{}/{}-1m-2023-{:0>2}.csv", dir, x.symbol, month);
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_path(file_name)
                .unwrap();
            let mut symbol_klines = Vec::new();
            for result in rdr.deserialize() {
                let record: Kline = result.unwrap();
                symbol_klines.push(record);
            }
            arc_datas
                .write()
                .unwrap()
                .insert(x.to_owned(), symbol_klines);
        });
    //println!("{:?}", arc_datas.read().unwrap());
    let data = arc_datas.read().unwrap().deref().to_owned();
    data
}
