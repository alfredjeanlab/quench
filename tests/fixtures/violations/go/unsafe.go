package violations

import "unsafe"

// VIOLATION: unsafe.Pointer without SAFETY comment
func UnsafeExample() uintptr {
	var x int = 42
	ptr := unsafe.Pointer(&x)
	return uintptr(ptr)
}
