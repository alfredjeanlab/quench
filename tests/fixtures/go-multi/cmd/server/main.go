package main

import (
	"example.com/go-multi/pkg/api"
	"example.com/go-multi/pkg/storage"
)

func main() {
	store := storage.New()
	api.Serve(store)
}
