use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Market {
    Traditional,
    Crypto,
}

impl Market {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Traditional => "traditional",
            Self::Crypto => "crypto",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRef {
    pub market: Market,
    pub symbol: String,
}

impl AssetRef {
    pub fn new(market: Market, symbol: impl Into<String>) -> Self {
        Self {
            market,
            symbol: symbol.into(),
        }
    }
}

pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::now_v7())
}

pub fn utc_now() -> DateTime<Utc> {
    Utc::now()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_reference_retains_market_and_symbol() {
        let asset = AssetRef::new(Market::Crypto, "BTC");
        assert_eq!(asset.market, Market::Crypto);
        assert_eq!(asset.symbol, "BTC");
    }
}
