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
use crate::bar::{get_huge_volume_bar_num, get_last_bar_shape_score, get_last_bar_volume_score, get_raise_bar_num};
use crate::constant::{BROKEN_UP_INTERVALS, INCREASE_PRICE_LEVEL1, INCREASE_PRICE_LEVEL2, INCREASE_VOLUME_LEVEL1, INCREASE_VOLUME_LEVEL2, KLINE_NUM_FOR_FIND_SIGNAL};
use crate::ex_info::{list_all_pair, Symbol};
use crate::filters::Root;
use crate::kline::{get_average_info, get_current_price, recent_kline_shape_score};
use crate::order::take_order;
use crate::utils::{get_unix_timestamp_ms, MathOperation, MathOperation2};
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};
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

//symbol -> order time,price,amount
#[derive(Debug, Serialize)]
struct TakeOrderInfo {
    take_time: u64,
    //如果没下单，则以30分钟内尝试检测是否再次拉升
    price: f32,
    amount: f32,
    is_took: bool, //是否已经下单
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
        std::thread::sleep(std::time::Duration::from_secs_f32(1.0));
    }
    line_data
}

//是否突破：分别和远期（1小时）和中期k线（30m）进行对比取低值
async fn is_break_through_market(market: &str, line_datas: &[Kline]) -> bool {
    assert_eq!(line_datas.len(), KLINE_NUM_FOR_FIND_SIGNAL);
    //选351个，后边再剔除量最大的
    let mut recent_klines = line_datas[0..=350].to_owned();
    let broken_klines = &line_datas[349..=358];
    assert_eq!(recent_klines.len(), 351);
    assert_eq!(broken_klines.len(), 10);

    let (recent_average_price, recent_average_volume) = get_average_info(&recent_klines[..]);
    //价格以当前high为准
    let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1].high_price.to_f32();

    //交易量要大部分bar都符合要求
    let mut recent_huge_volume_bars_num = get_huge_volume_bar_num(broken_klines, recent_average_volume, INCREASE_VOLUME_LEVEL2);

    let recent_price_increase_rate = (current_price - recent_average_price).div(recent_average_price);

    info!("judge_break_signal market {},recent_price_increase_rate {},recent_huge_volume_bars_num {}
    ",market,recent_price_increase_rate,recent_huge_volume_bars_num);
    if recent_price_increase_rate >= INCREASE_PRICE_LEVEL2 && recent_huge_volume_bars_num >= 5
    {
        return true;
    }
    false
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
            "https://open.larksuite.com/open-apis/bot/v2/hook/f1011068-13f1-4258-a98d-3c65a0449bb0",
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


pub async fn excute_take_order_logic() {}


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
    //let mut take_order_pair: HashMap<String, (u64, f32, f32)> = HashMap::new();
    let mut take_order_pair2: HashMap<String, TakeOrderInfo> = HashMap::new();
    loop {
        for (index, pair) in all_pairs.clone().into_iter().enumerate() {
            let kline_url = format!(
                "https://api.binance.com/api/v3/klines?symbol={}&interval=1m&limit={}",
                pair.symbol.as_str(), KLINE_NUM_FOR_FIND_SIGNAL
            );
            let line_datas = try_get(kline_url).await;
            let now = get_unix_timestamp_ms() as u64;
            //todo: 目前人工维护已下单数据，后期考虑链上获取
            match take_order_pair2.get(pair.symbol.as_str()) {
                None => {}
                Some(take_info) => {
                    if take_info.is_took == true {
                        let price_raise_ratio = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1].close_price.to_f32() / take_info.price;
                        //20X情况下：0.4个点止损,高峰之后根据20根k线之后，价格是否大于10根之前的价格5次这种情况就止盈
                        if price_raise_ratio > 1.002
                            || (line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 20].open_time > take_info.take_time && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 20..]) >= 5) {
                            take_order(pair.symbol.clone(), take_info.amount, "BUY".to_string()).await;
                            take_order_pair2.remove(pair.symbol.as_str());
                            let push_text = format!("止损止盈平空单: market {},price_raise_ratio {}", pair.symbol, price_raise_ratio);
                            notify_lark(push_text).await?;
                            continue;
                        } else if now.sub(take_info.take_time) < 1200000 {
                            //20分钟内不允许再次下单
                            continue;
                        } else {}
                    } else {
                        //加入观察列表五分钟内不在观察，40分钟内仍没有二次拉起的则将其移除观察列表
                        if now.sub(take_info.take_time) < 300000 {
                            continue;
                        }else if now.sub(take_info.take_time) > 1200000 {
                            take_order_pair2.remove(pair.symbol.as_str());
                        }else {
                        }
                    }
                }
            }
            let market = pair.symbol.as_str();
            if is_break_through_market(market, &line_datas).await {
                info!("found break signal：index {},market {}", index, market);
                let line_datas = &line_datas[340..360];
                let shape_score = get_last_bar_shape_score(line_datas.to_owned());
                let volume_score = get_last_bar_volume_score(line_datas.to_owned());
                //8-17。多一个作为价格比较的基准
                let recent_shape_score = recent_kline_shape_score(line_datas[7..=17].to_vec());

                //总分分别是：7分，5分，10分
                //分为三种情况：强信号直接下单，弱信号加入观测名单，弱信号且已经在观查名单且距离观察名单超过五分钟的就下单，
                if take_order_pair2.get(market).is_none() && shape_score >= 4 && volume_score >= 3 && recent_shape_score >= 5 {
                    let balance = get_usdt_balance().await;
                    //以倒数第二根的open，作为标记price
                    let price = line_datas[18]
                        .open_price
                        .parse::<f32>()
                        .unwrap();

                    //default lever ratio is 20x,每次2成仓位20倍
                    let taker_amount = balance
                        .mul(20.0)
                        .div(10.0)
                        .div(price)
                        .to_fix(pair.quantity_precision as u32);
                    let mut push_text = "".to_string();
                    //强信号或者二次拉升
                    if (shape_score >= 5 && volume_score >= 5 && recent_shape_score >= 8)
                        || take_order_pair2.get(market).is_some()
                    {
                        take_order(market.to_string(), taker_amount, "SELL".to_string()).await;
                        let order_info = TakeOrderInfo {
                            take_time: get_unix_timestamp_ms() as u64,
                            price,
                            amount: taker_amount,
                            is_took: true,
                        };
                        take_order_pair2.insert(
                            market.to_string(),
                            order_info,
                        );
                        push_text = format!("开空单: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                            market, shape_score, volume_score, recent_shape_score, taker_amount
                        );
                    } else {
                        let order_info = TakeOrderInfo {
                            take_time: get_unix_timestamp_ms() as u64,
                            price,
                            amount: 0.0,//not care
                            is_took: false,
                        };
                        take_order_pair2.insert(
                            market.to_string(),
                            order_info,
                        );
                        push_text = format!("加入观察列表: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                            market, shape_score, volume_score, recent_shape_score, taker_amount
                        );
                    }
                    info!("Take order {}",push_text );
                    notify_lark(push_text).await?;
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
        //保证每次顶多一次下单、平仓
        std::thread::sleep(std::time::Duration::from_secs_f32(26.0));
    }
    Ok(())
}
