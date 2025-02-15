# Modified VRS
This project tries to improve on Valve's current Ranking System. It's both significantly simpler than the original, and performs significantly better. Errors for both models are calculated by comparing actual versus expected win rate for each game. VRS has an error of 5%, while my Alternative VRS has an error of 1%.

It's written in Rust for no particular reason. One file shows an implementation of the old VRS in Rust.

If you have any questions, you can send me an email at mail@albertengan.no. 

### Original VRS

| EWR Bucket | Matches Played | Win Rate | Expected Win Rate |
|--|--|--|--|
| 0.00 | 521 | 0.17 | 0.05 | 
| 0.10 | 718 | 0.20 | 0.15 |
| 0.20 | 792 | 0.28 | 0.25 |
| 0.30 | 818 | 0.30 | 0.35 |
| 0.40 | 917 | 0.42 | 0.45 |

Total error from this model: 0.04955927052649679

### Alternative VRS
| EWR Bucket | Matches Played | Win Rate | Expected Win Rate |
|--|--|--|--|
| 0.00 | 526 | 0.07 | 0.05 | 
| 0.10 | 613 | 0.15 | 0.15 |
| 0.20 | 774 | 0.28 | 0.25 |
| 0.30 | 878 | 0.35 | 0.35 |
| 0.40 | 975 | 0.44 | 0.45 |

Total error from this model: 0.010816920590655634

## Differences between old VRS and my version
The four factor system is ported over from VRS, but each is modified, the idea being that:
* Event Participation rewards playing big events.
* Prize Money rewards playing good at big events.
* Opponent Network rewards beating many opponents.
* Opponent Winnings rewards beating good opponents.

The Elo adjustment now looks at map and not match wins, which represents the biggest improvoment in error. Outside of performance, I've focused on simplifying the system by moving and removing variables, names and code almost everywhere in the system. 

## Possible Improvements
The curve functions are the most arbitrary part of the system, currently:
* Opp Net and Opp Win: One over log_10 curve function at the end.
* Prize Money: Square root under aggregation.
* Event Participation: Log_10 under aggregation.

These were all set to minimise error, but there's absolutely no logic behind any of them, and so it's hard to say what they're doing under the hood.

Additionally, the system could be taking round wins into account, but that would require some more complicated math in the elo system (and you might be getting diminishing returns for the extra complexity).

## Detailed Differences.
1. Event prize pool is calculated from the sum of the prize distribution, meaning qualifier events can now have prize pools. (Presumably just a bug in the original)

2. All four factors, and Own Network, are scaled by the 5th best result. This makes Opp. Winnings and Opp. Network more important, while making the whole system more intuitive.

3. Removed "10 best results" cap for Event Participation and Prize Money, which could lead to tournaments becoming less valuable if they were arbitrarily made multi-stage by HLTV. Additionally, while teams shouldn't be able to grind opponents (Which they could if we removed the cap on Opp. Network and Opp. Winnings), there's nothing wrong with their grinding tournaments.

4. RankingContext includes essentially all variables one would want to change, which makes the system easier to adjust.

#### LAN Wins
5. LAN Wins completely removed. Event Participation is included instead, simply based on prize pool at events played at. Attempts at including LAN in this calculation only increased error: Below tier one, they're too arbitrary to reward.

#### Bounty Offered
6. "Bounty Offered" renamed to "Prize Money". Bounty Offered/Collected gives the impression that something is lost or taken. Prize Money gives a better intuition of what it's actually measuring.

7. Square Root instead of the Curve Function as a final adjustment. This punishes low-prize-money-teams more heavily and is easier to understand. Lowers error by just a bit.

#### Bounty Collected and Opponent Network
8. Removed Event Weight for both, which is spun off into Event Participation instead. Previously both factors were often measuring event participation at least as much as what they were initially intended for, so this change makes each factor more inuitive. It also gives more weight to qualifiers and the like, which seems only fair.

#### H2H adjustment.
9. Completely ripped out and replaced with a basic Elo equation. The current equation actually performs worse than the original one, but it's far clearer, so I believe it's worth it.

10. Elo is based on two main variables. A rank difference of 400 implies a 90% chance of victory (like in old VRS). K-constant defines how reactive Elo adjustments should be. Currently K = 32, which performed best in tests.

11. ELO adjustment is calculated on a map-by-map-basis. This is the most important change by far. It single-handedly decreased model error by 2/3.

12. Information context removed from Elo results, which simplifies the equation some more. Elo naturally makes older results less relevant anyway, and I believe the old system was slightly too reactive.
