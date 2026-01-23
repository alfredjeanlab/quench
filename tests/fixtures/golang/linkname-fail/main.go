package main

import _ "unsafe"

// Missing LINKNAME comment - should fail
//go:linkname runtimeNano runtime.nanotime
func runtimeNano() int64

func main() {
	_ = runtimeNano()
}
