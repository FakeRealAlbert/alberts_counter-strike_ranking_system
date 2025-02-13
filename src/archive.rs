// This is the original VRS script, adapted for Rust. It's included for the curious reader

#![allow(dead_code)]

use serde::*;
use std::fs;
use serde_aux::field_attributes::deserialize_number_from_string;
use std::ops::RangeInclusive;

pub fn archive_main() {
    let mut ranking_context = RankingContext::default();
    ranking_context.time_window_end = 1693330518;
    ranking_context.time_window_start = 1693330518 - (6 * 30 * 24 * 60 * 60); // End time minus six months

    let (matches, events, mut teams) = load_data(
        "./data/matchdata_sample_20230829.json".to_string(), 
        &ranking_context
    );

    seed_teams(&matches, &events, &mut teams, &ranking_context);
        
    elo_adjustments(&matches, &mut teams);

    print_to_console(teams);
}

// Loads data from path and cleans everything up. Returns vecs of matches, events and teams.
fn load_data(file_path: String, ranking_context: &RankingContext) -> (Vec<Match>, Vec<Event>, Vec<Team>) {
    let mut events: Vec<Event> = Vec::new();
    let mut matches: Vec<Match> = Vec::new();
    let mut teams = Vec::new();


    // Returns struct MatchData with all matches and events.
    let data = fs::read_to_string(file_path).expect("Invalid path!");
    let match_data: MatchData = serde_json::from_str(&data[..]).expect("Failed, man");

    // Adds events, skipping over all duplicates
    for i in match_data.events {
        if events.iter().any(|ev| ev.id == i.id) { continue; }
        events.push(Event::new(i));
    }

    // Filters matches
    'matches: for i in match_data.matches {
        // Removes if not played in our time frame, or if there were fewer than five players.
        if     i.team_1_players.len() != 5
            || i.team_2_players.len() != 5
            || i.match_start_time < ranking_context.time_window_start
            || i.match_start_time > ranking_context.time_window_end   {continue; }

        let mut m = i.clone();

        'events: for (ev_index, ev) in events.iter_mut().enumerate() {
            if ev.id != m.event_id { continue 'events; }
                
            // Updates match's Event reference to be index on the event list
            m.event_id = ev_index;

            // Finds the last match at the event
            ev.last_match_time = u32::max(ev.last_match_time, m.match_start_time);

            // Remove Showmatches
            if ev.name.to_lowercase().contains("showmatch") { continue 'matches; }
        }

        // Information context, i.e. it falls off as time goes by
        m.information_context = ranking_context.time_mod(m.match_start_time);    

        matches.push(m);
    }
    
    // Sorts matches by date
    matches.sort_by(|a, b| b.match_start_time.cmp(&a.match_start_time));

    for m in &mut matches {
        // Inserts team
        let team_one_idx = insert_team(&mut teams, &m.team_1_name, &m.team_1_players);
        let team_two_idx = insert_team(&mut teams, &m.team_2_name, &m.team_2_players);

        register_event_participation(&mut teams, &events, m.event_id, m.team_1_id, team_one_idx);
        register_event_participation(&mut teams, &events, m.event_id, m.team_2_id, team_two_idx);

        // Sets ID to index on match list
        m.winning_team = if m.winning_team == 1 { team_one_idx } else { team_two_idx };
        m.team_1_id = team_one_idx;
        m.team_2_id = team_two_idx;

        // Sets matches played and matches won. These are used to filter out teams at the very end
        teams[team_one_idx].matches_played += 1;
        teams[team_two_idx].matches_played += 1;
        teams[m.winning_team].matches_won += 1;
    }

    // Sorts from latest to earliest.
    matches.reverse();

    (matches, events, teams)
}

