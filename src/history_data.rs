use std::collections::HashMap;
use std::fmt::format;
use std::fs::File;
use std::io;
use std::io::{Bytes, Cursor, Read};
use reqwest::Response;
use crate::{Kline, list_all_pair, Symbol};
use rayon::prelude::*;

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

pub async fn download_history_data(){
    let dir = format!("./history_kline",);
    let all_pairs = list_all_pair().await;
    let url = "https://data.binance.vision/data/spot/monthly/klines".to_string();
    for pair in all_pairs {
        for month in 1..=12 {
            let file_name = format!("{}/{}-1m-2022-{:0>2}.zip",dir,pair.symbol,month);
            let url = format!("{}/{}/1m/{}-1m-2022-{:0>2}.zip",url,pair.symbol,pair.symbol,month);
            println!("start download {}",url);
            let response = try_get(url.clone()).await;
            let mut file = std::fs::File::create(file_name.clone()).unwrap();
            let mut content =  Cursor::new(response.bytes().await.unwrap());
            std::io::copy(&mut content, &mut file).unwrap();
            println!("finished download {}",url);
            std::thread::sleep(std::time::Duration::from_secs_f32(0.2));
        }
    }
}
//by month time to back testing
pub async fn load_history_data(month: u8){
    let dir = "./history_kline";
    let all_pairs = list_all_pair().await;
    let datas: HashMap<Symbol,Vec<Kline>> = HashMap::new();
    for pair in all_pairs {
        let file_name = format!("{}/{}-1m-2022-{:0>2}.csv",dir,pair.symbol,month);
        let mut rdr =
            csv::ReaderBuilder::new().has_headers(false).from_path(file_name).unwrap();

        for result in rdr.deserialize() {
            let record: Kline = result.unwrap();
            println!("--{:?}", record);
        }
    }
}

