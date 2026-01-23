package storage

import "example.com/go-multi/internal/core"

// Store represents a data store.
type Store struct {
	engine *core.Engine
}

// New creates a new store.
func New() *Store {
	return &Store{engine: core.NewEngine()}
}

// Name returns the store name.
func (s *Store) Name() string {
	return s.engine.Name()
}
