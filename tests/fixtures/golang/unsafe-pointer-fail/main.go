package main

import "unsafe"

func main() {
	// Missing SAFETY comment - should fail
	ptr := unsafe.Pointer(uintptr(0x1234))
	_ = ptr
}
