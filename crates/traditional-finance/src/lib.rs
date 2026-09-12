pub const LANES: [&str; 3] = ["trading", "event", "demand"];

#[cfg(test)]
mod tests {
    #[test]
    fn traditional_lanes_are_stable() {
        assert_eq!(super::LANES, ["trading", "event", "demand"]);
    }
}
