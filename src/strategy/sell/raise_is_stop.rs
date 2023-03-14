use std::collections::HashMap;
use std::error::Error;
use std::ops::{Div, Mul};
use super::SellStrategy;
use super::SellReason;
use crate::{get_last_bar_shape_score, get_last_bar_volume_score, Kline, MathOperation, MathOperation2, notify_lark, Pair, recent_kline_shape_score, take_order, TakeOrderInfo, TakeType};
use crate::ex_info::Symbol;

struct RIS {}

impl RIS {
    fn name() -> SellReason {
        SellReason::RaiseIsStop
    }

    pub async fn condition_passed(take_order_pair2: &mut HashMap<TakeType, Vec<TakeOrderInfo>>,
                              line_datas: &[Kline],
                              pair: &Symbol,
                              balance: f32,
                              is_real_trading: bool) -> Result<bool, Box<dyn Error>> {
        todo!()
    }
}