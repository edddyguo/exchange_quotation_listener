/// 算出来对应的k线形态和做空信号得分

//GET /fapi/v1/ticker/price
//symbol	STRING	NO	交易对


use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Price {
    pub symbol: String,
    pub price: String,
    pub time: i64,
}

pub async  fn get_current_price(symbol: &str) -> f32{
    let url = format!("https://fapi.binance.com/fapi/v1/ticker/price?symbol={}", symbol);

    let client = reqwest::Client::new();
    let res = client.get(url)
        .send()
        .await
        .unwrap()
        .json::<Price>()
        .await
        .unwrap();
    println!("get_current_price result {:?}",res);
    res.price.parse::<f32>().unwrap()
}

#[cfg(test)]
mod tests{
    use crate::kline::get_current_price;

    #[test]
    fn is_strong_signal() {
        todo!()
    }

    #[tokio::test]
    async fn test_get_current_price(){
        get_current_price("RLCUSDT").await;
    }
}