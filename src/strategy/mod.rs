mod buy;
pub(crate) mod sell;

use crate::constant::WEEK;
use crate::kline::volume_too_few;
use crate::strategy::sell::a_strong_signal::ASS;
use crate::strategy::sell::two_middle_signal::TMS;
use crate::strategy::sell::{SellReason, SellStrategy};
use crate::{
    get_average_info, get_huge_volume_bar_num, get_last_bar_shape_score, get_last_bar_volume_score,
    get_raise_bar_num, notify_lark, recent_kline_shape_score, strategy, take_order, timestamp2date,
    Kline, MathOperation, MathOperation2, Pair, Symbol, TakeOrderInfo, TakeType,
    INCREASE_PRICE_LEVEL2, INCREASE_VOLUME_LEVEL2, KLINE_NUM_FOR_FIND_SIGNAL,
};
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};
use crate::strategy::sell::a_very_strong_signal::AVSS;
use crate::strategy::sell::sequential_take_order::STO;
use crate::strategy::sell::three_continuous_signal::TCS;

pub struct OrderData {
    pair: String,
    sell_reason: SellReason,
    sell_time: u64,
    buy_time: u64,
    increase_ratio: f32,
}

//是否突破：分别和远期（1小时）和中期k线（30m）进行对比取低值
async fn is_break_through_market(market: &str, line_datas: &[Kline]) -> bool {
    assert_eq!(line_datas.len(), KLINE_NUM_FOR_FIND_SIGNAL);
    //选351个，后边再剔除量最大的
    let mut recent_klines = line_datas[0..=340].to_owned();
    let broken_klines = &line_datas[339..=358];
    assert_eq!(recent_klines.len(), 341);
    assert_eq!(broken_klines.len(), 20);
    let (recent_average_price, recent_average_volume) = get_average_info(&recent_klines[..]);
    //价格以当前high为准
    let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
        .high_price
        .to_f32();
    //最近2小时，交易量不能有大于准顶量1.5倍的
    for (index, bar) in line_datas[..358].iter().enumerate() {
        if index <= 340 && bar.volume.to_f32().div(10.0) > line_datas[358].volume.to_f32() {
            return false;
        } else if index > 340 && bar.volume.to_f32().div(5.0) > line_datas[358].volume.to_f32() {
            return false;
        }
    }

    //交易量要大部分bar都符合要求
    let mut recent_huge_volume_bars_num =
        get_huge_volume_bar_num(broken_klines, recent_average_volume, INCREASE_VOLUME_LEVEL2);

    let recent_price_increase_rate =
        (current_price - recent_average_price).div(recent_average_price);

    debug!(
        "judge_break_signal market {},recent_price_increase_rate {},recent_huge_volume_bars_num {}
    ",
        market, recent_price_increase_rate, recent_huge_volume_bars_num
    );
    debug!(
        "market {},start {} ,end {}: recent_price_increase_rate {},recent_huge_volume_bars_num {}",
        market,
        timestamp2date(line_datas[0].open_time),
        timestamp2date(line_datas[359].open_time),
        recent_price_increase_rate,
        recent_huge_volume_bars_num
    );
    if recent_price_increase_rate >= INCREASE_PRICE_LEVEL2 && recent_huge_volume_bars_num >= 4 {
        return true;
    }
    false
}

//配合buy放宽顶部信号条件
pub async fn sell(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    line_datas: &[Kline],
    pair: &Symbol,
    balance: f32,
    is_real_trading: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let pair_symbol = pair.symbol.as_str();
    let take_type = TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::SequentialTakeOrder,
    };
    let take_info = take_order_pair.get(&take_type);

    if take_info.is_none() && !is_break_through_market(pair_symbol, &line_datas).await {
        debug!("Have no obvious break signal");
        return Ok(false);
    }
    if take_info.is_some() {
        for (index, bar) in line_datas[180..358].iter().enumerate() {
            if index <= 340 && bar.is_raise() && bar.volume.to_f32().div(3.0) > line_datas[358].volume.to_f32() {
                return Ok(false);
            } else if index > 340 && bar.is_raise() && bar.volume.to_f32().div(2.0) > line_datas[358].volume.to_f32() {
                return Ok(false);
            }
        }
    }

    //以倒数第二根的open，作为信号发现价格，以倒数第一根的open为实际下单价格
    let price = line_datas[359].open_price.parse::<f32>().unwrap();
    let taker_amount = match take_info {
        None => {
            balance
                .mul(20.0)
                .div(20.0)
                .div(price)
                .to_fix(pair.quantity_precision as u32)
        }
        Some(data) => { data.last().unwrap().amount }
    };
    //todo: 将其中通用的计算逻辑拿出来
    //ASS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    //TMS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    //TCS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    //AVSS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    STO::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;

    Ok(true)
}

