#![feature(slice_take)]
extern crate core;
#[macro_use]
extern crate log;

mod account;
mod bar;
mod constant;
mod ex_info;
mod filters;
mod kline;
mod order;
mod utils;
use crate::account::get_usdt_balance;
use crate::bar::{get_last_bar_shape_score, get_last_bar_volume_score};
use crate::constant::{
    BROKEN_UP_INTERVALS, INCREASE_PRICE_LEVEL1, INCREASE_PRICE_LEVEL2, INCREASE_VOLUME_LEVEL1,
    INCREASE_VOLUME_LEVEL2, KLINE_NUM_FOR_FIND_SIGNAL,
};
use crate::ex_info::{list_all_pair, Symbol};
use crate::filters::Root;
use crate::kline::{get_current_price, recent_kline_shape_score};
use crate::order::take_order;
use crate::utils::{get_unix_timestamp_ms, MathOperation, MathOperation2};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Div, Mul};
use log::{debug, error, log_enabled, info, Level};


//15分钟粒度，价格上涨百分之1，量上涨10倍（暂时5倍）可以触发预警
//监控所有开了永续合约的交易对

#[derive(Debug, Serialize, Deserialize)]
struct AvgPrice {
    mins: u32,
    price: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Text {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Msg {
    msg_type: String,
    content: Text,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Kline {
    open_time: u64,
    open_price: String,
    high_price: String,
    low_price: String,
    close_price: String,
    volume: String,
    kline_close_time: u64,
    quote_asset_volume: String,
    number_of_trades: u32,
    taker_buy_base_asset_volume: String,
    taker_buy_quote_asset_volume: String,
    unused_field: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RateLimits {
    rateLimitType: String,
    interval: String,
    intervalNum: u8,
    limit: u32,
}
//todo: 不只是kline，用泛型弄
async fn try_get(kline_url: String) -> Vec<Kline> {
    let mut line_data;
    loop {
        match reqwest::get(&kline_url).await {
            Ok(res) => {
                //println!("url {},res {:?}", kline_url,res);
                let res_str = format!("{:?}", res);
                match res.json::<Vec<Kline>>().await {
                    Ok(data) => {
                        line_data = data;
                        break;
                    }
                    Err(error) => {
                        //println!("reqwest res string: {:?}",res_str);
                        warn!(
                            "res deserialize happened error {},and raw res {}",
                            error.to_string(),
                            res_str
                        );
                    }
                }
            }
            Err(error) => {
                warn!("reqwest get happened error {}", error.to_string());
            }
        }
    }
    line_data
}

//判断是否是突破形态，根据30分钟k线是否巨量
async fn is_break_through_market(market: &str) -> bool {
    let kline_url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}m&limit={}",
        market, BROKEN_UP_INTERVALS, KLINE_NUM_FOR_FIND_SIGNAL
    );
    let line_data = try_get(kline_url).await;
    println!(
        "test1 start {} - end {}",
        line_data[0].open_time, line_data[34].open_time
    );
    //当前是20-10-5的分布：20个作为平常参考，9个作为过度，5个作为突破信号判断
    //方案2：80-30-10
    //选81个，后边再剔除量最大的
    let mut recent_klines = line_data[0..=79+1].to_owned();
    //9..=13
    let broken_klines = &line_data[109..=118];
    assert_eq!(recent_klines.len(), 80);
    assert_eq!(broken_klines.len(), 10);
    //println!("recent_klines volume opentime {},start {},end {}",recent_klines[0].open_time,recent_klines[0].volume,recent_klines[19].volume);
    //println!("broken_klines volume opentime {},start {},end{}",broken_klines[0].open_time,broken_klines[0].volume,broken_klines[4].volume);
    recent_klines.sort_by(|a,b| a.volume.to_f32().partial_cmp(&b.volume.to_f32()).unwrap());
    let recent_klines = &recent_klines[0..=79];
    let recent_lines_num = recent_klines.len();

    let recent_volume = recent_klines
        .iter()
        .map(|x| x.volume.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(recent_lines_num as f32);

    let recent_price = recent_klines
        .iter()
        .map(|x| x.close_price.parse::<f32>().unwrap())
        .sum::<f32>()
        .div(recent_lines_num as f32);

    //价格以当前的计算就行
    let current_price = line_data[KLINE_NUM_FOR_FIND_SIGNAL - 1].high_price.to_f32();
    let increase_price = (current_price - recent_price).div(recent_price);

    println!("market {} increase_price {}", market, increase_price);
    if increase_price < INCREASE_PRICE_LEVEL2 {
        return false;
    }
    //交易量要大部分bar都符合要求
    let mut huge_volume_bars_num = 0;
    for broken_kline in broken_klines {
        let increase_volume = (broken_kline.volume.to_f32() - recent_volume).div(recent_volume);
        //println!("market {} increase_volume {}", market,increase_volume);
        if increase_volume > INCREASE_VOLUME_LEVEL2 {
            huge_volume_bars_num += 1;
        }
    }

    //暂时最近五个三个巨量就行
    //暂时最近十个六个巨量就行
    if huge_volume_bars_num <= 5 {
        return false;
    }
    true
}

//推送消息给lark机器人
async fn notify_lark(pushed_msg: String) -> Result<(), Box<dyn std::error::Error>> {
    //println!("increase_ratio {},increase_volume {}",increase_price,increase_volume);
    let data = Msg {
        msg_type: "text".to_string(),
        content: Text { text: pushed_msg },
    };
    let client = reqwest::Client::new();
    let res = client
        .post(
            //"https://open.larksuite.com/open-apis/bot/v2/hook/83874fa0-1316-4cc2-8e88-7f8fd9d5d5e9",
            "https://open.larksuite.com/open-apis/bot/v2/hook/38692ffa-9b47-4289-b254-cc4cfc5df048",
        )
        .json(&data)
        .header("Content-type", "application/json")
        .header("charset", "utf-8")
        .send()
        .await?;
    //send to lark
    println!("{:#?}", res.status());
    Ok(())
}

//判断是否五连阳
async fn is_many_increase_times(market: &str, limit: u8) -> bool {
    let kline_url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit={}",
        market, limit
    );
    let line_datas = try_get(kline_url).await;
    //1分钟k线中拥有五连阳的
    for (index, line_data) in line_datas.iter().enumerate() {
        if (index > 0 && line_data.close_price <= line_datas[index - 1].close_price)
            || line_data.close_price <= line_data.open_price
        {
            return false;
        }
    }
    true
}

//binance-doc: https://binance-docs.github.io/apidocs/spot/en/#public-api-definitions
//策略：1h的k线，涨幅百分之1，量增加2倍
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //https://api.binance.com/api/v3/avgPrice?symbol=BNBUSDT
    //let markets = get_all_market().await;
    //let markets = PERP_MARKET;
    env_logger::init();
    let all_pairs = list_all_pair().await;
    //symbol -> order time,price,amount
    let mut take_order_pair: HashMap<String, (i64, f32, f32)> = HashMap::new();
    loop {
        for (index, pair) in all_pairs.clone().into_iter().enumerate() {
            //20分钟内不允许再次下单
            //todo：增加止损的逻辑,20点就止损
            //todo: 目前人工维护已下单数据，后期考虑链上获取
            let current_price = get_current_price(pair.symbol.as_str()).await;
            match take_order_pair.get(pair.symbol.as_str()) {
                None => {}
                Some(take_info) => {
                    let price_raise_ratio = current_price / take_info.1;
                    //20X情况下：30个点止损，60个点止盈
                    if price_raise_ratio > 1.015 || price_raise_ratio < 0.97{
                        //taker_order
                        take_order(pair.symbol.clone(), take_info.2, "BUY".to_string()).await;
                        take_order_pair.remove(pair.symbol.as_str());
                        let push_text = format!("止损止盈平空单: market {},price_raise_ratio {}", pair.symbol,price_raise_ratio);
                        notify_lark(push_text).await?;
                    } else if get_unix_timestamp_ms() - take_info.0 < 1200000 {
                        continue;
                    } else {
                    }
                }
            }
            let market = pair.symbol.as_str();
            println!("index {},market {}", index, market);
            if is_break_through_market(market).await {
                println!("Found break signal");
                let kline_url = format!(
                    "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit=20",
                    market
                );
                let line_datas = try_get(kline_url).await;
                let shape_score = get_last_bar_shape_score(line_datas.clone());
                let volume_score = get_last_bar_volume_score(line_datas.clone());
                //8-17。多一个作为价格比较的基准
                let recent_shape_score = recent_kline_shape_score(line_datas[7..=17].to_vec());

                //总分分别是：7分，5分，10分
                if shape_score >= 4 && volume_score >= 3 && recent_shape_score >= 7 {
                    let balance = get_usdt_balance().await;
                    let price = line_datas
                        .last()
                        .unwrap()
                        .close_price
                        .parse::<f32>()
                        .unwrap();
                    //default lever ratio is 20x,每次1成仓位20倍
                    let taker_amount = balance
                        .mul(20.0)
                        .div(10.0)
                        .div(price)
                        .to_fix(pair.quantity_precision as u32);
                    take_order(market.to_string(), taker_amount, "SELL".to_string()).await;
                    take_order_pair.insert(
                        market.to_string(),
                        (get_unix_timestamp_ms(), price, taker_amount),
                    );
                    let push_text = format!("开空单: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                            market,shape_score,volume_score,recent_shape_score,taker_amount
                    );
                    notify_lark(push_text).await?
                } else {
                    info!("Have no take order signal,\
                     below is detail score:market {},shape_score {},volume_score {},recent_shape_score {}",
                              market,shape_score,volume_score,recent_shape_score
                     );
                }
            } else {
                info!("Have no obvious break signal");
            }
        }
        info!("complete listen all pairs");
        //保证1分钟只下单一次
        std::thread::sleep(std::time::Duration::from_secs_f32(50.0));
    }
    Ok(())
}