fn seed_teams(matches: &Vec<Match>, events: &Vec<Event>, teams: &mut Vec<Team>, ranking_context: &RankingContext) {
    // It's a surprise tool that will help us later!
    let mut all_winnings = Vec::new();
    let mut all_networks = Vec::new();
    let mut all_lan_wins = Vec::new();
    
    for (idx, team) in teams.iter_mut().enumerate() {
        let mut lan_wins = Vec::new();
        let mut opponents : Vec<(usize, f64)> = Vec::new();

        // FaZe 1: Calculates LAN Wins, Own Network and Bounty Offered
        for m in matches {
            if m.team_1_id != idx && m.team_2_id != idx { continue; } // Not our game

            let opp_id = m.team_1_id + m.team_2_id - idx;
            
            if m.winning_team == idx { 

                // Adds or updates opponent list. We want the latest game we played against each opponent
                // (note matches were sorted earlier. See load_data)
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

                // If LAN Win, add to LAN Win
                if events[m.event_id].is_lan { lan_wins.push(m.information_context); }
            };
        }

        // OWN NETWORK. Each team we've beaten scaled by how long ago that was.
        for op in opponents {
            team.own_network += op.1;
        }

        // LAN WINS. Note that since LAN Wins is only affected by time since game, and they were added from a list sorted from earliest to latest
        //           We only need to reverse the list to get our X best results
        lan_wins.reverse();
        lan_wins.resize(ranking_context.factor_bucket_size, 0.0);
        team.lan_wins = sum_vector(lan_wins);

        // BOUNTY OFFERED. Find our 10 best prize moneys.
        let mut bounties_offered = Vec::new();
        for (ev_id, prize_money) in team.prize_money.clone() {
            bounties_offered.push(prize_money as f64 * ranking_context.time_mod(events[ev_id].last_match_time));
        }
 
        // partial_cmp because floats can't be sorted perfectly.
        bounties_offered.sort_by(|a, b| b.partial_cmp(a).unwrap());
        bounties_offered.resize(ranking_context.factor_bucket_size, 0.0);
        team.adjusted_winnings = sum_vector(bounties_offered);

        all_winnings.push( team.adjusted_winnings );
        all_networks.push( team.own_network );
        all_lan_wins.push( team.lan_wins );
    }

    all_winnings.sort_by(|a, b| b.partial_cmp(a).unwrap());
    all_networks.sort_by(|a, b| b.partial_cmp(a).unwrap());
    all_lan_wins.sort_by(|a, b| b.partial_cmp(a).unwrap());

    let reference_winnings = all_winnings[ranking_context.top_outlier_count - 1];
    let reference_networks = all_networks[ranking_context.top_outlier_count - 1];
    let reference_lan_wins = all_lan_wins[ranking_context.top_outlier_count - 1];
    
    // Uncurved bounty offered is unnecessary, but interesting
    for t in teams.iter_mut() {
        t.uncurved_bounty_offered = f64::min(t.adjusted_winnings / reference_winnings,1.0);
        t.bounty_offered          = curve_function(t.uncurved_bounty_offered);
        t.own_network             = f64::min(t.own_network / reference_networks,1.0);
        t.lan_wins                = f64::min(t.lan_wins / reference_lan_wins,1.0);
    }


    // FaZe 2: Go through matches again to calculate Bounty Collected and Opp Net
    let mut bounty_collected = Vec::new();
    let mut opponent_network = Vec::new();

    for idx in 0..teams.len() {
        let mut bounties = Vec::new();
        let mut networks = Vec::new();

        for m in matches {
            if m.winning_team != idx { continue; }

            let opp_idx = m.team_1_id + m.team_2_id - idx;

            // These two are scaled by event prize pool as well.
            let event_prize_pool = f64::max(events[m.event_id].prize_pool, 1.0);
            let scaling = m.information_context * curve_function(f64::min(event_prize_pool / ranking_context.max_prize_pool_mod, 1.0));

            bounties.push(teams[opp_idx].uncurved_bounty_offered * scaling);
            networks.push(teams[opp_idx].own_network * scaling);   
        }

        bounties.sort_by(|a, b| b.partial_cmp(a).unwrap());
        bounties.resize(ranking_context.factor_bucket_size,0.0);
        bounty_collected.push( curve_function(sum_vector(bounties ) / ranking_context.factor_bucket_size as f64 ));

        networks.sort_by(|a, b| b.partial_cmp(a).unwrap());
        networks.resize(ranking_context.factor_bucket_size,0.0);
        opponent_network.push( sum_vector(networks) / ranking_context.factor_bucket_size as f64 );
    }

    // FaZe 3: We can finally calculate Starting Rating
    let mut highest_sum_of_factors = f64::MIN;
    let mut lowest_sum_of_factors = f64::MAX;
    for (idx, team) in teams.iter_mut().enumerate() {
        team.bounty_collected = bounty_collected[idx];
        team.opp_network      = opponent_network[idx];

        team.sum_of_factors = team.bounty_collected * ranking_context.bounty_collected_weight
                            + team.bounty_offered   * ranking_context.bounty_offered_weight
                            + team.opp_network      * ranking_context.opponent_network_weight
                            + team.own_network      * ranking_context.own_network_weight
                            + team.lan_wins         * ranking_context.lan_factor_weight;
        highest_sum_of_factors = f64::max(highest_sum_of_factors, team.sum_of_factors);
        lowest_sum_of_factors  = f64::min(lowest_sum_of_factors,  team.sum_of_factors);
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

fn elo_adjustments(matches: &Vec<Match>, teams: &mut Vec<Team>) {
    for m in matches {
        let elo_diff: f64 = elo_result(teams[m.winning_team].elo, teams[m.losing_team_id()].elo, m.information_context);

        teams[m.winning_team].elo += elo_diff;
        teams[m.losing_team_id()].elo -= elo_diff;
    }
}

// Old constants are featured for clarity's sake. Final version simplifies it.
// They achieve essentially the same results (though probably not exactly the same since we're dealing with floats)
fn elo_result(winner_elo: f64, loser_elo: f64, info: f64) -> f64 {
    // let q: f64 = f64::ln(10.0) / 400.0; 
    // let g = 1.0 / f64::sqrt(1.0 + ( q * q * 16875.0 * 1.0 / (3.14159265359 * 3.14159265359) ) );
    // const Q: f64 = 0.005756462732485115;
    // const G: f64 = 0.9728209910486931;
    // let ev = 1.0 / ( 1.0 + f64::powf(10.0, G * ( loser_elo - winner_elo ) / (400.0)) );
    // Q * G * (1.0 - ev) * info / ( ( 1.0 / 5625.0 ) + Q * Q * G * G * ev * (1.0 - ev ) * info * info)

    let ev = 1.0 / ( 1.0 + f64::powf(1.0056157171343996, loser_elo - winner_elo) );
    31.500043764474583 * (1.0 - ev) * info / ( 1.0 + 0.17640049016245588 * ev * (1.0 - ev ) * info * info)
}

fn register_event_participation(teams: &mut Vec<Team>, events: &Vec<Event>, event_id: usize, team_id: usize, team_idx: usize) {    
    // We've already added this event.
    if teams[team_idx].prize_money.iter().any(|pm| pm.0 == event_id) { return; }

    for pd in &events[event_id].prize_distribution {
        if pd.team_id == team_id {
            teams[team_idx].prize_money.push( (event_id, pd.prize) );
            return;
        };
    }
}

// Checks if team has a core of another team. If not, adds to team list. Returns index in the team list
fn insert_team(teams: &mut Vec<Team>, team_name: &String, team_players: &Vec<Player>) -> usize {
    for (idx, t) in teams.iter().enumerate() {
        let mut similarity = 0;
        for p1 in &t.core {
            for p2 in team_players {
                if p1.player_id == p2.player_id { similarity += 1; }
            }

            // Same team for our purposes
            if similarity >= 3 {
                return idx;
            }
        }
    };

    teams.push(Team::new(team_name.clone(),[
        team_players[0].clone(),
        team_players[1].clone(),
        team_players[2].clone(),
        team_players[3].clone(),
        team_players[4].clone(),
    ]));

    teams.len() - 1
}

// No clue in retrospect why this is a separate struct, but it doesn't really matter
#[derive(Serialize,Deserialize,Debug)]
struct JsonEvent {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "eventId"))]
    pub id: usize,
    #[serde(rename(deserialize = "eventName"))]
    pub name: String,
    #[serde(rename(deserialize = "prizePool"),default = "empty_string")]
    pub prize_pool: String,
    #[serde(rename(deserialize = "lan"))]
    pub is_lan: bool,
    #[serde(rename(deserialize = "prizeDistribution"))]
    pub prize_distribution: Vec<PrizeDist>,
}

