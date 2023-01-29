use std::ops::Add;
use chrono::Utc;
use sha2::Sha256;
use hmac::{Hmac, Mac};
//use hex_literal::hex;

type HmacSha256 = Hmac<Sha256>;


///
pub fn get_unix_timestamp_ms() -> i64 {
    let now = Utc::now();
    now.timestamp_millis()
}
pub fn hmac_sha256_sign(message: &str) -> String{
    let mut mac = HmacSha256::new_from_slice(b"Jh9pTnrvQ6vW1cZH3pS7yaH6Pm954M6Tt5Huq8Ti1xIC2BsfFGJI0z889RXgX8Q1")
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

// `result` has type `CtOutput` which is a thin wrapper around array of
// bytes for providing constant time equality check
    let result = mac.finalize();
// To get underlying array use `into_bytes`, but be careful, since
// incorrect use of the code value may permit timing attacks which defeats
// the security provided by the `CtOutput`
    let bytes = result.into_bytes().to_vec();
    let mut signature : String = "".to_string();
    for iterm in bytes {
        signature = format!("{}{:0>2x}",signature,iterm);
    }
    //let test2 = String::from_utf8(test1.to_vec()).unwrap();
    //test2
    signature
}

#[cfg(test)]
mod tests{
    use crate::get_unix_timestamp_ms;
    use crate::utils::hmac_sha256_sign;

    #[test]
    fn test_get_unix_timestamp_ms(){
        let time  = get_unix_timestamp_ms();
        println!("{:?}",time)
    }

    #[test]
    fn test_hmac_sha256_sign(){
        let signature  = hmac_sha256_sign("recvWindow=500000000&timestamp=1674871881000");
        println!("-------{:?}",signature);
        assert_eq!(signature,"1def85bc5e6ef11fe8b1da73a05aa123c5d4e83d8a0025db9e9a5d0db1237fce".to_string());
    }
}