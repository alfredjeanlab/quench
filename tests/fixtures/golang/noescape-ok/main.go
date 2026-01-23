package main

// NOESCAPE: Verified safe - pointer does not escape, used only within function
//go:noescape
func fastHash(data []byte) uint64

func main() {
	_ = fastHash([]byte("test"))
}
