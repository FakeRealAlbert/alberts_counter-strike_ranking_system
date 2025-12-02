#![allow(dead_code)]

#[derive(Debug)]
pub struct RankingContext {
    pub top_outlier_count: usize,
    pub factor_bucket_size: usize,

    pub elo_k: f64,
    pub elo_delta: f64,

    pub time_window_start: u32,
    pub time_window_end: u32,
    pub time_grace_period: u32,
    pub time_decay_factor: f64,     // The power of decay. 1 is linear

    pub max_prize_pool_mod: f64,

    pub opponet_winnings_weight: f64,
    pub prize_money_weight: f64,
    pub opponent_network_weight: f64,
    pub event_participation_weight: f64,

    pub min_seeded_rank: f64,
    pub max_seeded_rank: f64,

    pub min_matches_for_ranking: u32,
    pub min_wins_for_ranking: u32,
}

impl RankingContext {
    pub fn default() -> Self {
        Self {
            top_outlier_count: 5,
            factor_bucket_size: 10,

            elo_k: 32.0,
            elo_delta: 400.0,

            time_window_start: u32::MIN,
            time_window_end: u32::MAX,
            time_grace_period: 30 * 24 * 60 * 60, // One month
            time_decay_factor: 1.0,

            max_prize_pool_mod: 1_000_000.0,

            opponet_winnings_weight: 1.0,
            prize_money_weight: 1.0,
            opponent_network_weight: 1.0,
            event_participation_weight: 1.0,

            min_seeded_rank: 400.0,
            max_seeded_rank: 2000.0,

            min_matches_for_ranking: 10,
            min_wins_for_ranking: 1,
        }
    }

    pub fn time_mod(&self, time: u32) -> f64 {
        let above = time.clamp(self.time_window_start, self.time_window_end - self.time_grace_period) - self.time_window_start;
        let below = self.time_window_end - self.time_window_start - self.time_grace_period;

        ((above as f64) / (below as f64)).powf(self.time_decay_factor) 
    }
}