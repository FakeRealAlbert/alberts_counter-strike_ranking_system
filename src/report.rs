use crate::*;

pub fn output_report(teams: &[Team]) {
    for (i, t) in teams.iter().enumerate() {
        let players = format!("{}, {}, {}, {}, {}",t.players[0].nick,t.players[1].nick,t.players[2].nick,t.players[3].nick,t.players[4].nick);

        println!("|{0:3}. | {1:20} | {2:6.1} | {3:80}",
            i,
            t.name,
            t.elo,
            t.players,
        )
    }
}
