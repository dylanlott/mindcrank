package main

// This is a Monte Carlo simulation for how fast a 2 card combo can be
// drawn into in Magic: The Gathering. It simplifies the game down to
// just lands and non-lands, with non-lands being the only cards capable
// of being combo pieces. This simulation assumes 2 combo cards in hand
// is a win-con and doesn't attempt to discern if the combo was castable.

import (
	"errors"
	"flag"
	"fmt"
	"log"
	"math/rand"
	"runtime"
	"sync"
	"time"
)

// Card holds the information for a card in the game
type Card struct {
	keyword string // denotes land or non-land
	combo   bool   // denotes a combo piece
}

// Results collates the simulations of a scenario run
type Results struct {
	attempts               int64
	averageDrawsToWin      float64
	openingHandWins        int64
	averageOpeningHandWins float64
	averageOpeningLands    float64
}

// Config holds the configuration for a simulation run.
type Config struct {
	deckSize int
	lands    int
	combos   int
	required int
	runs     int
	seed     int64
}

// Simulation holds the results of the sim's run
type Simulation struct {
	// drawsToWinCon is the number of draws to find the required
	// number of combo pieces
	drawsToWinCon int64
	// openingHandWinCon is true if the first 7 cards drawn
	// contained the required number of combo pieces
	openingHandWin bool
	// openingHandLands records the number of lands drawn in the
	// opening hand
	openingHandLands int
}

// this first scenario models a 37 land deck with 62 permanents and
// 2 combo pieces. this deck is then shuffled and run until it hits
// both combo pieces snd records the turn count that happened.
func main() {
	fmt.Println("🔮 mtg-sim booting up")
	deckSizeFlag := flag.Int("deck-size", 99, "number of cards in the deck")
	landsFlag := flag.Int("lands", 37, "number of lands in the deck")
	combosFlag := flag.Int("combos", 4, "number of combo pieces in the deck")
	requiredFlag := flag.Int("required", 2, "number of combo pieces required for a win")
	runsFlag := flag.Int("runs", 10_000_000, "number of simulations to run")
	seedFlag := flag.Int64("seed", 0, "random seed (0 uses current time)")
	flag.Parse()

	seed := *seedFlag
	if seed == 0 {
		seed = time.Now().UnixNano()
	}
	cfg := Config{
		deckSize: *deckSizeFlag,
		lands:    *landsFlag,
		combos:   *combosFlag,
		required: *requiredFlag,
		runs:     *runsFlag,
		seed:     seed,
	}

	if err := validateConfig(cfg); err != nil {
		log.Fatalf("invalid config: %v", err)
	}

	fmt.Printf("🎲 RNG seed: %d\n", cfg.seed)

	results, err := runScenario(cfg)
	if err != nil {
		log.Fatalf("error: %+v", err)
	}

	fmt.Printf("📊 results:\n%+v\n", results)
}

// runScenario runs a deck simulations a given number of times.
func runScenario(cfg Config) (Results, error) {
	var results = Results{}

	workerCount := runtime.NumCPU()
	jobs := make(chan int, workerCount)
	output := make(chan Simulation, 10_000)

	workers := &sync.WaitGroup{}
	workers.Add(workerCount)
	for i := 0; i < workerCount; i++ {
		go func() {
			defer workers.Done()
			for simIndex := range jobs {
				rng := rand.New(rand.NewSource(simSeed(cfg.seed, simIndex)))
				deck := createDeck(cfg, rng)
				output <- runSimulation(deck, cfg.required)
			}
		}()
	}

	go func() {
		for i := 0; i < cfg.runs; i++ {
			jobs <- i
		}
		close(jobs)
		workers.Wait()
		close(output)
	}()

	var drawSum int64
	var landSum int64
	var openingWinCount int64

	for sim := range output {
		results.attempts++
		if sim.openingHandWin {
			openingWinCount++
		}
		drawSum += sim.drawsToWinCon
		landSum += int64(sim.openingHandLands)
	}

	if results.attempts > 0 {
		results.averageDrawsToWin = float64(drawSum) / float64(results.attempts)
		results.averageOpeningHandWins = float64(openingWinCount) / float64(results.attempts)
		results.openingHandWins = openingWinCount
		results.averageOpeningLands = float64(landSum) / float64(results.attempts)
	}

	return results, nil
}

