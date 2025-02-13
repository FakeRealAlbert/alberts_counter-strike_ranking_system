// None of these work very well, but they're fun to look at

pub fn gen_rank_pure_elo_map(matches: &Vec<Match>, teams: &mut Vec<Team>, ranking_context: &RankingContext) {
    const STARTING_RATING: f64 = 1500.0;
    
    for team in teams.iter_mut() {
        team.elo = STARTING_RATING;
    }

    map_elo_adjustments(matches, teams);
}

pub fn gen_rank_pure_elo(matches: &Vec<Match>, teams: &mut Vec<Team>, ranking_context: &RankingContext) {
    const STARTING_RATING: f64 = 1500.0;
    
    for team in teams.iter_mut() {
        team.elo = STARTING_RATING;
    }

    elo_adjustments(matches, teams);
}

pub fn gen_rank_org_rounds(matches: &Vec<Match>, events: &Vec<Event>, teams: &mut Vec<Team>, ranking_context: &RankingContext) {
    seed_teams(matches, events, teams, ranking_context);
    round_elo_adjustments(matches, teams);
}

fn round_elo_adjustments(matches: &Vec<Match>, teams: &mut Vec<Team>) {
    for m in matches {
        for map in &m.maps {
            let round_delta = map.team_1_score.abs_diff(map.team_2_score) as f64;

            let map_winning_team = if map.team_1_score > map.team_2_score { m.team_1_id } else { m.team_2_id };
            let map_losing_team  = m.team_1_id + m.team_2_id - map_winning_team;

            let elo_diff = elo_result(teams[map_winning_team].elo, teams[map_losing_team].elo) * round_delta;

            teams[map_winning_team].elo += elo_diff;
            teams[map_losing_team].elo -= elo_diff;
        }
    }        
}