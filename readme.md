# Modified VRS
This project tries to improve on Valve's current Ranking System. It's both significantly simpler than the original, and performs significantly better. Errors for both models are calculated by comparing actual versus expected win rate for each game. VRS' has an error of 5%, while my Alternative VRS has an error of 1%.

### Original VRS
EWR Bucket 0.00 | Matches played:   521 | Win rate: 0.17 | ExWR: 0.05
EWR Bucket 0.10 | Matches played:   718 | Win rate: 0.20 | ExWR: 0.15
EWR Bucket 0.20 | Matches played:   792 | Win rate: 0.28 | ExWR: 0.25
EWR Bucket 0.30 | Matches played:   818 | Win rate: 0.30 | ExWR: 0.35
EWR Bucket 0.40 | Matches played:   917 | Win rate: 0.42 | ExWR: 0.45
Total error from this model: 0.04955927052649679

### Alternative VRS
EWR Bucket 0.00 | Matches played:   526 | Win rate: 0.07 | ExWR: 0.05
EWR Bucket 0.10 | Matches played:   613 | Win rate: 0.15 | ExWR: 0.15
EWR Bucket 0.20 | Matches played:   774 | Win rate: 0.28 | ExWR: 0.25
EWR Bucket 0.30 | Matches played:   878 | Win rate: 0.35 | ExWR: 0.35
EWR Bucket 0.40 | Matches played:   975 | Win rate: 0.44 | ExWR: 0.45
Total error from this model: 0.010816920590655634

## Differences between old VRS and my version
The four factor system is ported over from VRS, but each is modified, the idea being that:
* Event Participation rewards playing big events.
* Prize Money         rewards playing good at big events.
* Opponent Network    rewards beating many opponents.
* Opponent Winnings   rewards beating good opponents.

I've simplified the system where it didn't negatively effect performance. Current discourse around VRS shows very few people genuinely understand it, even people who claim to know what they're talking about. The goal is for teams to be able to understand why they're winning or losing points in the system.

The biggest improvement you'd be looking to make is the curve functions. It currently applies:
* Opp Net and Opp Win: Curve function at the end.
* Prize Money: Square root under aggregation.
* Event Participation: Log_10 under aggregation.
I adjusted those to get small errors, but there's absolutely no logic behind any of them. Ideally, we'd look under the hood to get some sense of how we'd want to actually adjust these variables.

## Detailed.
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