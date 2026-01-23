package main

import (
	"fmt"

	"example.com/go-multi/pkg/storage"
)

func main() {
	store := storage.New()
	fmt.Println("CLI connected to:", store.Name())
}
