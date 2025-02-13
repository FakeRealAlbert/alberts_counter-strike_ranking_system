#![allow(dead_code)]

use serde::*;
use std::fs;
use serde_aux::field_attributes::deserialize_number_from_string;
use crate::ranking_context::RankingContext;

// Loads data from JSON file specified with file path.
// Note that Team and Event IDs are converted to refer to their index in both lists. Thus, neither list should ever be sorted
// Match vector is sorted chronologically, which is important for the Elo calculation later on, but in theory, nothing should break
// if it gets sorted some other way. 
pub fn load_data(file_path: String, ranking_context: &RankingContext) -> (Vec<Match>, Vec<Event>, Vec<Team>) {
    let mut events: Vec<Event> = Vec::new();
    let mut matches: Vec<Match> = Vec::new();
    let mut teams: Vec<Team> = Vec::new();

    // Returns struct MatchData with all matches and events.
    let data = fs::read_to_string(file_path).expect("Invalid path!");
    let match_data: MatchData = serde_json::from_str(&data[..]).expect("Failed, man");

    // Add events 
    for i in match_data.events {
        if events.iter().any(|ev| ev.id == i.id) { continue; }
        events.push(Event::new(i));
    }

    // Add matches
    'matches: for i in match_data.matches {
        // Removes if not played in our time frame, or if there were fewer than five players.
        if     i.team_1_players.len() != 5
            || i.team_2_players.len() != 5
            || i.match_start_time < ranking_context.time_window_start
            || i.match_start_time > ranking_context.time_window_end   {continue; }

        let mut m = i.clone();

        'events: for (ev_index, ev) in events.iter_mut().enumerate() {
            if ev.id != m.event_id { continue 'events; }

            // Remove Showmatches (obviously imperfect)
            if ev.name.to_lowercase().contains("showmatch") { continue 'matches; }

            // Updates match's Event reference to be index on the event list
            m.event_id = ev_index;

            // Finds the last match at the event
            ev.last_match_time = u32::max(ev.last_match_time, m.match_start_time);
        }

        // Information context, i.e. factor that decreases for older matches
        m.information_context = ranking_context.time_mod(m.match_start_time);    

        matches.push(m);
    }
    
    // Add "teams" the way VRS defines them, which is based on cores. We need to sort the match feed for the core system to function properly
    matches.sort_by(|a, b| b.match_start_time.cmp(&a.match_start_time));

    for m in &mut matches {
        // Checks if each core is "new", in which case they get added to the list. Returns that cores index in either case.
        let team_one_idx = insert_team(&mut teams, &m.team_1_name, &m.team_1_players);
        let team_two_idx = insert_team(&mut teams, &m.team_2_name, &m.team_2_players);

        // Update event team_id reference
        for pd in &mut events[m.event_id].prize_distribution {
            if m.team_1_id == pd.team_id {
                pd.team_id = team_one_idx;
                pd.is_in_ranking = true;
            }
            if m.team_2_id == pd.team_id {
                pd.team_id = team_two_idx;
                pd.is_in_ranking = false;
            }
        }

        // Set ID to index on match list
        m.winning_team = if m.winning_team == 1 { team_one_idx } else { team_two_idx };
        m.team_1_id = team_one_idx;
        m.team_2_id = team_two_idx;

        // Set matches played and matches won. These are used to filter out teams at the very end
        teams[team_one_idx].matches_played += 1;
        teams[team_two_idx].matches_played += 1;
        teams[m.winning_team].matches_won += 1;
    }


    (matches, events, teams)
}

// Checks if team has a core of another team. If not, adds to team list. Returns index in the team list
pub fn insert_team(teams: &mut Vec<Team>, team_name: &str, team_players: &Vec<Player>) -> usize {
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

    teams.push(Team::new(team_name.to_owned(),[
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
pub struct JsonEvent {
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

// Prize pool is a float because we only ever use it when multiplying with floats
// As an added bonus, we can handle incredibly particular tournament prize pools
#[derive(Debug)]
pub struct Event {
    pub id: usize,
    pub name: String,
    pub prize_pool: f64,
    pub prize_distribution: Vec<PrizeDist>,
    pub is_lan: bool,
    pub last_match_time: u32,
}

impl Event {
    pub fn new(json_event: JsonEvent) -> Self {
        let mut prize_pool = 0.0;
        for prize_moneys in &json_event.prize_distribution {
            prize_pool += prize_moneys.prize;
        }

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
pub struct Match {
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

    pub fn is_in_game(&self, id: usize) -> bool {
        self.team_1_id == id || self.team_2_id == id
    }

    pub fn other_team(&self, id: usize) -> usize {
        debug_assert!(id == self.team_1_id || id == self.team_2_id);

        self.team_1_id + self.team_2_id - id
    }
}

#[derive(Serialize,Deserialize,Debug,Clone,PartialEq)]
pub struct Player {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "playerId"))]
    pub player_id: u16,
    pub nick: String,
    pub country: String,
    #[serde(rename(deserialize = "countryIso"))]
    pub country_iso: String,
}

impl Player {
    pub fn empty() -> Self {
        Self {
            player_id: u16::MAX,
            nick: "".to_string(),
            country: "".to_string(),
            country_iso: "".to_string(),
        }
    }
}

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Map {
    #[serde(rename(deserialize = "mapName"))]
    pub map_name: String,
    #[serde(rename(deserialize = "team1Score"))]
    pub team_1_score: u16,
    #[serde(rename(deserialize = "team2Score"))]
    pub team_2_score: u16,
}

#[derive(Serialize,Deserialize,Debug)]
pub struct PrizeDist {
    pub placement: u32,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    #[serde(rename(deserialize = "teamId"))]
    pub team_id: usize,
    #[serde(default="default_false")]
    pub is_in_ranking: bool,
    pub prize: f64,
    pub shared: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct Team {
    pub name: String,
    pub core: [Player; 5],
    pub event_participation: f64,
    pub opponent_network: f64,
    pub opponent_winnings: f64,
    pub prize_money: f64,

    pub sum_of_factors: f64,
    pub seed_points: f64,
    pub elo: f64,

    pub own_network: f64,
    pub adjusted_winnings: f64,

    pub matches_played: u32,
    pub matches_won: u32,
}

impl Team {
    pub fn new(name: String, core: [Player; 5]) -> Self {
        Self {
            name,
            core,
            event_participation: 0.0,
            opponent_network: 0.0,
            opponent_winnings: 0.0,
            prize_money: 0.0,

            sum_of_factors: 0.0,
            seed_points: 0.0,
            elo: 0.0,

            own_network: 0.0,
            adjusted_winnings: 0.0,

            matches_played: 0,
            matches_won: 0,
        }
    }

    pub fn ranking_eligible(&self) -> bool {
        self.matches_played >= 10 && self.matches_won >= 1 
    }
}

fn default_information_context() -> f64 { 1.0 }
fn default_false() -> bool { false }
fn empty_string() -> String { "".to_string() }