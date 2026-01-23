package storage

import "testing"

func TestStoreName(t *testing.T) {
	s := New()
	if s.Name() != "core-engine" {
		t.Error("expected core-engine")
	}
}
