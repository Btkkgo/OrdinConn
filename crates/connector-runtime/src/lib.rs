use chrono::Utc;
use evidence_core::{Evidence, EvidenceRelation, FactualLevel, SourceType};
use market_core::{AssetRef, Market};
use serde::{Deserialize, Serialize};
use signal_core::{
    EvidenceLink, PublishError, Signal, SignalCandidate, SignalCategory, SignalDirection,
    SignalPublisher,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorDescriptor {
    pub id: String,
    pub name: String,
    pub market: String,
    pub connector_type: String,
    pub capabilities: Vec<String>,
    pub auth_type: String,
    pub status: String,
    pub reliability: f64,
    pub last_update: String,
    pub mock: bool,
}

#[derive(Clone, Debug)]
pub struct MockPipelineOutput {
    pub evidence: Vec<Evidence>,
    pub signals: Vec<Signal>,
    pub connectors: Vec<ConnectorDescriptor>,
}

#[derive(Default)]
pub struct MockConnectorSet;

impl MockConnectorSet {
    pub async fn run(&self) -> Result<MockPipelineOutput, PublishError> {
        let definitions = fixture_definitions();
        let publisher = SignalPublisher;
        let mut evidence = Vec::with_capacity(definitions.len());
        let mut signals = Vec::with_capacity(definitions.len());

        for definition in definitions {
            let item = Evidence::new(
                definition.connector,
                definition.source_type,
                AssetRef::new(definition.market, definition.asset),
                definition.evidence_title,
                definition.evidence_content,
                definition.factual_level,
                definition.reliability,
                definition.confidence,
                Utc::now(),
            );
            let candidate = SignalCandidate::new(
                AssetRef::new(definition.market, definition.asset),
                definition.category,
                definition.title,
                definition.summary,
                definition.direction,
                vec![EvidenceLink {
                    evidence: item.clone(),
                    relation: EvidenceRelation::Primary,
                }],
            );
            evidence.push(item);
            signals.push(publisher.publish(candidate)?);
        }

        Ok(MockPipelineOutput {
            evidence,
            signals,
            connectors: connector_descriptors(),
        })
    }
}

struct FixtureDefinition {
    market: Market,
    category: SignalCategory,
    asset: &'static str,
    title: &'static str,
    summary: &'static str,
    direction: SignalDirection,
    connector: &'static str,
    source_type: SourceType,
    evidence_title: &'static str,
    evidence_content: &'static str,
    factual_level: FactualLevel,
    reliability: f64,
    confidence: f64,
}

fn fixture_definitions() -> Vec<FixtureDefinition> {
    use SignalCategory as C;
    use SignalDirection as D;
    vec![
        fixture(
            Market::Traditional,
            C::Trading,
            "SPY",
            "Breadth expansion",
            "Participation broadened across the mock session.",
            D::Bullish,
            "Mock Traditional Market",
        ),
        fixture(
            Market::Traditional,
            C::Trading,
            "GLD",
            "Defensive momentum",
            "Gold momentum strengthened while volatility remained contained.",
            D::Bullish,
            "Mock Traditional Market",
        ),
        fixture(
            Market::Traditional,
            C::Trading,
            "CL",
            "Crude range pressure",
            "Mock order flow shows supply near the upper range.",
            D::Watch,
            "Mock Traditional Market",
        ),
        fixture(
            Market::Traditional,
            C::Event,
            "USD",
            "Central-bank repricing",
            "A mock policy event changed the expected rate path.",
            D::Watch,
            "Mock News",
        ),
        fixture(
            Market::Traditional,
            C::Event,
            "NVDA",
            "AI infrastructure update",
            "A mock company disclosure raises near-term infrastructure expectations.",
            D::Bullish,
            "Mock News",
        ),
        fixture(
            Market::Traditional,
            C::Event,
            "TLT",
            "Inflation surprise",
            "A mock macro release increases duration risk.",
            D::Bearish,
            "Mock News",
        ),
        fixture(
            Market::Traditional,
            C::Demand,
            "MU",
            "Memory demand tightening",
            "Mock procurement evidence points to stronger memory demand.",
            D::Bullish,
            "Mock Demand",
        ),
        fixture(
            Market::Traditional,
            C::Demand,
            "CAT",
            "Equipment backlog",
            "Mock manufacturing demand extends the equipment backlog.",
            D::Bullish,
            "Mock Demand",
        ),
        fixture(
            Market::Traditional,
            C::Demand,
            "COPX",
            "Grid investment demand",
            "Mock project activity supports a copper demand watch.",
            D::Watch,
            "Mock Demand",
        ),
        fixture(
            Market::Crypto,
            C::Onchain,
            "BTC",
            "Exchange outflow cluster",
            "Mock wallet events show a coordinated exchange outflow.",
            D::Bullish,
            "Mock Blockchain",
        ),
        fixture(
            Market::Crypto,
            C::Onchain,
            "ETH",
            "Staking deposit increase",
            "Mock staking deposits increased over the observation window.",
            D::Bullish,
            "Mock Blockchain",
        ),
        fixture(
            Market::Crypto,
            C::Onchain,
            "SOL",
            "Active address acceleration",
            "Mock active-address growth moved above its recent baseline.",
            D::Watch,
            "Mock Blockchain",
        ),
        fixture(
            Market::Crypto,
            C::Event,
            "BTC",
            "Regulatory clarification",
            "A mock official notice reduces one narrow policy uncertainty.",
            D::Bullish,
            "Mock Social",
        ),
        fixture(
            Market::Crypto,
            C::Event,
            "ETH",
            "Protocol release window",
            "A mock official release notice creates an event watch.",
            D::Watch,
            "Mock Social",
        ),
        fixture(
            Market::Crypto,
            C::Event,
            "SOL",
            "Ecosystem incident resolved",
            "A mock project update reports service recovery.",
            D::Neutral,
            "Mock Social",
        ),
        fixture(
            Market::Crypto,
            C::Exchange,
            "BTC",
            "Perpetual basis reset",
            "Mock funding normalized while spot demand remained steady.",
            D::Bullish,
            "Mock Exchange",
        ),
        fixture(
            Market::Crypto,
            C::Exchange,
            "ETH",
            "Open-interest compression",
            "Mock open interest declined without matching spot weakness.",
            D::Watch,
            "Mock Exchange",
        ),
        fixture(
            Market::Crypto,
            C::Exchange,
            "SOL",
            "Liquidation imbalance",
            "Mock liquidation flow is concentrated on one side.",
            D::Watch,
            "Mock Exchange",
        ),
    ]
}

fn fixture(
    market: Market,
    category: SignalCategory,
    asset: &'static str,
    title: &'static str,
    summary: &'static str,
    direction: SignalDirection,
    connector: &'static str,
) -> FixtureDefinition {
    let source_type = match connector {
        "Mock Blockchain" => SourceType::Blockchain,
        "Mock Exchange" => SourceType::ExchangeApi,
        "Mock News" => SourceType::OfficialNews,
        "Mock Social" => SourceType::Social,
        _ => SourceType::OfficialApi,
    };
    FixtureDefinition {
        market,
        category,
        asset,
        title,
        summary,
        direction,
        connector,
        source_type,
        evidence_title: "Mock source observation",
        evidence_content: "Deterministic demonstration evidence generated by a labeled mock connector.",
        factual_level: FactualLevel::Observed,
        reliability: 0.82,
        confidence: 0.8,
    }
}

pub fn connector_descriptors() -> Vec<ConnectorDescriptor> {
    [
        (
            "mock-traditional",
            "Mock Traditional Market",
            "traditional",
            vec!["realtime_price", "orderbook"],
        ),
        (
            "mock-news",
            "Mock News",
            "cross_market",
            vec!["news", "macro"],
        ),
        (
            "mock-demand",
            "Mock Demand",
            "traditional",
            vec!["fundamentals"],
        ),
        (
            "mock-blockchain",
            "Mock Blockchain",
            "crypto",
            vec!["onchain"],
        ),
        (
            "mock-exchange",
            "Mock Exchange",
            "crypto",
            vec!["realtime_price", "orderbook"],
        ),
        ("mock-social", "Mock Social", "crypto", vec!["social"]),
    ]
    .into_iter()
    .map(|(id, name, market, capabilities)| ConnectorDescriptor {
        id: id.into(),
        name: name.into(),
        market: market.into(),
        connector_type: "mock".into(),
        capabilities: capabilities.into_iter().map(String::from).collect(),
        auth_type: "none".into(),
        status: "ready".into(),
        reliability: 0.82,
        last_update: Utc::now().to_rfc3339(),
        mock: true,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_connectors_publish_three_signals_for_each_abc_lane() {
        let pipeline = MockConnectorSet;
        let output = pipeline.run().await.unwrap();

        assert_eq!(output.evidence.len(), 18);
        assert_eq!(output.signals.len(), 18);
        for lane in ["trading", "event", "demand", "onchain", "exchange"] {
            assert!(
                output
                    .signals
                    .iter()
                    .filter(|signal| signal.category.as_str() == lane)
                    .count()
                    >= 3
            );
        }
        assert_eq!(
            output
                .signals
                .iter()
                .filter(|signal| signal.market() == "traditional")
                .count(),
            9
        );
        assert_eq!(
            output
                .signals
                .iter()
                .filter(|signal| signal.market() == "crypto")
                .count(),
            9
        );
        assert!(
            output
                .signals
                .iter()
                .all(|signal| !signal.evidence.is_empty())
        );
    }
}