#[derive(Debug)]
struct Event {
    pub id: usize,
    pub name: String,
    pub prize_pool: f64,
    pub prize_distribution: Vec<PrizeDist>,
    pub is_lan: bool,
    pub last_match_time: u32,
}

impl Event {
    fn new(json_event: JsonEvent) -> Self {
        let parsed_prize_pool = json_event.prize_pool.replace("$","").replace(",","").parse::<f64>();
        
        let prize_pool = match parsed_prize_pool {
            Ok(p) => p,
            Err(_) => 0.0,
        };

        Self {
            id: json_event.id,
            name: json_event.name,
            prize_pool,
            prize_distribution: json_event.prize_distribution,
            is_lan: json_event.is_lan,
            last_match_time: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MatchData {
    pub matches: Vec<Match>,
    pub events: Vec<JsonEvent>,
}

#[derive(Serialize,Deserialize,Debug,Clone)]
struct Match {
    #[serde(rename(deserialize = "matchStartTime"))]
    pub match_start_time: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "team1Id"))]
    pub team_1_id: usize,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "team2Id"))]
    pub team_2_id: usize,
    #[serde(rename(deserialize = "team1Name"))]
    pub team_1_name: String,
    #[serde(rename(deserialize = "team2Name"))]
    pub team_2_name: String,
    #[serde(rename(deserialize = "team1Players"))]
    pub team_1_players: Vec<Player>,
    #[serde(rename(deserialize = "team2Players"))]
    pub team_2_players: Vec<Player>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "eventId"))]
    pub event_id: usize,
    pub maps: Vec<Map>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "winningTeam"))]
    pub winning_team: usize,
    #[serde(default = "default_information_context")] 
    pub information_context: f64,
}

