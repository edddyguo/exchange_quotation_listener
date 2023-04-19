mod buy;
pub(crate) mod sell;

use std::borrow::Borrow;
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
use std::ops::{Deref, Div, Mul, Sub};
use rust_decimal::prelude::ToPrimitive;
use crate::strategy::sell::a_strong_signal_v2::ASS_V2;
use crate::strategy::sell::a_very_strong_signal::AVSS;
use crate::strategy::sell::a_very_strong_signal_v2::AVSS_V2;
use crate::strategy::sell::sequential_take_order::STO;
use crate::strategy::sell::start_go_down::SGD;
use crate::strategy::sell::three_continuous_signal::TCS;
use crate::strategy::sell::two_middle_signal_v2::TMS_V2;

pub struct OrderData {
    pair: String,
    sell_reason: SellReason,
    sell_time: u64,
    buy_time: u64,
    increase_ratio: f32,
}
/*macro_rules! signal_exist {
	($t:expr) => {
        take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: $t,
        }).is_some()
    }
}*/

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
        if index <= 340 && bar.volume.to_f32().div(5.0) > line_datas[358].volume.to_f32() {
            return false;
        } else if index > 340 && bar.volume.to_f32().div(2.0) > line_datas[358].volume.to_f32() {
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
    let avss_exist = take_order_pair.get(&TakeType {
        pair: pair_symbol.to_string(),
        sell_reason: SellReason::AVeryStrongSignal,
    }).and_then(|x| Some(x.last().unwrap().is_took));

    let is_break = is_break_through_market(pair_symbol, &line_datas).await;

    //以倒数第二根的open，作为信号发现价格，以倒数第一根的open为实际下单价格
    let price = line_datas[359].open_price.parse::<f32>().unwrap();
    let taker_amount = balance
        .mul(20.0)
        .div(100.0)
        .div(price)
        .to_fix(pair.quantity_precision as u32);

    if avss_exist.is_none() && is_break
        || avss_exist.is_some() && avss_exist.unwrap() == false
    {
        AVSS::condition_passed(take_order_pair, line_datas, pair, taker_amount, price, is_real_trading).await?;
    }

    Ok(true)
}

fn get_down_up_price_ratio(top_bar: &Kline,line_datas: &[Kline]) -> (f32,f32){
    let last_bar = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].to_owned();
    let symmetry_up_bar_index = KLINE_NUM_FOR_FIND_SIGNAL - 2 * (last_bar.kline_close_time - top_bar.kline_close_time).div(60000) as usize;
    let symmetry_up_bar = line_datas[symmetry_up_bar_index].to_owned();

    let down_ratio = (last_bar.close_price.to_f32() - top_bar.close_price.to_f32())
            .div((last_bar.kline_close_time - top_bar.kline_close_time).div(60 * 1000) as f32);
    let up_ratio = (top_bar.close_price.to_f32() - symmetry_up_bar.close_price.to_f32())
        .div((top_bar.kline_close_time - symmetry_up_bar.kline_close_time).div(60 * 1000) as f32);

    (up_ratio,-down_ratio)
}
//(Pair,SellReason)
//不处理返回值对：多次确认的逻辑没有影响，对单次确认的来说，有可能造成短期多次下单，单这个也是没毛病的
pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    taker_type: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    let hold_four_hour_reason = "Positive income and held it for four hour，and price start increase".to_string();
    let current_price = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
        .open_price
        .to_f32();
    let sell_reason = taker_type.clone().sell_reason;
    let sell_reason_str:&str = sell_reason.into();
    match take_order_pair.get_mut(&taker_type) {
        None => {}
        Some(take_infos) => {
            let total_history_raise = take_infos
                .iter()
                .map(|x|
                    match x.buy_price {
                        None => {0.0}
                        Some(buy_price) => {buy_price.sub(x.sell_price)}
                    }
                )
                .sum::<f32>();
            let take_info = take_infos.last_mut().unwrap();
            if take_info.is_took == true {
                /***
                三种情况平仓
                    1、大于下单价格的，
                    2，2小时之内价格的下降坡度小于top之前2小时的爬升坡度的三分之一（每次都对比）
                    3、过了4小时之后，最近30根15根上扬的
                平仓的时候只是设置is_took的状态为false，不remove，这样就可以等待后续重新下单的机会
                */
                let (can_buy, buy_reason) = if current_price > take_info.sell_price
                {
                    (true, "current price more than open price")
                } else if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time < take_info.take_time
                {
                    let (up_ratio,down_ratio) = get_down_up_price_ratio(&take_info.top_bar,line_datas);
                    warn!("down_ratio({}), up ratio({})",down_ratio,up_ratio);
                    if down_ratio < up_ratio.div(3.0) {
                        (true, "down_ratio below than 1/3 of up ratio")
                    }else {
                        (false, "")
                    }
                }  else if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 240].open_time > take_info.take_time
                    && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10{
                    (true, hold_four_hour_reason.as_str())
                } else {
                    (false, "")
                };
                if can_buy {

                    if is_real_trading {
                        take_order(
                            taker_type.pair.clone(),
                            take_info.amount,
                            "BUY".to_string(),
                        )
                        .await;
                    }
                    //只在最后remove的时候才进行总盈利统计
                    if buy_reason == hold_four_hour_reason {
                        let raise_ratio = (current_price - take_info.sell_price + total_history_raise).div(take_info.sell_price);
                        let push_text = format!(
                            "strategy, buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {},price_raise_ratio {}",
                            buy_reason, sell_reason_str, taker_type.pair, raise_ratio);
                        if is_real_trading{
                            notify_lark(push_text.clone()).await?;
                        }
                        //历史的平仓统计，本次的单独计算
                        take_info.buy_price = Some(current_price);
                        info!("data0001: now {} market {},detail {:?},sell_info {:?}",timestamp2date(now),taker_type.pair,push_text,take_infos);
                        take_order_pair.remove(&taker_type);
                        return Ok((true, -raise_ratio));
                    }else {
                        let raise_ratio = (current_price - take_info.sell_price).div(take_info.sell_price);
                        let push_text = format!(
                            "strategy, buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {},price_raise_ratio {}",
                            buy_reason, sell_reason_str, taker_type.pair, raise_ratio);
                        take_info.buy_price = Some(current_price);
                        take_info.is_took = false;
                        info!("data0002: now {} market {},detail {:?},sell_info {:?}",timestamp2date(now),taker_type.pair,push_text,take_infos);
                        return Ok((true, 0.0));
                    }

                } else {
                    return Ok((true, 0.0));
                }
            } else {
                //加入观察列表五分钟内不在观察，2小时内仍没有二次拉起的则将其移除观察列表
                if now.sub(take_info.take_time) > 4 * 60 * 60 * 1000 {
                    let raise_ratio = total_history_raise.div(take_info.sell_price);
                    let push_text = format!(
                        "strategy,not found new chance,sell_reason <<{}>>:: take_buy_order: market {},price_raise_ratio {}",
                         sell_reason_str, taker_type.pair, raise_ratio);
                    if is_real_trading {
                        notify_lark(push_text.clone()).await?;
                    }
                    info!("data0001: now {} market {},detail {:?},sell_info {:?}",timestamp2date(now),taker_type.pair,push_text,take_infos);
                    take_order_pair.remove(&taker_type);
                    return Ok((true, -raise_ratio));
                }
            }
        }
    }
    Ok((false, 0.0))
}
