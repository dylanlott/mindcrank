package main

import (
	"math/rand"
	"testing"
)

func TestRunScenarioDeterministicWithSeed(t *testing.T) {
	cfg := Config{
		deckSize: 99,
		lands:    37,
		combos:   4,
		required: 2,
		runs:     5000,
		seed:     42,
	}

	got1, err := runScenario(cfg)
	if err != nil {
		t.Fatalf("first runScenario failed: %v", err)
	}
	got2, err := runScenario(cfg)
	if err != nil {
		t.Fatalf("second runScenario failed: %v", err)
	}

	if got1 != got2 {
		t.Fatalf("results differ for same seed/config: %+v vs %+v", got1, got2)
	}
}

func TestValidateConfigRejectsRequiredZero(t *testing.T) {
	cfg := Config{
		deckSize: 99,
		lands:    37,
		combos:   4,
		required: 0,
		runs:     100,
		seed:     1,
	}
	if err := validateConfig(cfg); err == nil {
		t.Fatal("expected required=0 to be rejected")
	}
}

func TestCreateDeckUsesDeckSize(t *testing.T) {
	cfg := Config{
		deckSize: 60,
		lands:    24,
		combos:   4,
		required: 2,
		runs:     1,
		seed:     1,
	}
	rng := rand.New(rand.NewSource(1))
	deck := createDeck(cfg, rng)

	if len(deck) != cfg.deckSize {
		t.Fatalf("deck length mismatch: got=%d want=%d", len(deck), cfg.deckSize)
	}

	landCount := 0
	comboCount := 0
	for _, c := range deck {
		if c.keyword == "land" {
			landCount++
		}
		if c.combo {
			comboCount++
		}
	}

	if landCount != cfg.lands {
		t.Fatalf("land count mismatch: got=%d want=%d", landCount, cfg.lands)
	}
	if comboCount != cfg.combos {
		t.Fatalf("combo count mismatch: got=%d want=%d", comboCount, cfg.combos)
	}
}
