# Alternative VRS

This is an alternative version of Valve's Regional Standings. My goal was two-fold: To make it more accurate, harder to manipulate, and easier to understand. 

As section 3 shows, it significantly outperforms VRS on accuracy. For the other two goals, I hope the explanation is of service. You can also see the source code in the "src" folder, and you can experiment with the source code yourself: The ranking_context includes about a dozen variables that significantly changes the rating system. I've written it in Rust. The original uses JavaScript, but I've no idea how to use that language.

If you have any questions about the project, feel free to send me an email at <mail@albertengan.no>. I'm also at twitter, @FakeRealAlbert, but I probably won't see any messages over there.

## 1. How the Alternative VRS works

The basic structure is similar to VRS: We use four factors to calculate a starting rating and then make a Head-to-Head adjustment. The difference is in the details:

The system's four factors are Price Winnings, Event Participation, Opponent Network, Opponent Winnings. In short, Opponent Network rewards beating many teams, Opponent Winnings rewards beating good teams, Event participation rewards playing many good events, and Price Winnings rewards playing good at many good events.

In long:

1. **Price Winnings**: For every event the team has played in, we add the *square root* of the team's prize winnings. Older events count for less.

2. **Event Participation**: For every event the team has played in, we add the *logarithm base 10* of the total event prize pool. Older events count for less.

3. **Opponent Winnings**: For every opponent the team has beaten, we consider their Price Winnings. Older results count for less. We sum up the ten highest scores.

4. **Opponent Network**: For every opponent the team has beaten, we consider their Network, i.e. how many distinct teams they have beaten in the past 6 months, scaled by how long ago they beat them. Older results count for less. We sum up the ten highest scores.

Every factor is then scaled so that the fifth best team automatically scores 1.000 on the metric and the worst team scores 0.000.

The Head-to-Head adjustment is a basic Elo system, going through every map in every match in the past six months. Here we assume that a 400 rank differential gives the high ranked team a 90% chance of winning the game.

## 2. Differences between AVRS and VRS

The biggest improvement is that the Head-to-Head adjustment considers every *map* the team has played, not every *match*. In other words, if you win a game 2-0 rather than 2-1, you will earn more points. This gives the algorithm way more data to work with, and massively increases its precision for teams that have played very few games. 

The second largest improvement is the removal of LAN Wins. That factor rewarded teams almost arbitrarily, since it only rewarded LAN play and didn't care about the level of competition. It was replaced with Event Participation: This is featured indirectly in VRS, since Opponent Network and Bounty Collected there are scaled by the event prize pool. Alternative VRS spins it off into its own factor, which has the side effect of making those two factors more intuitive. Lots of teams scored pooorly on Opponent Network because while they played lots of teams, they only did so in qualifiers without an event prize pool. The first stage of the Major, for example, has no event prize pool, and therefore improves neither factor.

It has been pointed out before that Opponent Network correlates poorly with a team's chance of victory. I still included it, also increased its performance by scaling the best result to 1.000 (usually, the best team only scores .5 on the VRS), because it makes the system harder to manipulate. We need teams to play lots of games so that the H2H-adjustment can move them closer to their "actual" rating: VRS generally only breaks down with teams who play few games. By encouraging teams to play more games and more tournaments (with Opponent Network and Event Participation) we are actually just punishing the type of teams that VRS tend to overrate. This, for example, means it's very unlikely that teams should ever skip out on tournaments to maintain their rating.

For a more detailed overview of the differences between the two, see the last section.

## 3. Performance 

Note that the test tests uses the matchdata sample from 2023, which is the only good datasource we have at the moment.

The performance is calculated as such: We divide each match into five groups based on what we think are the chances of the lowest team in that match up. We then compare the average *actual* win rate of that team compared to the models *expected* average win rate. You can see a breakdown for each bucket in the two tables. The "average" error is just the average for each bucket, weighed based on how many teams are in that bucket.

In summary, then, this basic test tells us that we should expect a normal team to be about 5% off their "actual" rating in VRS, and 1% off in AVRS. Also note that VRS performes significantly worse with very large rank differentials. AVRS is much more consistent.

### Original VRS

| EWR Bucket | Matches Played | Win Rate | Expected Win Rate |
|--|--|--|--|
| 0% | 521 | 17% | 5% | 
| 10% | 718 | 20% | 15% |
| 20% | 792 | 28% | 25% |
| 30% | 818 | 30% | 35% |
| 40% | 917 | 42% | 45% |

Average error from this model: 4.956%

### Alternative VRS
| EWR Bucket | Matches Played | Win Rate | Expected Win Rate |
|--|--|--|--|
| 0% | 526 | 7% | 5% | 
| 10% | 613 | 15% | 15% |
| 20% | 774 | 28% | 25% |
| 30% | 878 | 35% | 35% |
| 40% | 975 | 44% | 45% |

Average error from this model: 1.082%

## 4. Granular differences

1. Event prize pool is calculated from the sum of the prize distribution, not the HLTV description.

2. All four factors, and Own Network, are scaled by the 5th best result. This makes Opponent Winnings and Opponent Network more important, while making the whole system more intuitive.

3. Removed "10 best results" cap for Event Participation and Prize Money, which could lead to tournaments becoming less valuable if they were arbitrarily made multi-stage by HLTV. Additionally, while teams shouldn't be able to grind opponents (Which they could if we removed the cap on Opp. Network and Opp. Winnings), there's nothing wrong with their grinding tournaments.

#### LAN Wins
LAN Wins completely removed. Event Participation is included instead, simply based on prize pool at events played at. Attempts at including LAN in this calculation only increased error: Below tier one, they're too arbitrary to reward.

#### Bounty Offered
1. "Bounty Offered" renamed to "Prize Money". Bounty Offered/Collected gives the impression that something is lost or taken. Prize Money gives a better intuition of what it's actually measuring.

2. We use the Square Root instead of the Curve Function. This punishes low-prize-money-teams more heavily and is easier to understand. Lowers error by just a bit.

#### Bounty Collected and Opponent Network
Removed Event Weight for both, which is spun off into Event Participation instead. Previously both factors were often measuring event participation at least as much as what they were initially intended for, so this change makes each factor more inuitive. It also gives more weight to qualifiers and the like, which seems only fair.

#### H2H adjustment.
1. ELO adjustment is calculated on a map-by-map-basis. This is the most important change by far. It single-handedly decreased model error by 2/3.

2. Replaced with a basic Elo equation. The current equation actually performs worse than the original one, but the difference is pretty minor. VRS just used a glicko algorithm without ratings deviation, and that is essentially an Elo algorithm.

3. Currently K = 32, which performed best in tests. It is set so that a rank differential of 400 implies a 90% chance of victory, like in old VRS.

3. Information context removed from Elo results, which simplifies the equation some more. Elo naturally makes older results less relevant anyway, and I believe the old system was slightly too reactive.
