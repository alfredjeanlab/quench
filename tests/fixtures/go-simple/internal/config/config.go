package config

// Config holds application configuration.
type Config struct {
	Port int
}

// Default returns default configuration.
func Default() *Config {
	return &Config{Port: 8080}
}
