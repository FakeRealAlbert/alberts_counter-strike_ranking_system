#![allow(dead_code,unused_imports,unused)]

mod data_loader;
mod ranking_context;
mod ranking;
mod util;
mod test;
mod report;

use ranking_context::*;
use data_loader::*;
use ranking::*;
use test::*;
use report::*;

/*
    Time Window is just set manually to encompass the first and last games in the sample dataset
    You can adjust the model with RankingContext.
*/ 

fn main() {
    let mut ranking_context = RankingContext::default();
    ranking_context.time_window_end = 1693330518;
    ranking_context.time_window_start = 1693330518 - (6 * 30 * 24 * 60 * 60); // End time minus six months

    let (matches, events, mut teams) = load_data(
        "../data/matchdata_sample_20230829.json".to_string(),
        &ranking_context
    );

    /*
    Note that teams that haven't won a game or have played fewer than 10 are excluded from the ranking, but not from the vector.
    Instead, we use the method team.is_ranking_eligible() to filter them out after the fact.
    Importantly, analyze_fit does not filter out teams that aren't ranking eligible.
    */

    gen_rank_new(&matches, &events, &mut teams, &ranking_context);

    output_report(&teams, &ranking_context);
}
