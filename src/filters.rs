use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub timezone: String,
    pub server_time: i64,
    pub rate_limits: Vec<RateLimit>,
    pub exchange_filters: Vec<Filter>,
    pub symbols: Vec<Symbol>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub rate_limit_type: String,
    pub interval: String,
    pub interval_num: i64,
    pub limit: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub symbol: String,
    pub status: String,
    pub base_asset: String,
    pub base_asset_precision: i64,
    pub quote_asset: String,
    pub quote_precision: i64,
    pub quote_asset_precision: i64,
    pub base_commission_precision: i64,
    pub quote_commission_precision: i64,
    pub order_types: Vec<String>,
    pub iceberg_allowed: bool,
    pub oco_allowed: bool,
    pub quote_order_qty_market_allowed: bool,
    pub allow_trailing_stop: bool,
    pub cancel_replace_allowed: bool,
    pub is_spot_trading_allowed: bool,
    pub is_margin_trading_allowed: bool,
    pub filters: Vec<Filter>,
    pub permissions: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub filter_type: String,
    pub min_price: Option<String>,
    pub max_price: Option<String>,
    pub tick_size: Option<String>,
    pub multiplier_up: Option<String>,
    pub multiplier_down: Option<String>,
    pub avg_price_mins: Option<i64>,
    pub min_qty: Option<String>,
    pub max_qty: Option<String>,
    pub step_size: Option<String>,
    pub min_notional: Option<String>,
    pub apply_to_market: Option<bool>,
    pub limit: Option<i64>,
    pub min_trailing_above_delta: Option<i64>,
    pub max_trailing_above_delta: Option<i64>,
    pub min_trailing_below_delta: Option<i64>,
    pub max_trailing_below_delta: Option<i64>,
    pub max_num_orders: Option<i64>,
    pub max_num_algo_orders: Option<i64>,
}