impl Match {
    pub fn losing_team_id(&self) -> usize {
        self.team_1_id + self.team_2_id - self.winning_team
    }
}

#[derive(Serialize,Deserialize,Debug,Clone,PartialEq)]
struct Player {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "playerId"))]
    pub player_id: u16,
    pub nick: String,
    pub country: String,
    #[serde(rename(deserialize = "countryIso"))]
    pub country_iso: String,
}

impl Player {
    fn empty() -> Self {
        Self {
            player_id: u16::MAX,
            nick: "".to_string(),
            country: "".to_string(),
            country_iso: "".to_string(),
        }
    }
}

#[derive(Serialize,Deserialize,Debug,Clone)]
struct Map {
    #[serde(rename(deserialize = "mapName"))]
    pub map_name: String,
    #[serde(rename(deserialize = "team1Score"))]
    pub team_1_score: u16,
    #[serde(rename(deserialize = "team2Score"))]
    pub team_2_score: u16,
}

#[derive(Serialize,Deserialize,Debug)]
struct PrizeDist {
    pub placement: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "teamId"))]
    pub team_id: usize,
    pub prize: u32,
    pub shared: bool,
}

#[derive(Debug, Serialize, Clone)]
struct Team {
    pub name: String,
    pub core: [Player; 5],
    pub lan_wins: f64,
    pub opp_network: f64,
    pub bounty_collected: f64,
    pub bounty_offered: f64,

    pub uncurved_bounty_offered: f64,
    pub uncurved_bounty_collected: f64,

    pub sum_of_factors: f64,
    pub seed_points: f64,
    pub elo: f64,

