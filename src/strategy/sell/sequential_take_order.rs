use super::SellReason;
use super::SellStrategy;
use crate::ex_info::Symbol;
use crate::{
    get_last_bar_shape_score, get_last_bar_volume_score, notify_lark, recent_kline_shape_score,
    take_order, Kline, MathOperation, MathOperation2, Pair, TakeOrderInfo, TakeType,
};
use std::collections::HashMap;
use std::error::Error;
use std::ops::{Div, Mul};
use crate::utils::timestamp2date;


pub struct STO {}

impl STO {
    fn name() -> SellReason {
        SellReason::SequentialTakeOrder
    }

    pub async fn condition_passed(
        take_order_pair: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
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

        let broken_line_datas = &line_datas[340..360];
        let shape_score = get_last_bar_shape_score(broken_line_datas.to_owned());
        let volume_score = get_last_bar_volume_score(broken_line_datas.to_owned());
        //8-17。多一个作为价格比较的基准
        let recent_shape_score = recent_kline_shape_score(broken_line_datas[8..=18].to_vec());

        if shape_score >= 4 && volume_score >= 3 {
            let mut push_text = "".to_string();
            let take_info = take_order_pair.get_mut(&take_sell_type);
            //二次拉升才下单,并且量大于2倍
            if take_info.as_ref().is_none() && recent_shape_score >= 6 && volume_score >= 4
                ||
                //插眼期间：包括已经下了别的单或者最后一单已经平仓
                //且加仓条件为最后一次加仓的价格的不能低1个点，量不能少于九成
                take_info.as_ref().is_some()
                    && broken_line_datas[18].volume.to_f32().div(0.9)
                    > take_info.as_ref().unwrap().last().unwrap().top_bar.volume.to_f32()
                && broken_line_datas[18].close_price.to_f32().div(0.99)
                    > take_info.as_ref().unwrap().last().unwrap().sell_price
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
                if take_info.is_none() {
                    take_order_pair.insert(take_sell_type, vec![order_info]);
                }else {
                    take_info.unwrap().push(order_info);
                }
                push_text = format!("reason {}: take_sell_order: market {},shape_score {},volume_score {},recent_shape_score {},taker_amount {}",
                                    <&str>::from(Self::name()),pair_symbol, shape_score, volume_score, recent_shape_score, taker_amount
                );
                warn!("now {}, {}", timestamp2date(now), push_text);
                if is_real_trading {
                    notify_lark(push_text).await?;
                }
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
