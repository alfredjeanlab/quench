package main

import (
	"fmt"

	"example.com/myapp/internal/config"
	"example.com/myapp/pkg/api"
)

func main() {
	cfg := config.Load()
	api.Start(cfg)
	fmt.Println("started")
}
