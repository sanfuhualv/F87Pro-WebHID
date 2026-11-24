use std::time::Duration;

pub const SEND_BUDGET: Duration = Duration::from_millis(28);
pub fn should_send_audio(keep_sending: bool, quiet_for: Duration) -> bool {
    keep_sending || quiet_for < Duration::from_secs(60)
}

pub struct PacketBudget {
    limit: usize,
    packet_ms: f64,
}
impl PacketBudget {
    pub fn new() -> Self { Self { limit: 8, packet_ms: 3.0 } }
    pub fn limit(&self) -> usize { self.limit }
    pub fn packet_time(&self) -> Duration { Duration::from_secs_f64(self.packet_ms * 1.25 / 1000.0) }
    pub fn observe(&mut self, elapsed: Duration, completed: usize, complete: bool) {
        let cost = elapsed.as_secs_f64() * 1000.0 / completed.max(1) as f64;
        // React immediately to a slowdown; increase throughput gradually.
        self.packet_ms = cost.max(self.packet_ms * 0.8 + cost * 0.2).max(0.1);
        let target = (SEND_BUDGET.as_secs_f64() * 1000.0 / (self.packet_ms * 1.25)) as usize;
        self.limit = if complete {
            target.clamp(1, 15).min(self.limit + 1)
        } else { completed.min(self.limit / 2).max(1) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_stops_at_one_minute_and_can_be_disabled() {
        assert!(should_send_audio(false, Duration::from_secs(59)));
        assert!(!should_send_audio(false, Duration::from_secs(60)));
        assert!(should_send_audio(true, Duration::from_secs(120)));
        assert!(should_send_audio(false, Duration::ZERO));
    }
    #[test]
    fn slow_link_reduces_packets_and_fast_link_recovers_gradually() {
        let mut budget = PacketBudget::new();
        budget.observe(Duration::from_millis(40), 2, false);
        assert!(budget.limit() <= 2);
        let before = budget.limit();
        budget.observe(Duration::from_millis(1), 1, true);
        assert!(budget.limit() <= before + 1);
        for _ in 0..40 { budget.observe(Duration::from_millis(1), 1, true); }
        assert_eq!(budget.limit(), 15);
    }
    #[test]
    fn zero_completed_packets_is_safe() {
        let mut budget = PacketBudget::new();
        budget.observe(SEND_BUDGET, 0, false);
        assert_eq!(budget.limit(), 1);
    }
}
