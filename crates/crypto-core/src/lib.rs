pub const LANES: [&str; 3] = ["onchain", "event", "exchange"];

#[cfg(test)]
mod tests {
    #[test]
    fn crypto_lanes_are_stable() {
        assert_eq!(super::LANES, ["onchain", "event", "exchange"]);
    }
}
