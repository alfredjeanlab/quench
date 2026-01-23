package api

import (
	"example.com/go-multi/pkg/storage"
	"testing"
)

func TestServe(t *testing.T) {
	store := storage.New()
	Serve(store) // Should not panic
}
