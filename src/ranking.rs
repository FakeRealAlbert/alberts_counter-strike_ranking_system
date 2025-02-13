#![allow(dead_code)]

use std::ops::RangeInclusive;
use crate::data_loader::*;
use crate::ranking_context;
use crate::ranking_context::*;
use crate::util::*;

pub fn gen_rank_new(matches: &[Match], events: &[Event], teams: &mut [Team], ranking_context: &RankingContext) {
    seed_teams(matches, events, teams, ranking_context);
    map_elo_adjustments(matches, teams, ranking_context);
}

pub fn elo_result(winner_elo: f64, loser_elo: f64, ranking_context: &RankingContext) -> f64 {
    ranking_context.elo_k * (1.0 - 1.0 / ( 1.0 + f64::powf(10.0, (loser_elo - winner_elo)/ranking_context.elo_delta)))
}

fn seed_teams(matches: &[Match], events: &[Event], teams: &mut [Team], ranking_context: &RankingContext) {    
    // FaZe 1: Own Network
    for (idx, team) in teams.iter_mut().enumerate() {
        let mut opponents : Vec<(usize, f64)> = Vec::new();

        for m in matches {
            if m.winning_team != idx { continue; } // We only go through winning matches

            // Adds information_context for each new opponent. If we've already played this team, update info context 
            let opp_id = m.other_team(idx);
            let mut have_played_before = false;
            for op in &mut opponents {
                if op.0 != opp_id { continue; }
                op.1 = m.information_context;
                have_played_before = true;
                break;
            }
           
            if !have_played_before {
                opponents.push( (opp_id, m.information_context) );
            }
        }

        for op in opponents {
            team.own_network += op.1;
        }
    }

    // FaZe 2: Events and Bounty Offered
    for ev in events {
        for prize_dist in &ev.prize_distribution {
            if !prize_dist.is_in_ranking { continue; }

            let scale: f64 = ranking_context.time_mod(ev.last_match_time);
            teams[prize_dist.team_id].adjusted_winnings += prize_dist.prize.sqrt() * scale;
            teams[prize_dist.team_id].event_participation += ev.prize_pool.max(1.0).log10() * scale;
        }
    }    

    let reference_winnings = nth_highest(teams, ranking_context, |t| t.adjusted_winnings);
    let reference_network  = nth_highest(teams, ranking_context, |t| t.own_network);
    let reference_event    = nth_highest(teams, ranking_context, |t| t.event_participation);

    for t in teams.iter_mut() {
        t.prize_money           = f64::min(t.adjusted_winnings / reference_winnings, 1.0);
        t.own_network           = f64::min( t.own_network / reference_network,1.0);
        t.event_participation   = f64::min( t.event_participation / reference_event,1.0);
    }

    // FaZe 3: Go through won matches again to calculate OPP NETWORK og BOUNTY COLLECTED
    for idx in 0..teams.len() {
        let mut opp_winnings = Vec::new();
        let mut opp_networks = Vec::new();

        for m in matches {
            if m.winning_team != idx { continue; }

            let opp_idx = m.other_team(idx);
            opp_winnings.push(teams[opp_idx].adjusted_winnings * m.information_context);
            opp_networks.push(teams[opp_idx].own_network       * m.information_context);   
        }

        teams[idx].opponent_winnings = sum_of_nth_best(opp_winnings, ranking_context);
        teams[idx].opponent_network  = sum_of_nth_best(opp_networks, ranking_context);
    }

    let reference_opp_network  = nth_highest(teams, ranking_context, |t| t.opponent_network);
    let reference_opp_winnings = nth_highest(teams, ranking_context, |t| t.opponent_winnings);

    let mut highest_sum_of_factors = f64::MIN;
    let mut lowest_sum_of_factors  = f64::MAX;
    for (idx, t) in teams.iter_mut().enumerate() {
        t.opponent_winnings = curve_function(f64::min(t.opponent_winnings / reference_opp_winnings, 1.0));
        t.opponent_network      = curve_function(f64::min(t.opponent_network / reference_opp_network, 1.0));

        t.sum_of_factors = t.opponent_winnings   * ranking_context.opponet_winnings_weight
                         + t.prize_money         * ranking_context.prize_money_weight
                         + t.opponent_network    * ranking_context.opponent_network_weight
                         + t.event_participation * ranking_context.event_participation_weight;

        highest_sum_of_factors = f64::max(highest_sum_of_factors, t.sum_of_factors);
        lowest_sum_of_factors  = f64::min(lowest_sum_of_factors,  t.sum_of_factors);
    }

    for team in teams.iter_mut() {
        team.seed_points = remap_value_clamped(team.sum_of_factors, 
            lowest_sum_of_factors, 
            highest_sum_of_factors, 
            ranking_context.min_seeded_rank,
            ranking_context.max_seeded_rank
        );

        team.elo = team.seed_points;
    }
}

fn map_elo_adjustments(matches: &[Match], teams: &mut [Team], ranking_context: &RankingContext) {
    for m in matches {
        for map in &m.maps {
            let map_winning_team = if map.team_1_score > map.team_2_score { m.team_1_id } else { m.team_2_id };
            let map_losing_team  = m.other_team(map_winning_team);

            let elo_diff = elo_result(teams[map_winning_team].elo, teams[map_losing_team].elo, ranking_context);

            teams[map_winning_team].elo += elo_diff;
            teams[map_losing_team].elo -= elo_diff;
        }
    }        
}

// Expects an input between 0.0 and 1.0 inclusive. Curves the results out, simply meaning worse results become less worse
// 0.1 => 0.5 | 0.2 => 0.6 | 0.5 => 0.75 | 0.8 => 0.9
pub fn curve_function(x: f64) -> f64 {
    debug_assert!(x <= 1.0);
    debug_assert!(x >= 0.0);
    1.0 / (1.0 + f64::abs(f64::log10(x)))
}

// Finds the Nth highest (from ranking_context.top_outlier_count) value of t_var for each team.
// The closure is just a fancy way of letting us use this function for each factor.
fn nth_highest<F>(teams: &[Team], ranking_context: &RankingContext, t_var: F) -> f64 where 
    F: Fn(&Team) -> f64 {
    let mut var_vec = Vec::new();
    for t in teams {
        var_vec.push(t_var(t));
    }

    var_vec.sort_unstable_by(|a, b| b.partial_cmp(a).unwrap());
    var_vec[ranking_context.top_outlier_count - 1]
}

fn sum_of_nth_best(mut vec: Vec<f64>,ranking_context: &RankingContext) -> f64 {
    vec.sort_by(|a, b| b.partial_cmp(a).unwrap());
    vec.resize(ranking_context.factor_bucket_size,0.0);
    sum_vector(vec)
}