//(Pair,SellReason)
//不处理返回值对：多次确认的逻辑没有影响，对单次确认的来说，有可能造成短期多次下单，单这个也是没毛病的
/***
pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    taker_type: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair.get(&taker_type) {
        None => {}
        Some(take_infos) => {
            let take_info = take_infos.last().unwrap();
            if take_info.is_took == true {
                let price_raise_ratio = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32()
                    / take_info.price;

                let interval_from_take =
                    line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1].open_time - take_info.take_time;
                //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
                let (can_buy, buy_reason) = if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].open_time <= take_info.take_time + 1000 * 60 * 3 //顶后三根
                    && line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].volume.to_f32() <= take_info.top_bar.volume.to_f32().div(6.0)
                {
                    (true, "too few volume in last 3 bars")
                    //} else if volume_too_few(&line_datas[350..],take_info.top_bar.volume.to_f32())
                    //{
                    //    (true,"last 10 bars volume too few")
                } else if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time
                    > take_info.take_time
                    && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
                {
                    (
                        true,
                        "Positive income and held it for two hour，and price start increase",
                    )
                } else {
                    (false, "")
                };
                if can_buy {
                    //和多久之前的比较，比较多少根？
                    let sell_reason_str:&str = taker_type.clone().sell_reason.into();
                    let push_text = format!(
                        "strategy2: buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {},price_raise_ratio {}",
                        buy_reason, sell_reason_str, taker_type.pair, price_raise_ratio);
                    //fixme: 这里remove会报错
                    //take_order_pair2.remove(pair_symbol);
                    if is_real_trading {
                        take_order(
                            taker_type.pair.clone(),
                            take_info.amount,
                            "BUY".to_string(),
                        )
                        .await;
                        notify_lark(push_text.clone()).await?;
                    }
                    take_order_pair.remove(&taker_type);
                    warn!("now {} , {}", timestamp2date(now), push_text);
                    return Ok((true, 1.0 - price_raise_ratio));
                } else {
                    return Ok((true, 0.0));
                }
            } else {
                //加入观察列表五分钟内不在观察，2小时内仍没有二次拉起的则将其移除观察列表
                if now.sub(take_info.take_time) > 4 * 60 * 60 * 1000 {
                    take_order_pair.remove(&taker_type);
                }
            }
        }
    }
    Ok((false, 0.0))
}
***/


pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    taker_type: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair.get(&taker_type) {
        None => {}
        Some(take_infos) => {
            let last_take_info = take_infos.last().unwrap();
            //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
            let (can_buy, buy_reason)= if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time
                > last_take_info.take_time
                && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
            {
                (
                    true,
                    "Positive income and held it for two hour，and price start increase",
                )
            } else {
                (false, "")
            };
            if can_buy {
                //和多久之前的比较，比较多少根？
                let sell_reason_str: &str = taker_type.clone().sell_reason.into();
                let push_text = format!(
                    "strategy2: buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {}",
                    buy_reason, sell_reason_str, taker_type.pair);
                //fixme: 这里remove会报错
                //take_order_pair2.remove(pair_symbol);
                if is_real_trading {
                    take_order(
                        taker_type.pair.clone(),
                        last_take_info.amount * take_infos.len() as f32,
                        "BUY".to_string(),
                    )
                        .await;
                    notify_lark(push_text.clone()).await?;
                }
                //计算总的收入
                let mut batch_profit = 0.0f32;
                let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32();
                let mut detail_profits = Vec::new();
                for take_info in take_infos{
                    let price_raise_ratio = current_price / take_info.price;
                    let iterm_profit = 1.0 - price_raise_ratio - 0.0008;
                    batch_profit += iterm_profit;
                    detail_profits.push((iterm_profit,timestamp2date(take_info.take_time)));
                }
                info!("data0001: now {} market {},total_profit {},detail {:?}",timestamp2date(now),taker_type.pair,batch_profit,detail_profits);
                take_order_pair.remove(&taker_type);
                //warn!("now {} , {}", timestamp2date(now), push_text);
                return Ok((true, batch_profit));
            } else {
                return Ok((true, 0.0));
            }
        }
    }
    Ok((false, 0.0))
}