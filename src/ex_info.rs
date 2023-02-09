use serde::Deserialize;
use serde::Serialize;
//use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EXInfo {
    pub timezone: String,
    pub server_time: i64,
    pub futures_type: String,
    pub rate_limits: Vec<RateLimit>,
    pub exchange_filters: Vec<String>, //todo
    pub assets: Vec<Asset>,
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
pub struct Asset {
    pub asset: String,
    pub margin_available: bool,
    pub auto_asset_exchange: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize,Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Symbol {
    pub symbol: String,
    pub pair: String,
    pub contract_type: String,
    pub delivery_date: i64,
    pub onboard_date: i64,
    pub status: String,
    pub maint_margin_percent: String,
    pub required_margin_percent: String,
    pub base_asset: String,
    pub quote_asset: String,
    pub margin_asset: String,
    pub price_precision: i64,
    pub quantity_precision: i64,
    pub base_asset_precision: i64,
    pub quote_precision: i64,
    pub underlying_type: String,
    pub underlying_sub_type: Vec<String>,
    pub settle_plan: i64,
    pub trigger_protect: String,
    pub liquidation_fee: String,
    pub market_take_bound: String,
    pub filters: Vec<Filter>,
    pub order_types: Vec<String>,
    pub time_in_force: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize,Eq,Hash)]
#[serde(rename_all = "camelCase")]
pub struct Filter {
    pub min_price: Option<String>,
    pub max_price: Option<String>,
    pub filter_type: String,
    pub tick_size: Option<String>,
    pub step_size: Option<String>,
    pub max_qty: Option<String>,
    pub min_qty: Option<String>,
    pub limit: Option<i64>,
    pub notional: Option<String>,
    pub multiplier_down: Option<String>,
    pub multiplier_up: Option<String>,
    pub multiplier_decimal: Option<String>,
}

// proxychains4 curl -X GET "https://fapi.binance.com/fapi/v1/exchangeInfo" |
// jq | grep "symbol\"" | grep -v "BUSD\|331\|1000" | wc -l
//剔除100相关的合约
pub async fn list_all_pair() -> Vec<Symbol> {
    let url = format!("https://fapi.binance.com/fapi/v1/exchangeInfo");

    let client = reqwest::Client::new();
    let res = client
        .get(url)
        .send()
        .await
        .unwrap()
        .json::<EXInfo>()
        .await
        .unwrap();
    let symbols = res.symbols;
    //println!("list_all_pair result {:#?}",symbols);

    let pairs = symbols
        .iter()
        .filter(|x| !x.symbol.contains("BUSD"))
        .filter(|x| !x.symbol.contains("1000"))
        .filter(|x| !x.symbol.contains("331"))
        .filter(|x| !x.symbol.contains("DEFI"))
        .filter(|x| !x.symbol.contains("BTCDOM"))
        .filter(|x| !x.symbol.contains("FOOTBALL"))
        .filter(|x| !x.symbol.contains("LUNA2"))
        .filter(|x| !x.symbol.contains("BLUEBIRDUSDT"))
        .filter(|x| x.status == "TRADING")
        .map(|x| x.to_owned())
        .collect::<Vec<Symbol>>();
    println!("list_all_pair result {:?}", pairs.len());
    pairs
}

#[cfg(test)]
mod tests {
    use crate::ex_info::list_all_pair;

    #[tokio::test]
    async fn test_list_all_pair() {
        let _ = list_all_pair().await;
    }
}
