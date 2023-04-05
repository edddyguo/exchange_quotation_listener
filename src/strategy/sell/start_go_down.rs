/// 三次的出货信号，对k线的要求比较低
use super::SellReason;
use super::SellStrategy;
use crate::ex_info::Symbol;
use crate::utils::timestamp2date;
use crate::{
    get_last_bar_shape_score, get_last_bar_volume_score, notify_lark, recent_kline_shape_score,
    take_order, Kline, MathOperation, MathOperation2, Pair, TakeOrderInfo, TakeType,
};
use std::collections::HashMap;
use std::error::Error;
use std::ops::{Div, Mul};


pub struct SGD {}

// 三连阴对交易量有要求不能低于20分之一
impl SGD {
    fn name() -> SellReason {
        SellReason::StartGoDown
    }

    pub async fn condition_passed(
        take_order_pair2: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
        line_datas: &[Kline],
        pair: &Symbol,
        taker_amount: f32,
        price: f32,
        is_real_trading: bool,
    ) -> Result<bool, Box<dyn Error>> {
        let pair_symbol = pair.symbol.as_str();
        let now = line_datas[359].open_time + 1000;
        let take_sell_type = TakeType {
            pair: pair_symbol.to_string(),
            sell_reason: Self::name(),
        };

        let half_hour_inc_ratio =
            (line_datas[358].open_price.to_f32() - line_datas[328].open_price.to_f32()).div(30.0);
        let ten_minutes_inc_ratio =
            (line_datas[358].open_price.to_f32() - line_datas[348].open_price.to_f32()).div(10.0);

        let broken_line_datas = &line_datas[340..360];
        let shape_score = get_last_bar_shape_score(broken_line_datas.to_owned());
        let volume_score = get_last_bar_volume_score(broken_line_datas.to_owned());
        //8-17。多一个作为价格比较的基准
        let recent_shape_score = recent_kline_shape_score(broken_line_datas[7..=17].to_vec());

        //总分分别是：7分，5分，10分
        //分为三种情况：强信号直接下单，弱信号加入观测名单，弱信号且已经在观查名单且距离观察名单超过五分钟的就下单，
        let take_info = take_order_pair2.get_mut(&take_sell_type);
         if take_info.as_ref().is_some()
            && take_info.as_ref().unwrap().len() >= 4
            && line_datas[358].close_price.to_f32() < line_datas[240].close_price.to_f32()
        {
            if is_real_trading {
                take_order(pair_symbol.to_string(), taker_amount, "SELL".to_string()).await;
            }
            let order_info = TakeOrderInfo {
                take_time: now,
                sell_price: price,
                buy_price: None,
                amount: taker_amount,
                top_bar: broken_line_datas[18].clone(),
                is_took: true,
            };
            take_order_pair2.insert(take_sell_type, vec![order_info]);
            let push_text = format!("reason {}: take_sell_order: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    <&str>::from(Self::name()), pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
            );
            warn!("now {}, {}", timestamp2date(now), push_text);
            if is_real_trading {
                notify_lark(push_text).await?;
            }
        } else if shape_score >= 4 && volume_score >= 3
            && (take_info.as_ref().is_none() && recent_shape_score >= 6
            || take_info.as_ref().is_some() && recent_shape_score >= 3
        ){
            let order_info = TakeOrderInfo {
                take_time: now,
                sell_price: price,
                buy_price: None,
                amount: taker_amount, //not care
                top_bar: broken_line_datas[18].clone(),
                is_took: false,
            };
            if take_info.as_ref().is_none() {
                take_order_pair2.insert(take_sell_type, vec![order_info]);
            } else if take_info.as_ref().is_some() && take_info.as_ref().unwrap().last().unwrap().is_took == true {
                return Ok(false);
            } else {
                take_info.unwrap().push(order_info)
            }
            let push_text = format!("reason {}: add_observe_list: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    <&str>::from(Self::name()),pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
            );

            warn!("now {}, {}", timestamp2date(now), push_text);
            if is_real_trading {
                notify_lark(push_text).await?;
            }
        } else {
            debug!("Have no take order signal,\
                     below is detail score:market {},shape_score {},volume_score {},recent_shape_score {}",
                              pair_symbol,shape_score,volume_score,recent_shape_score
                     );
        }
        Ok(false)
    }
}
