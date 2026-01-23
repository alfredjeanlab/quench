package main

// Missing NOESCAPE comment - should fail
//go:noescape
func fastHash(data []byte) uint64

func main() {
	_ = fastHash([]byte("test"))
}
