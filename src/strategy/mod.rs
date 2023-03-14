mod buy;
pub(crate) mod sell;

use crate::{get_last_bar_shape_score, get_last_bar_volume_score, get_raise_bar_num, notify_lark, recent_kline_shape_score, take_order, timestamp2date, Kline, MathOperation, MathOperation2, Symbol, TakeOrderInfo, KLINE_NUM_FOR_FIND_SIGNAL, get_average_info, get_huge_volume_bar_num, INCREASE_VOLUME_LEVEL2, INCREASE_PRICE_LEVEL2, Pair, strategy, TakeType};
use std::collections::HashMap;
use std::ops::{Div, Mul, Sub};
use crate::constant::WEEK;
use crate::kline::volume_too_few;
use crate::strategy::sell::a_strong_signal::ASS;
use crate::strategy::sell::{SellReason, SellStrategy};
use crate::strategy::sell::two_middle_signal::TMS;

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
    for (index,bar) in line_datas[..358].iter().enumerate() {
        if index <= 340 && bar.volume.to_f32().div(10.0) > line_datas[358].volume.to_f32() {
            return false;
        }else if index > 340 && bar.volume.to_f32().div(5.0) > line_datas[358].volume.to_f32() {
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
    debug!("market {},start {} ,end {}: recent_price_increase_rate {},recent_huge_volume_bars_num {}"
    ,market
    ,timestamp2date(line_datas[0].open_time)
    ,timestamp2date(line_datas[359].open_time)
    ,recent_price_increase_rate
    ,recent_huge_volume_bars_num);
    if recent_price_increase_rate >= INCREASE_PRICE_LEVEL2 && recent_huge_volume_bars_num >= 4 {
        return true;
    }
    false
}

//配合buy放宽顶部信号条件
pub async fn sell(
    take_order_pair2: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    line_datas: &[Kline],
    pair: &Symbol,
    balance: f32,
    is_real_trading: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    let pair_symbol = pair.symbol.as_str();
    let now = line_datas[359].open_time + 1000;
    if !is_break_through_market(pair_symbol,&line_datas).await {
        debug!("Have no obvious break signal");
        return Ok(false);
    }
    ASS::condition_passed(take_order_pair2,line_datas,pair,balance,is_real_trading).await?;
    //TMS::condition_passed(take_order_pair2,line_datas,pair,balance,is_real_trading)?;
    Ok(true)
}

//(Pair,SellReason)
//下单之后判断交易量，临近的三根必须大于五分之一，否则就大概率不是顶
pub async fn buy(
    take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
    pair_and_sell_reason: TakeType,
    line_datas: &[Kline],
    is_real_trading: bool,
) -> Result<(bool, f32), Box<dyn std::error::Error>> {
    let now = line_datas[359].open_time + 1000;
    match take_order_pair.get(&pair_and_sell_reason) {
        None => {}
        Some(take_infos) => {
            let take_info = take_infos.last().unwrap();
            if take_info.is_took == true {
                let price_raise_ratio = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1]
                    .open_price
                    .to_f32()
                    / take_info.price;

                let interval_from_take = line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 1].open_time - take_info.take_time;
                //三种情况平仓1、顶后三根有小于五分之一的，2，20根之后看情况止盈利
                let (can_buy, buy_reason) = if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].open_time <= take_info.take_time + 1000 * 60 * 3 //顶后三根
                    && line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 2].volume.to_f32() <= take_info.top_bar.volume.to_f32().div(6.0)
                {
                    (true,"too few volume in last 3 bars")
                    //} else if volume_too_few(&line_datas[350..],take_info.top_bar.volume.to_f32())
                    //{
                    //    (true,"last 10 bars volume too few")
                } else if line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 120].open_time > take_info.take_time
                    && get_raise_bar_num(&line_datas[KLINE_NUM_FOR_FIND_SIGNAL - 30..]) >= 10
                {
                    (true,"Positive income and held it for two hour，and price start increase")
                }else {
                    (false,"")
                };
                if can_buy {//和多久之前的比较，比较多少根？
                    let push_text = format!(
                        "strategy2: buy_reason <<{}>>,sell_reason <<{}>>:: take_buy_order: market {},price_raise_ratio {}",
                        buy_reason, pair_and_sell_reason.sell_reason.to_string(),pair_and_sell_reason.pair, price_raise_ratio);
                    //fixme: 这里remove会报错
                    //take_order_pair2.remove(pair_symbol);
                    if is_real_trading {
                        take_order(pair_and_sell_reason.pair.clone(), take_info.amount, "BUY".to_string())
                            .await;
                        notify_lark(push_text.clone()).await?;
                    }
                    take_order_pair.remove(&pair_and_sell_reason);
                    warn!("now {} , {}",timestamp2date(now),push_text);
                    return Ok((true, 1.0 - price_raise_ratio));
                } else {
                    return Ok((true, 0.0));
                }
            } else {
                //加入观察列表五分钟内不在观察，2小时内仍没有二次拉起的则将其移除观察列表
                if now.sub(take_info.take_time) > 4 * 60 * 60 * 1000 {
                    take_order_pair.remove(&pair_and_sell_reason);
                }
            }
        }
    }
    Ok((false, 0.0))
}
