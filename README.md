# mindcrank

> Monte Carlo simulations for Magic the Gathering

## What is a Monte Carlo simulation

Monte Carlo simulations are a way to generate a forecast - _a set of predicted scenarios with an associated probability of occurring_. Forecasts crucially need both. Monte Carlo simulations are a powerful way to model a wide spectrum of phenomena, from estimating `pi` or `e`, the natural number, to weather predictions and fission energy production. In fact, the Monte Carlo method was developed by Neumann and associated at Los Alamos during research on the Manhattan Project to simulate atomic collisions in fission reactions.

A monte carlo simulation works by assigning multiple values to a single variable to achieve multiple results and then averaging the results to obtain an estimate of the given outcome. So, for example, if we wanted to model the rate of land draws on a variety of land compositions, we could do that, and then weight the following outputs to determine a most-likely outcome based on each given land composition. Or we could model the likelihood of drawing into a combo with `n` number of cards.

This is all necessary because Magic is a _[stochastic process](https://en.wikipedia.org/wiki/Stochastic_process)_. You can’t know how a game will turn out until you’ve played it through even with perfect knowledge of all game state, which you don’t normally have anyway. Because of the random order of cards, the choices of players, and in-game sources of randomness like coin flips, each game is a fingerprint of a set of decks in a given sequence of time.

## Setup

To start, I setup a Monte Carlo simulation that modeled a 2 card win condition (let’s say it’s our bff Thoracle/Consult) in the 99 of a deck with 37 land cards.

Simulations had an average turn count at 27 after drawing the initial 7 cards, meaning that for a 2 card combo, you have to dig down an average of 27 cards after your opening hand.

```sh
☁  mtg-sim [main] ⚡  go run main.go
mtg-sim booting up
### results: {10000 27.5876}
☁  mtg-sim [main] ⚡  go run main.go
mtg-sim booting up
### results: {10000 27.6021}
```

Cranking the sample space up to 10,000,000 produces no significant difference in the numbers.

```go
☁  mtg-sim [main] ⚡  go run main.go
mtg-sim booting up
### results: {10000000 27.5382594}
```

## Opening hand wins

On this second set of runs, I added the ability to detect opening hand wins. The results are consistent - there’s about a 0.31% chance (a third of a 1%!) that you draw your two-card combo in your opening hand. Bumping the runs up to 10,000,000 this time _actually did_ effect our numbers. The variation at 10,000 runs was a lot higher, but over 10,000,000 it smoothed out to the number above. Monte Carlo simulations are known for being sensitive to the number of scenario runs, but it’s always interesting to actually see it happen.

```sh
☁  mtg-sim [main] ⚡  go run main.go
📊 results:
{attempts:10000000 averageDrawsToWin:27.6532424 openingHandWins:31032 averageOpeningHandWins:0.0031032}

☁  mtg-sim [main] ⚡  go run main.go
📊 results:
{attempts:10000000 averageDrawsToWin:27.6540736 openingHandWins:31077 averageOpeningHandWins:0.0031077}

☁  mtg-sim [main] ⚡  go run main.go
📊 results:
{attempts:10000000 averageDrawsToWin:27.6540033 openingHandWins:30667 averageOpeningHandWins:0.0030667}
```

Ya, I jazzed up the log lines. _The hood gonna love it._

This set of runs has 4 combo pieces in it but kept the same concentration of lands and the same 2 cards required to solidify a win.

```sh
[main] ⚡  go run main.go 
📊 results:
{attempts:10000000 averageDrawsToWin:23.3397672
openingHandWins:175493 averageOpeningHandWins:0.0175493}
[main] ⚡  go run main.go
📊 results:
{attempts:10000000 averageDrawsToWin:23.3396951 openingHandWins:175921 averageOpeningHandWins:0.0175921}
[main] ⚡  go run main.go
📊 results:
{attempts:10000000 averageDrawsToWin:23.3407488 openingHandWins:175475 averageOpeningHandWins:0.0175475}
```

## Limitations and configuration

The simulator currently only distinguishes between lands and non lands, and only non lands can be combo pieces. There’s a configurable number of required combo pieces, number of lands per deck, and number of combo pieces included. The simulator also doesn’t care about lands being drawn, only combo pieces. Instead, the land/non-land distinction has been added for future sampling of opening hands and land drop curves.

## Synthesis

Turning the number up of combo pieces up to 4 but with only 2 still required decreased the average draw count down to 22, an ~18% reduction in draws. This means that adding two additional combo cards to your existing combo lines is the same as adding approximately 5 cards worth  of draw. This is a surprising result to me, since it suggests that adding redundant combo pieces to your deck is less beneficial than it might first seem. The analysis doesn’t account for whether or not the cards are in the proper zone, or even if the cards are the right combination of the combo pieces, but it does offer insight into the raw likelihoods of drawing them in the first place.

When looking at opening hand wins, the likelihood of drawing your combo in your opening hand is about 0.031%, or a third of a percent chance. Increasing this to 4 combo pieces as in the second set of runs increased the chance to about 1.7%. There’s some exploration around mulligans to be done here to see how a second combo line effects mulligan choices before I think one can definitively say whether the addition of a second combo line is really worth it.

## Next steps

The next step are tracking land counts in opening hands and landfall curves in general, the effects of tutors on average draw counts, mulligans and shaping opening hands, and how mana rocks and dorks might effect mana availability.
