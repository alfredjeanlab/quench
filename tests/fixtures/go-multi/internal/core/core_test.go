package core

import "testing"

func TestEngineName(t *testing.T) {
	e := NewEngine()
	if e.Name() != "core-engine" {
		t.Error("expected core-engine")
	}
}
