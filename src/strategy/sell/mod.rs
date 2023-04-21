pub mod a_strong_signal;
pub mod sequential_take_order;
pub mod three_continuous_signal;
pub mod two_middle_signal;
pub mod a_very_strong_signal;
pub mod start_go_down;
pub mod two_middle_signal_v2;
pub mod a_very_strong_signal_v2;
pub mod a_strong_signal_v2;

use crate::ex_info::Symbol;
use crate::{Kline, Pair, StrategyEffect, TakeOrderInfo};
use async_trait::async_trait;
use std::collections::HashMap;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TakeType {
    pub pair: String,
    pub sell_reason: SellReason,
}

//context is the bar detail
//v2 is super v1，reduce profit and get more win ratio
#[derive(Debug, Clone, Eq, PartialEq, Hash,EnumIter)]
pub enum SellReason {
    AStrongSignal,
    AStrongSignal_V2,
    AVeryStrongSignal,
    AVeryStrongSignal_V2,
    TwoMiddleSignal,
    TwoMiddleSignal_V2,
    ThreeContinuousSignal,
    RaiseIsStop,
    SequentialTakeOrder,
    StartGoDown,
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
            "AStrongSignal_V2" => SellReason::AStrongSignal_V2,
            "AVeryStrongSignal" => SellReason::AVeryStrongSignal,
            "AVeryStrongSignal_V2" => SellReason::AVeryStrongSignal_V2,
            "TwoMiddleSignal" => SellReason::TwoMiddleSignal,
            "TwoMiddleSignal_V2" => SellReason::TwoMiddleSignal_V2,
            "ThreeContinuousSignal" =>  SellReason::ThreeContinuousSignal,
            "RaiseIsStop" => SellReason::RaiseIsStop,
            "SequentialTakeOrder" => SellReason::SequentialTakeOrder,
            "StartGoDown" => SellReason::StartGoDown,
            "Buy1" => SellReason::Buy1,
            _  => unreachable!()
        }
    }
}

impl From<SellReason> for &str {
    fn from(v: SellReason) -> Self {
        match v {
            SellReason::AStrongSignal => "AStrongSignal",
            SellReason::AStrongSignal_V2 => "AStrongSignal_V2",
            SellReason::AVeryStrongSignal => "AVeryStrongSignal",
            SellReason::AVeryStrongSignal_V2 => "AVeryStrongSignal_V2",
            SellReason::TwoMiddleSignal => "TwoMiddleSignal",
            SellReason::TwoMiddleSignal_V2 => "TwoMiddleSignal_V2",
            SellReason::ThreeContinuousSignal => "ThreeContinuousSignal",
            SellReason::RaiseIsStop => "RaiseIsStop",
            SellReason::SequentialTakeOrder => "SequentialTakeOrder",
            SellReason::StartGoDown => "StartGoDown",
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