    pub own_network: f64,
    pub adjusted_winnings: f64,

    pub matches_played: u32,
    pub matches_won: u32,

    pub prize_money: Vec<(usize, u32)>,
}

impl Team {
    fn new(name: String, core: [Player; 5]) -> Self {
        Self {
            name,
            core,
            lan_wins: 0.0,
            opp_network: 0.0,
            bounty_collected: 0.0,
            bounty_offered: 0.0,

            uncurved_bounty_offered: 0.0,
            uncurved_bounty_collected: 0.0,

            sum_of_factors: 0.0,
            seed_points: 0.0,
            elo: 0.0,

            own_network: 0.0,
            adjusted_winnings: 0.0,

            matches_played: 0,
            matches_won: 0,

            prize_money: Vec::new(),
        }
    }

    fn ranking_eligible(&self) -> bool {
        self.matches_played >= 10 && self.matches_won >= 1 
    }
}

fn default_information_context() -> f64 { 1.0 }
fn empty_string() -> String { "".to_string() }

fn curve_function(x: f64) -> f64 {
    1.0 / (1.0 + f64::abs(f64::log10(x)))
}

fn sum_vector(vec: Vec<f64>) -> f64 {
    let mut sum = 0.0;
    for i in vec { sum += i }
    sum
}

fn remap_value_clamped(val: f64, in_low: f64, in_high: f64, out_low: f64, out_high: f64) -> f64 {
    let interpolated = (val - in_low ) / (in_high - in_low);
    let clamped = interpolated.clamp(0.0, 1.0);

    clamped * out_high + ( 1.0 - clamped ) * out_low
}

#[derive(Debug)]
struct RankingContext {
    pub top_outlier_count: usize,
    pub factor_bucket_size: usize,

    pub time_window_start: u32,
    pub time_window_end: u32,
    pub time_grace_period: u32,
    pub time_decay_factor: f64,     // The power of decay. 1 is linear

    pub max_prize_pool_mod: f64,

    pub bounty_collected_weight: f64,
    pub bounty_offered_weight: f64,
    pub opponent_network_weight: f64,
    pub own_network_weight: f64,
    pub lan_factor_weight: f64,

    pub min_seeded_rank: f64,
    pub max_seeded_rank: f64,
}

impl RankingContext {
    fn default() -> Self {
        Self {
            top_outlier_count: 5,
            factor_bucket_size: 10,

            time_window_start: u32::MIN,
            time_window_end: u32::MAX,
            time_grace_period: 30 * 24 * 60 * 60, // One month
            time_decay_factor: 1.0,

            max_prize_pool_mod: 1_000_000.0,

            bounty_collected_weight: 1.0,
            bounty_offered_weight: 1.0,
            opponent_network_weight: 1.0,
            own_network_weight: 0.0,
            lan_factor_weight: 1.0,

            min_seeded_rank: 400.0,
            max_seeded_rank: 2000.0,
        }
    }

    fn time_mod(&self, time: u32) -> f64 {
        let above = time.clamp(self.time_window_start, self.time_window_end - self.time_grace_period) - self.time_window_start;
        let below = self.time_window_end - self.time_window_start - self.time_grace_period;

        ((above as f64) / (below as f64)).powf(self.time_decay_factor) 
    }
}

pub fn print_to_console(mut teams: Vec<Team>) {
    teams.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap());

    let mut rank = 1;
    for t in teams {
        if !t.ranking_eligible() { continue; }

        println!("{8:3}. {6:20} | Elo {0:6.1} | Diff {7:6.1} | Seed {1:6.1} | BO {2:.3} | BC {3:.3} | LF {4:.3} | ON {5:.3} | $EARNED {9:.0}",
            t.elo,
            t.seed_points,
            t.bounty_offered,
            t.bounty_collected,
            t.lan_wins,
            t.opp_network,
            t.name,
            t.elo - t.seed_points,
            rank,
            t.adjusted_winnings,
        );

        rank += 1;
    }
}