package main

import _ "unsafe"

// LINKNAME: Accessing runtime internal for high-precision timing
//go:linkname runtimeNano runtime.nanotime
func runtimeNano() int64

func main() {
	_ = runtimeNano()
}
