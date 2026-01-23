package core

// Engine is the internal storage engine.
type Engine struct{}

// NewEngine creates a new engine.
func NewEngine() *Engine {
	return &Engine{}
}

// Name returns the engine name.
func (e *Engine) Name() string {
	return "core-engine"
}
