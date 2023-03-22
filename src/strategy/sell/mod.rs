pub mod a_strong_signal;
pub mod sequential_take_order;
pub mod three_continuous_signal;
pub mod two_middle_signal;
pub mod a_very_strong_signal;

use crate::ex_info::Symbol;
use crate::{Kline, Pair, StrategyEffect, TakeOrderInfo};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TakeType {
    pub pair: String,
    pub sell_reason: SellReason,
}

//context is the bar detail
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum SellReason {
    AStrongSignal,
    AVeryStrongSignal,
    TwoMiddleSignal,
    ThreeContinuousSignal,
    RaiseIsStop,
    SequentialTakeOrder,
    Buy1
}

/*impl SellReason {
    pub fn to_string(&self) -> String {
        match self {
            SellReason::AStrongSignal => "AStrongSignal".to_string(),
            SellReason::TwoMiddleSignal => "TwoMiddleSignal".to_string(),
            SellReason::ThreeContinuousSignal => "ThreeContinuousSignal".to_string(),
            SellReason::RaiseIsStop => "RaiseIsStop".to_string(),
        }
    }
}*/

impl From<&str> for SellReason {
    fn from(v: &str) -> Self {
        match v {
            "AStrongSignal" => SellReason::AStrongSignal,
            "AVeryStrongSignal" => SellReason::AVeryStrongSignal,
            "TwoMiddleSignal" => SellReason::TwoMiddleSignal,
            "ThreeContinuousSignal" =>  SellReason::ThreeContinuousSignal,
            "RaiseIsStop" => SellReason::RaiseIsStop,
            "SequentialTakeOrder" => SellReason::SequentialTakeOrder,
            "Buy1" => SellReason::Buy1,
            _  => unreachable!()
        }
    }
}

impl From<SellReason> for &str {
    fn from(v: SellReason) -> Self {
        match v {
            SellReason::AStrongSignal => "AStrongSignal",
            SellReason::AVeryStrongSignal => "AVeryStrongSignal",
            SellReason::TwoMiddleSignal => "TwoMiddleSignal",
            SellReason::ThreeContinuousSignal => "ThreeContinuousSignal",
            SellReason::RaiseIsStop => "RaiseIsStop",
            SellReason::SequentialTakeOrder => "SequentialTakeOrder",
            SellReason::Buy1 => "Buy1",
        }
    }
}

#[async_trait]
pub trait SellStrategy {
    fn name() -> SellReason;
    //fixme: 目前trait的async函数实现有问题
    async fn condition_passed<'a>(
        take_order_pair2: &'a mut HashMap<TakeType, Vec<TakeOrderInfo>>,
        line_datas: &[Kline],
        pair: &Symbol,
        balance: f32,
        is_real_trading: bool,
    ) -> Result<bool, Box<dyn std::error::Error>>;
}
