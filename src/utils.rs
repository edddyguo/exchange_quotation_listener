use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::ops::Add;
//use hex_literal::hex;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

type HmacSha256 = Hmac<Sha256>;

pub trait MathOperation {
    fn to_fix(&self, precision: u32) -> f32;
}

pub trait MathOperation2 {
    fn to_f32(&self) -> f32;
}

impl MathOperation for f32 {
    /// Keep decimal significant digits
    fn to_fix(&self, precision: u32) -> f32 {
        let times = 10_u32.pow(precision);
        let number = self * times as f32;
        let real_number = number.round();
        let decimal_number = Decimal::new(real_number as i64, precision);
        decimal_number.to_f32().unwrap()
    }
}

impl MathOperation2 for String {
    /// Keep decimal significant digits
    fn to_f32(&self) -> f32 {
        self.parse::<f32>().unwrap()
    }
}

///
pub fn get_unix_timestamp_ms() -> i64 {
    let now = Utc::now();
    now.timestamp_millis()
}
pub fn timestamp2date(time: u64) -> String {
    let naive = NaiveDateTime::from_timestamp_millis(time as i64).unwrap();
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);
    let newdate = datetime.format("%Y-%m-%d %H:%M:%S %f");
    // Print the newly formatted date and time
    newdate.to_string()
}

pub fn hmac_sha256_sign(message: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(
        b"4UDzQ00ejK4FeppVi9jtvOQzxd6JtrxKa5SWihqfcqfAYTu1N0CIo6hKrhCril3g",
    )
    .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    // `result` has type `CtOutput` which is a thin wrapper around array of
    // bytes for providing constant time equality check
    let result = mac.finalize();
    // To get underlying array use `into_bytes`, but be careful, since
    // incorrect use of the code value may permit timing attacks which defeats
    // the security provided by the `CtOutput`
    let bytes = result.into_bytes().to_vec();
    let mut signature: String = "".to_string();
    for iterm in bytes {
        signature = format!("{}{:0>2x}", signature, iterm);
    }
    //let test2 = String::from_utf8(test1.to_vec()).unwrap();
    //test2
    signature
}

#[cfg(test)]
mod tests {
    use crate::get_unix_timestamp_ms;
    use crate::utils::hmac_sha256_sign;

    #[test]
    fn test_get_unix_timestamp_ms() {
        let time = get_unix_timestamp_ms();
        println!("{:?}", time)
    }

    #[test]
    fn test_hmac_sha256_sign() {
        let signature = hmac_sha256_sign("recvWindow=500000000&timestamp=1674871881000");
        println!("-------{:?}", signature);
        assert_eq!(
            signature,
            "1def85bc5e6ef11fe8b1da73a05aa123c5d4e83d8a0025db9e9a5d0db1237fce".to_string()
        );
    }
}
