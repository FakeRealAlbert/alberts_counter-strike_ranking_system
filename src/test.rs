use crate::*;
use rand::prelude::*;

// Lazy function that checks error while adjusting a certain factor. Note that you have to manually change the lines
// To check different variables.
pub fn ranking_var_checker() {
    let mut ranking_context = RankingContext::default();
    ranking_context.time_window_end = 1693330518;
    ranking_context.time_window_start = 1693330518 - (6 * 30 * 24 * 60 * 60); // End time minus six months

    ranking_context.event_participation_weight = 0.0;

    while ranking_context.event_participation_weight < 4.0 {
        let (matches, events, mut teams) = load_data(
            "./data/matchdata_sample_20230829.json".to_string(), 
            &ranking_context
        );

        gen_rank_new(&matches, &events, &mut teams, &ranking_context);

        let error = analyze_fit(&teams, &matches, &ranking_context, false);
        println!("Weight {0:2.1}, error {1:5.4}",ranking_context.event_participation_weight,error);

        ranking_context.event_participation_weight += 0.1;
    }
}

// Finds difference between actual and expected win rate. Returns average error per match
pub fn analyze_fit(teams: &[Team], matches: &[Match], ranking_context: &RankingContext, verbose: bool) -> f64 {
    const BUCKET_SIZE: usize = 10;

    let mut bucket_wins = [0.0;BUCKET_SIZE];
    let mut bucket_ewins = [0.0;BUCKET_SIZE];
    let mut bucket_played = [0.0;BUCKET_SIZE];

    for m in matches {
        let ewr = expected_win_rate(teams[m.winning_team].elo, teams[m.losing_team_id()].elo, ranking_context);

        let bucket_index = (ewr * BUCKET_SIZE as f64).floor() as usize;

        bucket_wins[bucket_index] += 1.0;
        bucket_played[bucket_index] += 1.0;
        bucket_played[BUCKET_SIZE - 1 - bucket_index] += 1.0;
        bucket_ewins[bucket_index] += ewr;
        bucket_ewins[BUCKET_SIZE - 1 - bucket_index] += 1.0 - ewr;
    }

    let mut error = 0.0;
    let mut large_error = 0.0;

    let mut sum_matches_played: f64 = 0.0;
    for i in 0..(BUCKET_SIZE / 2) {
        error += (bucket_wins[i] - bucket_ewins[i]).abs();

        if (bucket_ewins[i] - 0.5).abs() > 0.2 {
            large_error += (bucket_wins[i] - bucket_ewins[i]).abs();
        }

        sum_matches_played += bucket_played[i];

        if verbose {
            println!("EWR Bucket {0:3.2} | Matches played: {1:5} | Win rate: {2:4.2} | ExWR: {3:4.2}",
                i as f64 / BUCKET_SIZE as f64,
                bucket_played[i],
                bucket_wins[i] / bucket_played[i],
                bucket_ewins[i] / bucket_played[i],
            );
        }
    }

    error /= sum_matches_played;

    if verbose {
        println!("Total error from this model: {error}");
    }

    error
} 

// Error for each team. analyze_fit can score bad models well if those models are egregiously over/underranking some teams.
// This complies error for every team and reports the worst offenders, as well as a sample of other teams for comparsions.
pub fn team_fit(teams: &[Team], matches: &[Match], ranking_context: &RankingContext, verbose: bool) -> Vec<(f64, f64)> {
    struct TeamDiff {
        idx: usize,
        abs_diff: f64,
        net_diff: f64,
    }

    fn report_team_fit(td: &TeamDiff, teams: &[Team]) {
        println!("{0:25} | Elo {1:6.1} | Absolute Diff. {2:6.2} | Net Diff. {3:6.2} | Matches PLayed {4:5}",
            teams[td.idx].name,
            teams[td.idx].elo,
            td.abs_diff,
            td.net_diff,
            teams[td.idx].matches_played,
        );
    }

    let mut team_diffs: Vec<TeamDiff> = Vec::new();
    
    for (idx, t) in teams.iter().enumerate() {
        //if !t.ranking_eligible() { continue; }

        let mut team_diff = TeamDiff {
            idx,
            abs_diff: 0.0,
            net_diff: 0.0,
        };

        for m in matches {
            if !m.is_in_game(idx) { continue; }

            let ewr = expected_win_rate(t.elo, teams[m.other_team(idx)].elo, ranking_context);
            let actual = if m.winning_team == idx { 1.0 } else { 0.0 };

            team_diff.abs_diff += (ewr - actual).abs();
            team_diff.net_diff += ewr - actual;
        }

        team_diff.abs_diff /= t.matches_played as f64;
        team_diff.net_diff /= t.matches_played as f64;

        team_diffs.push(team_diff);
    }

    let mut out = Vec::new();
    for i in &team_diffs {
        out.push( (i.abs_diff, i.net_diff) );
    }

    if verbose {
        let reported_team_names = [
            String::from("Vitality"),
            String::from("ENCE"),
            String::from("M80"),
            String::from("Nigma Galaxy"),
            String::from("ORKS"),
            String::from("BetBoom"),
            String::from("TRAFFIC Tashkent"),
        ];
    
        for td in &team_diffs {
            if reported_team_names.contains(&teams[td.idx].name) {
                report_team_fit(td, teams);
            }
        }
    
        team_diffs.sort_by(|a, b| a.abs_diff.partial_cmp(&b.abs_diff).unwrap());
        report_team_fit(team_diffs.first().unwrap(), teams);
        report_team_fit(team_diffs.last().unwrap(), teams);
    }

    out
} 

// Gives the winner's expected win rate 
fn expected_win_rate(winner_elo: f64, loser_elo: f64, ranking_context: &RankingContext) -> f64 {
    let win_delta  = elo_result(winner_elo, loser_elo, ranking_context);
    let lose_delta = elo_result(loser_elo, winner_elo, ranking_context);

    1.0 - ( win_delta / ( win_delta + lose_delta ) )
}