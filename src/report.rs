use crate::*;

pub fn print_to_console(mut teams: Vec<Team>) {
    teams.sort_by(|a, b| b.elo.partial_cmp(&a.elo).unwrap());

    let mut rank = 1;
    for t in teams {
        if !t.ranking_eligible() { continue; }

        println!("{8:3}. {6:20} | Elo {0:6.1} | Diff {7:6.1} | Seed {1:6.1} | PM {2:.3} | OW {3:.3} | EP {4:.3} | ON {5:.3} | $EARNED {9:.0}",
            t.elo,
            t.seed_points,
            t.prize_money,
            t.opponent_winnings,
            t.event_participation,
            t.opponent_network,
            t.name,
            t.elo - t.seed_points,
            rank,
            t.adjusted_winnings,
        );

        rank += 1;
    }
}

pub fn output_report(teams: Vec<Team>) {
    let mut i = 1;
    for t in teams {
        if !t.ranking_eligible() { continue; }

        let players = format!("{}, {}, {}, {}, {}",t.core[0].nick,t.core[1].nick,t.core[2].nick,t.core[3].nick,t.core[4].nick);

        println!("| {0:3}. | {1:20} | {2:6.1} | {3:50} |",
            i,
            t.name,
            t.elo,
            players,
        );

        i += 1;
    }
}
