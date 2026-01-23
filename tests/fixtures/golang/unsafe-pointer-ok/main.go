package main

import "unsafe"

func main() {
	// SAFETY: Converting pointer to access underlying memory layout for testing
	ptr := unsafe.Pointer(uintptr(0x1234))
	_ = ptr
}