// createDeck creates a deck with the default setup of lands,
// non-lands, and combo pieces.
func createDeck(cfg Config, rng *rand.Rand) []Card {
	// setup the distribution of cards for our simulation
	var numLands = cfg.lands
	// set the number of non-lands to the rest of the deck
	var numNonLands = cfg.deckSize - numLands
	// assumes the commander is not a part of the combo strategy
	var numComboPieces = cfg.combos

	// create a deck
	var deck []Card

	// add lands to the deck
	for i := 0; i < numLands; i++ {
		deck = append(deck, Card{
			keyword: "land",
		})
	}

	// add non-combo permanents
	for i := 0; i < numNonLands-numComboPieces; i++ {
		deck = append(deck, Card{
			keyword: "non-land",
			combo:   false,
		})
	}

	// finally, add the appropriate number of combo pieces to the deck.
	// it is assumed that all combo pieces must be drawn to trigger
	// the win condition.
	for i := 0; i < numComboPieces; i++ {
		deck = append(deck, Card{
			keyword: "non-land",
			combo:   true,
		})
	}

	return shuffleDeck(deck, rng)
}

// shuffleDeck shuffles a slice of Cards and returns the shuffled slice
func shuffleDeck(deck []Card, rng *rand.Rand) []Card {
	rng.Shuffle(len(deck), func(i, j int) {
		deck[i], deck[j] = deck[j], deck[i]
	})
	return deck
}

// runSimulation starts drawing down until it hits a win con and
// then records the results of the simulation for later analysis
func runSimulation(deck []Card, required int) Simulation {
	var drawCount int64 = 0
	hand, deck := deck[:7], deck[7:]

	openingLands := 0
	// check lands in opening hand
	for _, c := range hand {
		if c.keyword == "land" {
			openingLands++
		}
	}

	if checkComboWin(hand, required) {
		return Simulation{
			drawsToWinCon:    drawCount,
			openingHandWin:   true,
			openingHandLands: openingLands,
		}
	}

	for len(deck) > 0 {
		drawCount++
		// draw
		drawn := deck[0]
		deck = deck[1:]
		hand = append(hand, drawn)
		// check if enough combo pieces have been hit
		if checkComboWin(hand, required) {
			return Simulation{
				drawsToWinCon:    drawCount,
				openingHandWin:   false,
				openingHandLands: openingLands,
			}
		}
	}

	return Simulation{
		drawsToWinCon:    drawCount,
		openingHandWin:   false,
		openingHandLands: openingLands,
	}
}

// checks if the required number of combo cards has been drawn
// into hand for a naive win-con check
func checkComboWin(hand []Card, required int) bool {
	var count int = 0
	for i := 0; i < len(hand); i++ {
		if hand[i].combo {
			count++
			if count == required {
				return true
			}
		}
	}
	return false
}

func validateConfig(cfg Config) error {
	if cfg.deckSize < 7 {
		return errors.New("deck size must be at least 7")
	}
	if cfg.lands < 0 {
		return errors.New("lands cannot be negative")
	}
	if cfg.combos < 0 {
		return errors.New("combos cannot be negative")
	}
	if cfg.required < 1 {
		return errors.New("required combo pieces must be at least 1")
	}
	if cfg.required > cfg.combos {
		return fmt.Errorf("required combo pieces (%d) cannot exceed total combo pieces (%d)", cfg.required, cfg.combos)
	}
	if cfg.lands+cfg.combos > cfg.deckSize {
		return fmt.Errorf("lands (%d) + combos (%d) cannot exceed deck size (%d)", cfg.lands, cfg.combos, cfg.deckSize)
	}
	if cfg.runs < 1 {
		return errors.New("runs must be at least 1")
	}
	return nil
}

func simSeed(baseSeed int64, simIndex int) int64 {
	// Mix the base seed with simulation index for deterministic, distinct RNG streams.
	x := uint64(baseSeed) + uint64(simIndex) + 0x9e3779b97f4a7c15
	x = (x ^ (x >> 30)) * 0xbf58476d1ce4e5b9
	x = (x ^ (x >> 27)) * 0x94d049bb133111eb
	x ^= x >> 31
	return int64(x)
}
